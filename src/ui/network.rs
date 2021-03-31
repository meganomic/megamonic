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

    cache1: Vec::<String>,
    cache2: Vec::<String>,
    cache3: Vec::<String>,
    cache4: Vec::<String>,
}

impl <'a> Network <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        let y = if let Ok(networkinfo) = system.networkinfo.lock() {
            networkinfo.stats.len() as u16 * 2 + 2
        } else {
            0
        };

        Self {
            system,
            cache1: Vec::new(),
            cache2: Vec::new(),
            cache3: Vec::new(),
            cache4: Vec::new(),
            pos,
            size: XY { x: 23, y }
        }
    }

    pub fn update_cache (&mut self) {
        if let Ok(networkinfo) = self.system.networkinfo.lock() {
            self.size.y = networkinfo.stats.len() as u16 * 2 + 2;
        }
        self.cache1.clear();
        self.cache2.clear();
        self.cache3.clear();
        self.cache4.clear();
    }

    pub fn draw_static(&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        queue!(stdout,
            cursor::MoveTo(self.pos.x, self.pos.y),
            Print("\x1b[95mNetwork\x1b[0m"),
        )?;
        Ok(())
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        self.draw_static(stdout)?;

        if let Ok(networkinfo) = self.system.networkinfo.lock() {

            let freq = self.system.config.frequency.load(atomic::Ordering::Relaxed);
            let mut count = 0;

            for (key, val) in networkinfo.stats.iter() {
                if self.cache1.len() <= count / 2 {
                    self.cache1.push(format!(
                        "{}\x1b[1K{}\x1b[37m{:<8}\x1b[91m[ \x1b[92m",
                        cursor::MoveTo(self.pos.x+25, self.pos.y + 1 + count as u16),
                        cursor::MoveTo(self.pos.x, self.pos.y + 1 + count as u16),
                        key
                    ));
                }

                if self.cache2.len() <= count / 2 {
                    self.cache2.push(format!(
                        "{}\x1b[1K{}\x1b[37m{:<8}\x1b[38;5;244m[ \x1b[37m",
                        cursor::MoveTo(self.pos.x+25, self.pos.y + 1 + count as u16),
                        cursor::MoveTo(self.pos.x, self.pos.y + 1 + count as u16),
                        key
                    ));
                }

                if self.cache3.len() <= count / 2 {
                    self.cache3.push(format!(
                        "{}\x1b[91m{:>10}\x1b[92m",
                        cursor::MoveTo(self.pos.x, self.pos.y + 2 + count as u16),
                        "[ ",
                    ));
                }

                if self.cache4.len() <= count / 2 {
                    self.cache4.push(format!(
                        "{}\x1b[38;5;244m{:>10}\x1b[37m",
                        cursor::MoveTo(self.pos.x, self.pos.y + 2 + count as u16),
                        "[ ",

                    ));
                }

                unsafe {
                    if val.recv != 0 {
                        queue!(stdout,
                            Print(&self.cache1.get_unchecked(count / 2)),
                            Print(&convert_speed(val.recv, freq)),

                        )?;

                    } else {
                        queue!(stdout,
                            Print(&self.cache2.get_unchecked(count / 2)),
                            Print(&convert_speed(val.recv, freq))

                        )?;
                    }

                    if val.sent != 0 {
                        queue!(stdout,
                            Print(&self.cache3.get_unchecked(count / 2)),
                            Print(&convert_speed(val.sent, freq)),

                        )?;
                    } else {
                        queue!(stdout,
                            Print(&self.cache4.get_unchecked(count / 2)),
                            Print(&convert_speed(val.sent, freq))

                        )?;
                    }
                }
                count += 2;
            }
            self.size.y = count as u16 + 2;
        }

        Ok(())
    }
}
