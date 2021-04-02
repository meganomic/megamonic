use crossterm::{ cursor, queue, style::Print };
use std::io::Write;
use anyhow::Result;
use std::sync::atomic;

use crate::system::System as System;
use super::XY as XY;

pub struct Network <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: Vec::<(String, String, String, String)>,
}

impl <'a> Network <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        let y = if let Ok(networkinfo) = system.networkinfo.lock() {
            networkinfo.stats.len() as u16 * 2 + 2
        } else {
            2
        };

        Self {
            system,
            cache: Vec::new(),
            pos,
            size: XY { x: 23, y }
        }
    }

    pub fn rebuild_cache (&mut self) {
        if let Ok(networkinfo) = self.system.networkinfo.lock() {
            self.size.y = networkinfo.stats.len() as u16 * 2 + 2;

            self.cache.clear();

        }
    }

    pub fn draw_static(&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        queue!(stdout,
            cursor::MoveTo(self.pos.x, self.pos.y),
            Print("\x1b[95mNetwork\x1b[0m"),
        )?;
        Ok(())
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        if let Ok(networkinfo) = self.system.networkinfo.lock() {
            if self.cache.len() != networkinfo.stats.len() {
                self.cache.clear();

                let mut count: u16 = 0;
                for key in networkinfo.stats.keys() {
                    self.cache.push(
                        (
                            format!(
                                "{}\x1b[1K{}\x1b[37m{:<8}\x1b[91m[ \x1b[92m",
                                cursor::MoveTo(self.pos.x+25, self.pos.y + 1 + count ),
                                cursor::MoveTo(self.pos.x, self.pos.y + 1 + count ),
                                key
                            ),
                            format!(
                                "{}\x1b[1K{}\x1b[37m{:<8}\x1b[38;5;244m[ \x1b[37m",
                                cursor::MoveTo(self.pos.x+25, self.pos.y + 1 + count ),
                                cursor::MoveTo(self.pos.x, self.pos.y + 1 + count ),
                                key
                            ),
                            format!(
                                "{}\x1b[91m{:>10}\x1b[92m",
                                cursor::MoveTo(self.pos.x, self.pos.y + 2 + count ),
                                "[ ",
                            ),
                            format!(
                                "{}\x1b[38;5;244m{:>10}\x1b[37m",
                                cursor::MoveTo(self.pos.x, self.pos.y + 2 + count ),
                                "[ "
                            )
                        )
                    );

                    count += 2;
                }
            }

            let freq = self.system.config.frequency.load(atomic::Ordering::Relaxed);
            let mut count = 0;

            for val in networkinfo.stats.values() {
                unsafe {
                    if val.recv != 0 {
                        write!(stdout, "{}{}",
                            &self.cache.get_unchecked(count).0,
                            &convert_speed(val.recv, freq),
                        )?;
                    } else {
                        write!(stdout, "{}{}",
                            &self.cache.get_unchecked(count).1,
                            &convert_speed(val.recv, freq),
                        )?;
                    }

                    if val.sent != 0 {
                        write!(stdout, "{}{}",
                            &self.cache.get_unchecked(count).2,
                            &convert_speed(val.sent, freq),
                        )?;
                    } else {
                        write!(stdout, "{}{}",
                            &self.cache.get_unchecked(count).3,
                            &convert_speed(val.sent, freq),
                        )?;
                    }
                }
                count += 1;
            }
            self.size.y = count as u16 * 2 + 2;
        }

        Ok(())
    }
}

// Convert function for network with special handling
fn convert_speed(num: u64, freq: u64) -> String {
    if num == 0 {
        return format!("{:>5.0} b/s\x1b[38;5;244m ]\x1b[37m Rx\x1b[0m", num);
    }
    // convert it to a f64 type to we can use ln() and stuff on it.
    let num = num as f64 / (freq as f64 / 1000.0);

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

    // Different behaviour for different units 7
    match unit {
        "b" => format!("{:>5.0} {}/s\x1b[91m ]\x1b[37m Tx\x1b[0m", pretty_bytes, unit),
        "Kb" => format!("{:>4.0} {}/s\x1b[91m ]\x1b[37m Tx\x1b[0m", pretty_bytes, unit),
        _ => format!("{:>4.1} {}/s\x1b[91m ]\x1b[37m Tx\x1b[0m", pretty_bytes, unit),
    }
}
