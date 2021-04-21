use crossterm::cursor;
use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::{ bail, Result};
use std::sync::atomic;

use crate::system::System;
use super::XY;

pub struct Network <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: Vec::<(String, String, String, String)>,
    buffer_speed1: String,
    buffer_speed2: String
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
            buffer_speed1: String::new(),
            buffer_speed2: String::new(),
            pos,
            size: XY { x: 23, y }
        }
    }

    pub fn rebuild_cache (&mut self) -> Result<()> {
        if let Ok(networkinfo) = self.system.networkinfo.lock() {
            self.size.y = networkinfo.stats.len() as u16 * 2 + 2;
            let mut count: u16 = 0;
            self.cache.clear();

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

        } else {
            bail!("networkinfo lock is poisoned!");
        }

        Ok(())
    }

    pub fn draw_static(&mut self, buffer: &mut Vec::<u8>) -> Result<()> {
        write!(buffer, "{}\x1b[95mNetwork\x1b[0m",
            cursor::MoveTo(self.pos.x, self.pos.y)
        )?;
        Ok(())
    }

    pub fn draw (&mut self, buffer: &mut Vec::<u8>) -> Result<bool> {
        if let Ok(networkinfo) = self.system.networkinfo.lock() {
            // Trigger cache rebuild if lengths don't match
            if self.cache.len() != networkinfo.stats.len() {
                return Ok(true);
            }

            let freq = self.system.config.frequency.load(atomic::Ordering::Relaxed);

            for (count, val) in networkinfo.stats.values().enumerate() {
                self.buffer_speed1.clear();
                convert_speed(&mut self.buffer_speed1, val.recv, freq)?;

                let cache1 = if val.recv != 0 {
                    unsafe { self.cache.get_unchecked(count).0.as_bytes() }
                } else {
                    unsafe { self.cache.get_unchecked(count).1.as_bytes() }
                };

                self.buffer_speed2.clear();
                convert_speed(&mut self.buffer_speed2, val.sent, freq)?;

                let cache2 = if val.sent != 0 {
                    unsafe { self.cache.get_unchecked(count).2.as_bytes() }
                } else {
                    unsafe { self.cache.get_unchecked(count).3.as_bytes() }
                };

                let _ = buffer.write_vectored(&[
                    std::io::IoSlice::new(cache1),
                    std::io::IoSlice::new(self.buffer_speed1.as_bytes()),
                    std::io::IoSlice::new(b"\x1b[37m Rx\x1b[0m"),
                    std::io::IoSlice::new(cache2),
                    std::io::IoSlice::new(self.buffer_speed2.as_bytes()),
                    std::io::IoSlice::new(b"\x1b[37m Tx\x1b[0m")
                ]);
            }
        } else {
            bail!("networkinfo lock is poisoned!");
        }

        Ok(false)
    }
}

// Convert function for network with special handling
fn convert_speed(buffer: &mut String, num: u64, freq: u64) -> Result<()> {
    if num == 0 {
        write!(buffer, "{:>5.0} b/s\x1b[38;5;244m ]", num)?;
        return Ok(());
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
        "b" => write!(buffer, "{:>5.0} {}/s\x1b[91m ]", pretty_bytes, unit)?,
        "Kb" => write!(buffer, "{:>4.0} {}/s\x1b[91m ]", pretty_bytes, unit)?,
        _ => write!(buffer, "{:>4.1} {}/s\x1b[91m ]", pretty_bytes, unit)?,
    }

    Ok(())
}
