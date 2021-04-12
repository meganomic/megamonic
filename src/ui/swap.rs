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
    buffer: (String, String, String),

    total: i64,
    free: i64,
}

impl <'a> Swap <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache: (String::new(), String::new(), String::new()),
            buffer: (String::new(), String::new(), String::new()),
            total: 0,
            free: 0,
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
                convert_with_padding(&mut self.buffer.0, val.total, 4)?;
            }

            if self.free != val.free {
                self.free = val.free;
                convert_with_padding(&mut self.buffer.1, val.used, 4)?;
                convert_with_padding(&mut self.buffer.2, val.free, 4)?;

            }

            write!(stdout, "{}{}{}{}{}{}\x1b[38;5;244m ]\x1b[0m",
                &self.cache.0,
                &self.buffer.0,
                &self.cache.1,
                &self.buffer.1,
                &self.cache.2,
                &self.buffer.2
            )?;
        }

        Ok(())
    }
}
