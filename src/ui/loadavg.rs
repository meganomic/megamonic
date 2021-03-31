use crossterm::{ cursor, queue, style::Print };
use std::io::Write;
use anyhow::Result;

use crate::system::System as System;
use super::XY as XY;

pub struct Loadavg <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: Vec::<String>,
}

impl <'a> Loadavg <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        let mut cache = Vec::<String>::new();
        cache.push(String::new());
        cache.push(String::new());
        cache.push(String::new());

        Self {
            system,
            cache,
            pos,
            size: XY { x: 16, y: 4 }
        }
    }

    pub fn update_cache (&mut self) {
        unsafe {
            let load1 = self.cache.get_unchecked_mut(0);
            load1.clear();
            load1.push_str(&format!(
                "{}\x1b[95mLoad\x1b[0m{}\x1b[0K\x1b[37m1 min:  \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x, self.pos.y),
                cursor::MoveTo(self.pos.x, self.pos.y+1)
            ));

            let load2 = self.cache.get_unchecked_mut(1);
            load2.clear();
            load2.push_str(&format!(
                "\x1b[91m ]\x1b[0m{}\x1b[0K\x1b[37m5 min:  \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x, self.pos.y+2)
            ));

            let load3 = self.cache.get_unchecked_mut(2);
            load3.clear();
            load3.push_str(&format!(
                "\x1b[91m ]\x1b[0m{}\x1b[0K\x1b[37m15 min: \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x, self.pos.y+3)
            ));
        }
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        if let Ok(loadavg) = self.system.loadavg.lock() {
            let len = loadavg.min1.len().max(loadavg.min5.len().max(loadavg.min15.len()));
            self.size.x = len as u16 + 12;


                unsafe {
                    queue!(
                        stdout,
                        Print(&self.cache.get_unchecked(0)),

                        Print(&format!("{:>pad$}", &loadavg.min1, pad=len)),

                        Print(&self.cache.get_unchecked(1)),

                        Print(&format!("{:>pad$}", &loadavg.min5, pad=len)),

                        Print(&self.cache.get_unchecked(2)),

                        Print(&format!("{:>pad$}", &loadavg.min15, pad=len)),
                        Print("\x1b[91m ]\x1b[0m")
                    )?;
                }

        }
        Ok(())
    }
}
