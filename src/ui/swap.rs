use crossterm::cursor;
use std::io::Write;
use anyhow::Result;

use std::fmt::Write as fmtWrite;

use crate::system::System as System;
use super::XY as XY;
use super::convert_with_padding as convert_with_padding;

pub struct Swap <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: (String, String, String),

    total: i64,
    free: i64,
    total_str: String,
    free_str: String,
    used_str: String,
}

impl <'a> Swap <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache: (String::new(), String::new(), String::new()),
            total: 0,
            free: 0,
            total_str: String::new(),
            free_str: String::new(),
            used_str: String::new(),
            pos,
            size: XY { x: 18, y: 4 }
        }
    }

    pub fn rebuild_cache (&mut self) -> Result<()> {
        self.cache.0.clear();
        write!(self.cache.0,
            "{}\x1b[95mSwap\x1b[0m{}                  {}\x1b[37mTotal: \x1b[38;5;244m[ \x1b[37m",
            cursor::MoveTo(self.pos.x, self.pos.y),
            cursor::MoveTo(self.pos.x, self.pos.y+1),
            cursor::MoveTo(self.pos.x, self.pos.y+1)
        )?;

        self.cache.1.clear();
        write!(self.cache.1,
            "\x1b[38;5;244m ]\x1b[0m{}                  {}\x1b[37mUsed:  \x1b[91m[ \x1b[92m",
            cursor::MoveTo(self.pos.x, self.pos.y+2),
            cursor::MoveTo(self.pos.x, self.pos.y+2)
        )?;

        self.cache.2.clear();
        write!(self.cache.2,
            "\x1b[91m ]\x1b[0m{}                  {}\x1b[37mFree:  \x1b[38;5;244m[ \x1b[37m",
            cursor::MoveTo(self.pos.x, self.pos.y+3),
            cursor::MoveTo(self.pos.x, self.pos.y+3)
        )?;

        Ok(())
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        if let Ok(val) = self.system.swapinfo.lock() {
            if self.total != val.total {
                self.total = val.total;
                self.total_str = convert_with_padding(val.total, 4);
            }

            if self.free != val.free {
                self.free = val.free;
                self.free_str = convert_with_padding(val.free, 4);
                self.used_str = convert_with_padding(val.used, 4);
            }

            write!(stdout, "{}{}{}{}{}{}\x1b[38;5;244m ]\x1b[0m",
                &self.cache.0,
                &self.total_str,
                &self.cache.1,
                &self.used_str,
                &self.cache.2,
                &self.free_str
            )?;
        }

        Ok(())
    }
}
