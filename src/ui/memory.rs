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

    cache: (String, String, String)
}

impl <'a> Memory <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache: (String::new(), String::new(), String::new()),
            pos,
            size: XY { x: 18, y: 4 }
        }
    }

    pub fn rebuild_cache (&mut self) {
        self.cache.0.clear();
        self.cache.0.push_str(&format!(
            "{}\x1b[95mMemory\x1b[0m{}                    {}\x1b[37mTotal: \x1b[38;5;244m[ \x1b[37m",
            cursor::MoveTo(self.pos.x, self.pos.y),
            cursor::MoveTo(self.pos.x, self.pos.y+1),
            cursor::MoveTo(self.pos.x, self.pos.y+1)
        ));

        self.cache.1.clear();
        self.cache.1.push_str(&format!(
            "\x1b[38;5;244m ]\x1b[0m{}                    {}\x1b[37mUsed:  \x1b[91m[ \x1b[92m",
            cursor::MoveTo(self.pos.x, self.pos.y+2),
            cursor::MoveTo(self.pos.x, self.pos.y+2)
        ));

        self.cache.2.clear();
        self.cache.2.push_str(&format!(
            "\x1b[91m ]\x1b[0m{}                    {}\x1b[37mFree:  \x1b[38;5;244m[ \x1b[37m",
            cursor::MoveTo(self.pos.x, self.pos.y+3),
            cursor::MoveTo(self.pos.x, self.pos.y+3)
        ));
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        if let Ok(val) = self.system.memoryinfo.lock() {
            write!(stdout, "{}{}{}{}{}{}\x1b[38;5;244m ]\x1b[0m",
                &self.cache.0,
                &convert_with_padding(val.total, 4),
                &self.cache.1,
                &convert_with_padding(val.used, 4),
                &self.cache.2,
                &convert_with_padding(val.free, 4)
            )?;
        }

        Ok(())
    }
}
