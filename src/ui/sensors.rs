use crossterm::cursor;
use std::io::Write;
use anyhow::{ bail, Result };

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
            sensorinfo.chips.len() as u16 + 2
        } else {
            2
        };

        Self {
            system,
            cache: Vec::new(),
            pos,
            size: XY { x: 22, y }
        }
    }

    pub fn rebuild_cache (&mut self) -> Result<()> {
        if let Ok(sensorinfo) = self.system.sensorinfo.lock() {
            self.size.y = sensorinfo.chips.len() as u16 + 2;

            self.cache.clear();
            for (idx, key) in sensorinfo.chips.keys().enumerate() {
                let y = self.pos.y + 1 + idx as u16;
                self.cache.push(
                    (
                        format!(
                            "{}\x1b[1K{}\x1b[37m{}{}\x1b[91m[ \x1b[92m",
                            cursor::MoveTo(self.pos.x + 23, y),
                            cursor::MoveTo(self.pos.x, y),
                            key,
                            cursor::MoveTo(self.pos.x + 15, y),
                        ),
                        0
                    )
                );
            }
        } else {
            bail!("sensorinfo lock is poisoned!");
        }

        Ok(())
    }

    pub fn draw_static(&mut self, buffer: &mut Vec::<u8>) -> Result<()> {
        write!(
            buffer, "{}\x1b[95mSensors\x1b[0m",
            cursor::MoveTo(self.pos.x, self.pos.y)
        )?;

        Ok(())
    }

    pub fn draw (&mut self, buffer: &mut Vec::<u8>) -> Result<bool> {
        if let Ok(sensorinfo) = self.system.sensorinfo.lock() {
            // Trigger cache rebuild if lengths aren't equal
            if self.cache.len() != sensorinfo.chips.len() {
                return Ok(true);
            }

            for (idx, val) in sensorinfo.chips.values().enumerate() {
                unsafe {
                    let cache = self.cache.get_unchecked_mut(idx);

                    // Don't update the value if it hasn't changed
                    if cache.1 != *val {
                        write!(buffer, "{}{} C\x1b[91m ]\x1b[0m", &cache.0, val)?;
                        cache.1 = *val;
                    }
                }
            }
        } else {
            bail!("sensorinfo lock is poisoned!");
        }

        Ok(false)
    }
}
