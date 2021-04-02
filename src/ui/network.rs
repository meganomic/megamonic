use crossterm::{ cursor, queue, style::Print };
use std::io::Write;
use anyhow::Result;
use std::sync::atomic;

use crate::system::System as System;
use super::XY as XY;
use super::convert_speed as convert_speed;

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
                            //&self.cache1.get_unchecked(count),
                            &self.cache.get_unchecked(count).0,
                            &convert_speed(val.recv, freq),
                        )?;
                    } else {
                        write!(stdout, "{}{}",
                            //&self.cache2.get_unchecked(count),
                            &self.cache.get_unchecked(count).1,
                            &convert_speed(val.recv, freq),
                        )?;
                    }

                    if val.sent != 0 {
                        write!(stdout, "{}{}",
                            //&self.cache3.get_unchecked(count),
                            &self.cache.get_unchecked(count).2,
                            &convert_speed(val.recv, freq),
                        )?;
                    } else {
                        write!(stdout, "{}{}",
                            //&self.cache4.get_unchecked(count),
                            &self.cache.get_unchecked(count).3,
                            &convert_speed(val.recv, freq),
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
