use crossterm::{ cursor, queue, style::Print };
use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::Result;
use std::sync::atomic;

use crate::system::System as System;
use super::XY as XY;

pub struct Processes <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    pidlen: usize,
    cache1: Vec::<String>,
    cache2: std::collections::HashMap<u32, String>,
    memory: String,
}

impl <'a> Processes <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache1: Vec::<String>::new(),
            cache2: std::collections::HashMap::new(),
            memory: String::new(),
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

    pub fn draw_static(&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        queue!(
            stdout,
            cursor::MoveTo(self.pos.x, self.pos.y),
            Print("\x1b[95mProcesses\x1b[0m")
        )?;

        Ok(())
    }

    pub fn draw(&mut self, stdout: &mut std::io::Stdout, terminal_size: &XY) -> Result<()> {
        let smaps = self.system.config.smaps.load(atomic::Ordering::Relaxed);

        if let Ok(processinfo) = self.system.processinfo.lock() {
            let (pidlen, list) = processinfo.cpu_sort();

            // Update cache if the length of PID increases
            if pidlen > self.pidlen {
                self.cache2.clear();
            }

            //let now = std::time::Instant::now();
            for (idx, val) in list.iter().enumerate() {
                // Break once we printed all the processes that fit on screen
                if idx == self.size.y as usize {
                    break;
                }

                // Check if there actually is a PSS value
                // If there isn't it probably requires root access, use RSS instead
                if smaps && val.pss != -1{
                    convert_with_padding_proc(&mut self.memory, val.pss, 4, true)?;
                } else {
                    convert_with_padding_proc(&mut self.memory, val.rss, 4, false)?;
                }

                unsafe {
                    // This is needed because of rounding errors. There's probably a better way
                    let max_length = (terminal_size.x - self.pos.x - 19) as usize;
                    if val.cpu_avg > 0.0 && val.cpu_avg < 99.5 {
                        write!(stdout,
                            "{}\x1b[91m[ \x1b[92m{:>4.1}%\x1b[91m ] \x1b[0m\x1b[91m[ {}{}",
                            &self.cache1.get_unchecked(idx),
                            val.cpu_avg,
                            &self.memory,
                            &self.cache2.entry(val.pid).or_insert_with(||
                                maxstr(
                                    &val.executable,
                                    &val.cmdline,
                                    val.not_executable,
                                    val.pid,
                                    pidlen,
                                    max_length
                                )
                            )
                        )?;
                    } else if val.cpu_avg >= 99.5 {
                        write!(stdout,
                            "{}\x1b[91m[ \x1b[92m{:>4.0}%\x1b[91m ] \x1b[0m\x1b[91m[ {}{}",
                            &self.cache1.get_unchecked(idx),
                            val.cpu_avg,
                            &self.memory,
                            &self.cache2.entry(val.pid).or_insert_with(||
                                maxstr(
                                    &val.executable,
                                    &val.cmdline,
                                    val.not_executable,
                                    val.pid,
                                    pidlen,
                                    max_length
                                )
                            )
                        )?;
                    } else {
                        write!(stdout,
                            "{}\x1b[38;5;244m[ \x1b[37m{:>4.1}%\x1b[38;5;244m ] \x1b[0m\x1b[91m[ {}{}",
                            &self.cache1.get_unchecked(idx),
                            val.cpu_avg,
                            &self.memory,
                            &self.cache2.entry(val.pid).or_insert_with(||
                                maxstr(
                                    &val.executable,
                                    &val.cmdline,
                                    val.not_executable,
                                    val.pid,
                                    pidlen,
                                    max_length
                                )
                            )
                        )?;
                    }
                }
            }
            //eprintln!("{}", now.elapsed().as_nanos());

            // Save the length of the longest PID in the cache so we can check if it changes
            // In which case we need to rebuild the cache
            self.pidlen = pidlen;
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
fn convert_with_padding_proc(buffer: &mut String, num: i64, padding: usize, blue: bool) -> Result<()> {
    buffer.clear();

    let color = if blue {
        "\x1b[94m"
    } else {
        "\x1b[92m"
    };

    if num == 0 {
        write!(buffer, "{}  {:>pad$}", color, "-", pad=padding+1)?;
        return Ok(());
    }
    // convert it to a f64 type to we can use ln() and stuff on it.
    let num = num as f64;

    let units = ["b", "Kb", "Mb", "Gb", "Tb", "Pb", "Eb", "Zb", "Yb"];

    // A kilobyte is 1024 bytes. Fight me!
    let delimiter = 1024_f64;

    // Magic that makes no sense to me
    let exponent = std::cmp::min(
        (num.ln() / delimiter.ln()).floor() as i32,
        (units.len() - 1) as i32,
    );
    let pretty_bytes = num / delimiter.powi(exponent as i32);
    let unit = units[exponent as usize];

    // Different behaviour for different units
    match unit {
        "b" => write!(buffer, "{}{:>pad$.0} {}", color, pretty_bytes, unit, pad=padding+1)?,
        "Kb" | "Mb" => write!(buffer, "{}{:>pad$.0} {}", color, pretty_bytes, unit, pad=padding)?,
        "Gb" => {
            if pretty_bytes >= 10.0 { write!(buffer, "{}{:>pad$.1} {}", color, pretty_bytes, unit, pad=padding)?; }
            else { write!(buffer, "{}{:>pad$.2} {}", color, pretty_bytes, unit, pad=padding)?; }
        },
        _ => write!(buffer, "{}{:>pad$.1} {}", color, pretty_bytes, unit, pad=padding)?,
    }

    Ok(())
}
