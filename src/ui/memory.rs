use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::{ bail, Result };
use std::sync::atomic::Ordering;

use crate::system::System;
use super::{ XY, convert_with_padding };

pub struct Memory <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: (String, String, String),
    buffer: (String, String, String),

    total: u64,
    free: u64
}

impl <'a> Memory <'a> {
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

    pub fn rebuild_cache (&mut self) {
        self.cache.0.clear();
        let _ = write!(self.cache.0,
            "\x1b[{};{}H\x1b[95mMemory\x1b[0m\x1b[{};{}H                    \x1b[{};{}H\x1b[37mTotal: \x1b[38;5;244m[ \x1b[37m",
            self.pos.y, self.pos.x,
            self.pos.y+1, self.pos.x,
            self.pos.y+1, self.pos.x
        );

        self.cache.1.clear();
        let _ = write!(self.cache.1,
            "\x1b[38;5;244m ]\x1b[0m\x1b[{};{}H                    \x1b[{};{}H\x1b[37mUsed:  \x1b[91m[ \x1b[92m",
            self.pos.y+2, self.pos.x,
            self.pos.y+2, self.pos.x
        );

        self.cache.2.clear();
        let _ = write!(self.cache.2,
            "\x1b[91m ]\x1b[0m\x1b[{};{}H                    \x1b[{};{}H\x1b[37mFree:  \x1b[38;5;244m[ \x1b[37m",
            self.pos.y+3, self.pos.x,
            self.pos.y+3, self.pos.x,
        );
    }

    pub fn draw (&mut self, buffer: &mut Vec::<u8>) -> Result<()> {
        if let Ok(val) = self.system.memoryinfo.lock() {
            if self.total != val.mem_total.load(Ordering::Relaxed) {
                self.total = val.mem_total.load(Ordering::Relaxed);
                convert_with_padding(&mut self.buffer.0, val.mem_total.load(Ordering::Relaxed));
            }

            if self.free != val.mem_free {
                self.free = val.mem_free;
                convert_with_padding(&mut self.buffer.1, val.mem_used);
                convert_with_padding(&mut self.buffer.2, val.mem_free);
            }
        } else {
            bail!("memoryinfo lock is poisoned!");
        }

        let _ = buffer.write_vectored(&[
            std::io::IoSlice::new(self.cache.0.as_bytes()),
            std::io::IoSlice::new(self.buffer.0.as_bytes()),
            std::io::IoSlice::new(self.cache.1.as_bytes()),
            std::io::IoSlice::new(self.buffer.1.as_bytes()),
            std::io::IoSlice::new(self.cache.2.as_bytes()),
            std::io::IoSlice::new(self.buffer.2.as_bytes()),
            std::io::IoSlice::new(b"\x1b[38;5;244m ]\x1b[0m")
        ]);

        Ok(())
    }
}
