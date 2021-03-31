use crossterm::{ cursor, queue, style::Print };
use std::io::Write;
use anyhow::Result;

use crate::system::System as System;
use super::XY as XY;

pub struct Sensors <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: Vec::<(String, u8)>,
}

impl <'a> Sensors <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        let y = if let Ok(sensorinfo) = system.sensorinfo.lock() {
            sensorinfo.chips.len() as u16 + 1
        } else {
            0
        };

        Self {
            system,
            cache: Vec::new(),
            pos,
            size: XY { x: 22, y }
        }
    }

    pub fn update_cache (&mut self) {
        if let Ok(sensorinfo) = self.system.sensorinfo.lock() {
            self.size.y = sensorinfo.chips.len() as u16 + 1;
        }
        self.cache.clear();

    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        queue!(
            stdout,
            cursor::MoveTo(self.pos.x, self.pos.y),
            Print("\x1b[95mSensors\x1b[0m")
        )?;

        if let Ok(sensorinfo) = self.system.sensorinfo.lock() {
            for (idx, (key, val)) in sensorinfo.chips.iter().enumerate() {
                if (self.pos.y + 1 + idx as u16) < (self.pos.y + 8) {
                    if self.cache.len() <= idx {
                        self.cache.push((format!(
                            "{}\x1b[1K{}\x1b[37m{}{}\x1b[91m[ \x1b[92m",
                            cursor::MoveTo(self.pos.x + 23, self.pos.y + 1 + idx as u16),
                            cursor::MoveTo(self.pos.x, self.pos.y + 1 + idx as u16),
                            key,
                            cursor::MoveTo(self.pos.x + 15, self.pos.y + 1 + idx as u16),
                        ), 0));
                    }

                    unsafe {
                        let cache = self.cache.get_unchecked_mut(idx);
                        if cache.1 != *val {
                            queue!(
                                stdout,
                                Print(&cache.0),
                                Print(&format!("{}", val)),
                                Print(" C\x1b[91m ]\x1b[0m"),
                            )?;
                            cache.1 = *val;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
