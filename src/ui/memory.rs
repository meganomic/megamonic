use crossterm::cursor;
use std::io::Write;
use anyhow::Result;

use crate::system::System as System;
use super::XY as XY;
use super::convert_with_padding as convert_with_padding;

pub struct Memory <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: Vec::<String>,
}

impl <'a> Memory <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        let mut cache = Vec::<String>::new();
        cache.push(String::new());
        cache.push(String::new());
        cache.push(String::new());

        Self {
            system,
            cache,
            pos,
            size: XY { x: 18, y: 4 }
        }
    }

    pub fn rebuild_cache (&mut self) {
        unsafe {
            let cache1 = self.cache.get_unchecked_mut(0);
            cache1.clear();
            cache1.push_str(&format!(
                "{}\x1b[95mMemory\x1b[0m{}                    {}\x1b[37mTotal: \x1b[38;5;244m[ \x1b[37m",
                cursor::MoveTo(self.pos.x, self.pos.y),
                cursor::MoveTo(self.pos.x, self.pos.y+1),
                cursor::MoveTo(self.pos.x, self.pos.y+1)
            ));

            let cache2 = self.cache.get_unchecked_mut(1);
            cache2.clear();
            cache2.push_str(&format!(
                "\x1b[38;5;244m ]\x1b[0m{}                    {}\x1b[37mUsed:  \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x, self.pos.y+2),
                cursor::MoveTo(self.pos.x, self.pos.y+2)
            ));

            let cache3 = self.cache.get_unchecked_mut(2);
            cache3.clear();
            cache3.push_str(&format!(
                "\x1b[91m ]\x1b[0m{}                    {}\x1b[37mFree:  \x1b[38;5;244m[ \x1b[37m",
                cursor::MoveTo(self.pos.x, self.pos.y+3),
                cursor::MoveTo(self.pos.x, self.pos.y+3)
            ));
        }
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        if let Ok(val) = self.system.memoryinfo.lock() {
            unsafe {
                write!(stdout, "{}{}{}{}{}{}\x1b[38;5;244m ]\x1b[0m",
                    &self.cache.get_unchecked(0),
                    &convert_with_padding(val.total, 4),
                    &self.cache.get_unchecked(1),
                    &convert_with_padding(val.used, 4),
                    &self.cache.get_unchecked(2),
                    &convert_with_padding(val.free, 4)
                )?;
            }
        }

        Ok(())
    }
}
