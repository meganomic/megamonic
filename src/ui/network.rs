use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::{ bail, Result};
use std::sync::atomic;

use crate::system::System;
use super::{ DELIMITER_LN, DELIMITER, UNITS, XY };

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

            for (idx, key) in networkinfo.stats.keys().enumerate() {
                let y = self.pos.y + count + 1;
                if idx == 0 {
                    self.cache.push(
                        (
                            format!(
                                "\x1b[{};{}H\x1b[95mNetwork\x1b[0m\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[37m{:<8}\x1b[91m[ \x1b[92m",
                                self.pos.y, self.pos.x,
                                y, self.pos.x + 25,
                                y, self.pos.x,
                                key
                            ),
                            format!(
                                "\x1b[{};{}H\x1b[95mNetwork\x1b[0m\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[37m{:<8}\x1b[38;5;244m[ \x1b[37m",
                                self.pos.y, self.pos.x,
                                y, self.pos.x + 25,
                                y, self.pos.x,
                                key
                            ),
                            format!(
                                "\x1b[{};{}H\x1b[91m{:>10}\x1b[92m",
                                y + 1, self.pos.x,
                                "[ ",
                            ),
                            format!(
                                "\x1b[{};{}H\x1b[38;5;244m{:>10}\x1b[37m",
                                y + 1, self.pos.x,
                                "[ "
                            )
                        )
                    );
                } else {
                    self.cache.push(
                        (
                            format!(
                                "\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[37m{:<8}\x1b[91m[ \x1b[92m",
                                y, self.pos.x + 25,
                                y, self.pos.x,
                                /*cursor::MoveTo(self.pos.x+25, self.pos.y + 1 + count ),
                                cursor::MoveTo(self.pos.x, self.pos.y + 1 + count ),*/
                                key
                            ),
                            format!(
                                "\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[37m{:<8}\x1b[38;5;244m[ \x1b[37m",
                                y, self.pos.x + 25,
                                y, self.pos.x,
                                /*cursor::MoveTo(self.pos.x+25, self.pos.y + 1 + count ),
                                cursor::MoveTo(self.pos.x, self.pos.y + 1 + count ),*/
                                key
                            ),
                            format!(
                                "\x1b[{};{}H\x1b[91m{:>10}\x1b[92m",
                                y + 1, self.pos.x,
                                //cursor::MoveTo(self.pos.x, self.pos.y + 2 + count ),
                                "[ ",
                            ),
                            format!(
                                "\x1b[{};{}H\x1b[38;5;244m{:>10}\x1b[37m",
                                y + 1, self.pos.x,
                                //cursor::MoveTo(self.pos.x, self.pos.y + 2 + count ),
                                "[ "
                            )
                        )
                    );
                }

                count += 2;
            }

        } else {
            bail!("networkinfo lock is poisoned!");
        }

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
                convert_speed(&mut self.buffer_speed1, val.recv, freq);

                let cache1 = if val.recv != 0 {
                    unsafe { self.cache.get_unchecked(count).0.as_bytes() }
                } else {
                    unsafe { self.cache.get_unchecked(count).1.as_bytes() }
                };

                convert_speed(&mut self.buffer_speed2, val.sent, freq);

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

// Taken from https://github.com/banyan/rust-pretty-bytes/blob/master/src/converter.rs
// And customized for my use
// Convert function for network with special handling
fn convert_speed(buffer: &mut String, num: u64, freq: u64) {
    buffer.clear();

    if num != 0 {
        // convert it to a f64 type to we can use ln() and stuff on it.
        let num = num as f64 / (freq as f64 / 1000.0);

        // Magic that makes no sense to me
        let exponent = (num.ln() / DELIMITER_LN).floor() as i32;
        let pretty_bytes = num / DELIMITER.powi(exponent);

        // Different behaviour for different units 7
        match exponent {
            0 => { let _ = write!(buffer, "{:>5.0} b/s\x1b[91m ]", pretty_bytes); },
            1 => { let _ = write!(buffer, "{:>4.0} Kb/s\x1b[91m ]", pretty_bytes); },
            2 => { let _ = write!(buffer, "{:>4.0} Mb/s\x1b[91m ]", pretty_bytes); },
            _ => {
                let unit = UNITS[exponent as usize];
                let _ = write!(buffer, "{:>4.1} {}/s\x1b[91m ]", pretty_bytes, unit);
            },
        }
    } else {
        let _ = write!(buffer, "{:>5.0} b/s\x1b[38;5;244m ]", num);
    }
}
