use crossterm::cursor;
use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::{ bail, Result };
use std::sync::atomic;
use ahash::AHashMap;

use crate::system::System as System;
use super::XY as XY;

pub struct Processes <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    pidlen: usize,
    cache1: Vec::<String>,
    cache2: AHashMap<u32, String>,
    cpu_buffer: String,
    memory_buffer: String,
}

impl <'a> Processes <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache1: Vec::<String>::new(),
            cache2: AHashMap::new(),
            cpu_buffer: String::new(),
            memory_buffer: String::new(),
            pos,
            size: XY { x: 0, y: 4 },
            pidlen: 0,
        }
    }

    pub fn rebuild_cache(&mut self, terminal_size: &XY) {
        self.cache1.clear();
        self.cache2.clear();
        self.size.y = terminal_size.y.saturating_sub(self.pos.y).saturating_sub(3);

        for idx in 0..self.size.y {
            self.cache1.push(format!(
                "{}\x1b[0K{}",
                cursor::MoveTo(self.pos.x, self.pos.y + 1 + idx as u16),
                cursor::MoveTo(self.pos.x, self.pos.y + 1 + idx as u16),
            ));
        }
    }

    pub fn draw_static(&mut self, buffer: &mut Vec::<u8>) -> Result<()> {
        write!(
            buffer, "{}\x1b[95mProcesses\x1b[0m",
            cursor::MoveTo(self.pos.x, self.pos.y),
        )?;

        Ok(())
    }

    pub fn draw(&mut self, buffer: &mut Vec::<u8>, terminal_size: &XY) -> Result<()> {
        let smaps = self.system.config.smaps.load(atomic::Ordering::Relaxed);

        if let Ok(processinfo) = self.system.processinfo.lock() {
            let (pidlen, list) = processinfo.cpu_sort();

            // Update cache if the length of PID increases
            if pidlen > self.pidlen {
                self.pidlen = pidlen;
                self.cache2.clear();
            }

            let max_length = (terminal_size.x - self.pos.x - 19) as usize;

            //let now = std::time::Instant::now();
            for (idx, val) in list.iter().enumerate() {
                // Break once we printed all the processes that fit on screen
                if idx == self.size.y as usize {
                    break;
                }

                // Check if there actually is a PSS value
                // If there isn't it probably requires root access, use RSS instead
                if smaps & (val.pss != -1) {
                    convert_with_padding_proc(&mut self.memory_buffer, val.pss, "\x1b[94m");
                } else {
                    convert_with_padding_proc(&mut self.memory_buffer, val.rss, "\x1b[92m");
                }

                // This is needed because of rounding errors. There's probably a better way
                self.cpu_buffer.clear();
                if val.cpu_avg > 0.0 && val.cpu_avg < 99.5 {
                    let _ = write!(self.cpu_buffer, "\x1b[91m[ \x1b[92m{:>4.1}%\x1b[91m ] \x1b[0m\x1b[91m[ ", val.cpu_avg);
                } else if val.cpu_avg >= 99.5 {
                    let _ = write!(self.cpu_buffer, "\x1b[91m[ \x1b[92m{:>4.0}%\x1b[91m ] \x1b[0m\x1b[91m[ ", val.cpu_avg);
                } else {
                    let _ = write!(self.cpu_buffer, "\x1b[38;5;244m[ \x1b[37m{:>4.1}%\x1b[38;5;244m ] \x1b[0m\x1b[91m[ ", val.cpu_avg);
                }

                let ioslice = &[
                    unsafe { std::io::IoSlice::new(self.cache1.get_unchecked(idx).as_bytes()) },
                    std::io::IoSlice::new(self.cpu_buffer.as_bytes()),
                    std::io::IoSlice::new(self.memory_buffer.as_bytes()),
                    std::io::IoSlice::new(self.cache2.entry(val.pid).or_insert_with(||
                        maxstr(
                            &val.executable,
                            &val.cmdline,
                            val.not_executable,
                            val.pid,
                            pidlen,
                            max_length
                        )
                    ).as_bytes())
                ];

                let _ = buffer.write_vectored(ioslice);
            }

            //eprintln!("{}", now.elapsed().as_nanos());

            // Save the length of the longest PID in the cache so we can check if it changes
            // In which case we need to rebuild the cache
        } else {
            bail!("processinfo lock is poisoned!");
        }

        Ok(())
    }
}

fn maxstr(exec: &str, cmd: &str, is_not_exec: bool, pid: u32, pidlen: usize, maxlen: usize) -> String {
    let mut e = exec.to_string();
    let mut c = cmd.to_string();

    let color = if is_not_exec {
        "\x1b[94m"
    } else {
        "\x1b[92m"
    };

    let mut p = format!("{:>pad$} ", pid, pad=pidlen);

    if (p.len() + 3) > maxlen {
        p.truncate(maxlen.saturating_sub(3));
        return format!("\x1b[91m ] \x1b[37m{}\x1b[0m", p);
    }

    if (e.len() + p.len() + 3) > maxlen {
        e.truncate(maxlen.saturating_sub(p.len() + 3));
        return format!("\x1b[91m ] \x1b[37m{}\x1b[0m{}{}\x1b[0m", p, color, e);

    }

    if (c.len() + e.len() + p.len() + 3) > maxlen {
        c.truncate(maxlen.saturating_sub(e.len() + p.len() + 3));
        return format!("\x1b[91m ] \x1b[37m{}\x1b[0m{}{}\x1b[38;5;244m{}\x1b[0m", p, color, e, c);
    }

    format!("\x1b[91m ] \x1b[37m{}\x1b[0m{}{}\x1b[38;5;244m{}\x1b[0m", p, color, e, c)
}

// Special handling for 0 memory for processe list
fn convert_with_padding_proc(buffer: &mut String, num: i64, color: &str) {
    buffer.clear();

    if num != 0 {
        // convert it to a f64 type to we can use ln() and stuff on it.
        let num = num as f64;

        static UNITS: [&str; 9] = ["b", "Kb", "Mb", "Gb", "Tb", "Pb", "Eb", "Zb", "Yb"];

        // A kilobyte is 1024 bytes. Fight me!
        let delimiter = 1024_f64;

        // Magic that makes no sense to me
        let exponent = (num.ln() / delimiter.ln()).floor() as i32;

        let pretty_bytes = num / delimiter.powi(exponent);
        //let unit = UNITS[exponent as usize];

        // Different behaviour for different units
        // They are in order of most commonly used
        match exponent {
            2 => { let _ = write!(buffer, "{}{:>4.0} Mb", color, pretty_bytes); },
            3 => {
                if pretty_bytes >= 10.0 { let _ = write!(buffer, "{}{:>4.1} Gb", color, pretty_bytes); }
                else { let _ = write!(buffer, "{}{:>4.2} Gb", color, pretty_bytes); }
            },
            1 => { let _ = write!(buffer, "{}{:>4.0} Kb", color, pretty_bytes); },
            0 => { let _ = write!(buffer, "{}{:>5.0} b", color, pretty_bytes); },
            _ => {
                let _ = write!(buffer, "{}{:>4.1} {}", color, pretty_bytes, UNITS[exponent as usize]);
            },
        };
    } else {
        let _ = write!(buffer, "{}  {:>5}", color, "-");
    }
}
