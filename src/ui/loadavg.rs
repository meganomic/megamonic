use crossterm::cursor;
use std::io::Write;
use anyhow::Result;

use crate::system::System as System;
use super::XY as XY;

pub struct Loadavg <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: (String, String, String),
}

impl <'a> Loadavg <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache: (String::new(), String::new(), String::new()),
            pos,
            size: XY { x: 16, y: 4 }
        }
    }

    pub fn rebuild_cache (&mut self) {
        self.cache.0.clear();
        self.cache.0.push_str(&format!(
            "{}\x1b[95mLoad\x1b[0m{}\x1b[0K\x1b[37m1 min:  \x1b[91m[ \x1b[92m",
            cursor::MoveTo(self.pos.x, self.pos.y),
            cursor::MoveTo(self.pos.x, self.pos.y+1)
        ));

        self.cache.1.clear();
        self.cache.1.push_str(&format!(
            "\x1b[91m ]\x1b[0m{}\x1b[0K\x1b[37m5 min:  \x1b[91m[ \x1b[92m",
            cursor::MoveTo(self.pos.x, self.pos.y+2)
        ));

        self.cache.2.clear();
        self.cache.2.push_str(&format!(
            "\x1b[91m ]\x1b[0m{}\x1b[0K\x1b[37m15 min: \x1b[91m[ \x1b[92m",
            cursor::MoveTo(self.pos.x, self.pos.y+3)
        ));
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        if let Ok(loadavg) = self.system.loadavg.lock() {
            let len = loadavg.min1.len().max(loadavg.min5.len().max(loadavg.min15.len()));
            self.size.x = len as u16 + 12;

            write!(stdout, "{}{:>pad$}{}{:>pad$}{}{:>pad$}\x1b[91m ]\x1b[0m",
                &self.cache.0,
                &loadavg.min1,
                &self.cache.1,
                &loadavg.min5,
                &self.cache.2,
                &loadavg.min15,
                pad=len
            )?;
        }

        Ok(())
    }
}
