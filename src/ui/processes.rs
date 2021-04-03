use crossterm::{ cursor, queue, style::Print };
use std::io::Write;
use anyhow::Result;
use std::sync::atomic;

use crate::system::System as System;
use super::XY as XY;
//use super::convert_with_padding_proc as convert_with_padding_proc;

pub struct Processes <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    pidlen: usize,
    cache1: Vec::<String>,
    cache2: std::collections::HashMap<u32, String>,
}

impl <'a> Processes <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache1: Vec::<String>::new(),
            cache2: std::collections::HashMap::new(),
            pos,
            size: XY { x: 0, y: 4 },
            pidlen: 0,
        }
    }

    pub fn rebuild_cache(&mut self, terminal_size: &XY) {
        self.cache1.clear();
        self.cache2.clear();
        let items = terminal_size.y.saturating_sub(self.pos.y).saturating_sub(3);

        for idx in 0..items {
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
        //let now = std::time::Instant::now();
        let items = terminal_size.y - self.pos.y - 4;

        if let Ok(processinfo) = self.system.processinfo.read() {
            let (pidlen, vector) = processinfo.cpu_sort();

            // Update cache if the length of PID increases
            if pidlen > self.pidlen {
                self.cache2.clear();
            }

            for (idx, (_, val)) in vector.iter().enumerate() {
                //let now = std::time::Instant::now();
                let memory = if self.system.config.smaps.load(atomic::Ordering::Relaxed) {
                    // Check if there actually is a PSS value
                    // If there isn't it probably requires root access, use RSS instead
                    if val.pss != -1 {
                        format!("\x1b[94m{}\x1b[0m", &convert_with_padding_proc(val.pss, 4))
                    } else {
                        convert_with_padding_proc(val.rss, 4)
                    }
                } else {
                    convert_with_padding_proc(val.rss, 4)
                };
                //eprintln!("{}", now.elapsed().as_nanos());


                if !self.cache2.contains_key(&val.pid) {
                    let shoe = maxstr(&val.executable, &val.cmdline, val.not_executable, val.pid, pidlen, (terminal_size.x - self.pos.x - 19) as usize);
                    self.cache2.insert(val.pid, shoe);
                }

                unsafe {
                    if val.cpu_avg > 0.0 && val.cpu_avg < 99.5 {
                        write!(stdout,
                            "{}\x1b[91m[ \x1b[92m{:>4.1}%\x1b[91m ] \x1b[0m\x1b[91m[ \x1b[92m{}{}",
                            &self.cache1.get_unchecked(idx),
                            val.cpu_avg,
                            &memory,
                            &self.cache2.get(&val.pid)
                                .expect("Process cache is corrupted!")
                        )?;
                    } else if val.cpu_avg >= 99.5 {
                        write!(stdout,
                            "{}\x1b[91m[ \x1b[92m{:>4.0}%\x1b[91m ] \x1b[0m\x1b[91m[ \x1b[92m{}{}",
                            &self.cache1.get_unchecked(idx),
                            val.cpu_avg,
                            &memory,
                            &self.cache2.get(&val.pid)
                                .expect("Process cache is corrupted!")
                        )?;
                    } else {
                        write!(stdout,
                            "{}\x1b[38;5;244m[ \x1b[37m{:>4.1}%\x1b[38;5;244m ] \x1b[0m\x1b[91m[ \x1b[92m{}{}",
                            &self.cache1.get_unchecked(idx),
                            val.cpu_avg,
                            &memory,
                            &self.cache2.get(&val.pid)
                                .expect("Process cache is corrupted!")
                        )?;
                    }
                }

                if idx == items as usize {
                    break;
                }
            }

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
fn convert_with_padding_proc(num: i64, padding: usize) -> String {
    if num == -1 {
        return format!("Error");
    }
    if num == 0 {
        return format!("  {:>pad$}", "-", pad=padding+1);
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
        "b" => format!("{:>pad$.0} {}", pretty_bytes, unit, pad=padding+1),
        "Kb" | "Mb" => format!("{:>pad$.0} {}", pretty_bytes, unit, pad=padding),
        "Gb" => {
            if pretty_bytes >= 10.0 { format!("{:>pad$.1} {}", pretty_bytes, unit, pad=padding) }
            else { format!("{:>pad$.2} {}", pretty_bytes, unit, pad=padding) }
        },
        _ => format!("{:>pad$.1} {}", pretty_bytes, unit, pad=padding),
    }
}
