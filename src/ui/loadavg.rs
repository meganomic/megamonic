use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::{ bail, Result };

use crate::system::System;
use super::XY;

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
        let _ = write!(self.cache.0,
            "\x1b[{};{}H\x1b[95mLoad\x1b[0m\x1b[{};{}H\x1b[0K\x1b[37m1 min:  \x1b[91m[ \x1b[92m",
            self.pos.y, self.pos.x,
            self.pos.y+1, self.pos.x,
        );

        self.cache.1.clear();
        let _ = write!(self.cache.1,
            "\x1b[91m ]\x1b[0m\x1b[{};{}H\x1b[0K\x1b[37m5 min:  \x1b[91m[ \x1b[92m",
            self.pos.y+2, self.pos.x,
        );

        self.cache.2.clear();
        let _ = write!(self.cache.2,
            "\x1b[91m ]\x1b[0m\x1b[{};{}H\x1b[0K\x1b[37m15 min: \x1b[91m[ \x1b[92m",
            self.pos.y+3, self.pos.x,
        );
    }

    pub fn draw (&mut self, buffer: &mut Vec::<u8>) -> Result<()> {
        if let Ok(loadavg) = self.system.loadavg.lock() {
            let len = loadavg.min1.len().max(loadavg.min5.len().max(loadavg.min15.len()));
            self.size.x = len as u16 + 12;

            // If they are all the same length write it efficently
            if loadavg.min1.len() == loadavg.min5.len() && loadavg.min1.len() == loadavg.min15.len() {
                let _ = buffer.write_vectored(&[
                    std::io::IoSlice::new(self.cache.0.as_bytes()),
                    std::io::IoSlice::new(loadavg.min1.as_bytes()),
                    std::io::IoSlice::new(self.cache.1.as_bytes()),
                    std::io::IoSlice::new(loadavg.min5.as_bytes()),
                    std::io::IoSlice::new(self.cache.2.as_bytes()),
                    std::io::IoSlice::new(loadavg.min15.as_bytes()),
                    std::io::IoSlice::new(b"\x1b[91m ]\x1b[0m")
                ]);
            } else {
                write!(buffer, "{}{:>pad$}{}{:>pad$}{}{:>pad$}\x1b[91m ]\x1b[0m",
                    &self.cache.0,
                    &loadavg.min1,
                    &self.cache.1,
                    &loadavg.min5,
                    &self.cache.2,
                    &loadavg.min15,
                    pad=len
                )?;
            }
        } else {
            bail!("loadavg lock is poisoned!");
        }

        Ok(())
    }
}
