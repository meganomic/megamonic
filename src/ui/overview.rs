use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::{ bail, Result };
use std::sync::atomic::Ordering;

use crate::system::System;
use super::XY;

pub struct Overview <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: (String, String, String)
}

impl <'a> Overview <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache: (String::new(), String::new(), String::new()),
            pos,
            size: XY { x: 15, y: 4 }
        }
    }

    pub fn rebuild_cache (&mut self) {
        self.cache.0.clear();
        let _ = write!(self.cache.0,
            "\x1b[{};{}H\x1b[95mOverview\x1b[0m\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[37mCPU:  \x1b[91m[ \x1b[92m",
            self.pos.y, self.pos.x,
            self.pos.y+1, self.pos.x+16,
            self.pos.y+1, self.pos.x,
        );

        self.cache.1.clear();
        let _ = write!(self.cache.1,
            "\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[37mMem:  \x1b[91m[ \x1b[92m",
            self.pos.y+2, self.pos.x+16,
            self.pos.y+2, self.pos.x,
        );

        self.cache.2.clear();
        let _ = write!(self.cache.2,
            "\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[37mSwap: \x1b[91m[ \x1b[92m",
            self.pos.y+3, self.pos.x+16,
            self.pos.y+3, self.pos.x,
        );
    }

    pub fn draw (&mut self, buffer: &mut Vec::<u8>) -> Result<()> {
        let cpu_avg = if let Ok(cpuinfo) = self.system.cpuinfo.lock() {
            cpuinfo.cpu_avg
        } else {
            bail!("cpuinfo lock is poisoned!");
        };

        let (mem_use, swap_use) = if let Ok(memoryinfo) = self.system.memoryinfo.lock() {
            ((memoryinfo.mem_used as f32 / memoryinfo.mem_total.load(Ordering::Relaxed) as f32) * 100.0,
            (memoryinfo.swap_used as f32 / memoryinfo.swap_total as f32) * 100.0)
        } else {
            bail!("memoryinfo lock is poisoned!");
        };

        if cpu_avg < 100.0 {
            let _ = write!(buffer, "{}{:4.1}%\x1b[91m ]\x1b[0m", &self.cache.0, cpu_avg);
        } else {
            let _ = write!(buffer, "{}{:4.0}%\x1b[91m ]\x1b[0m", &self.cache.0, cpu_avg);
        }

        if mem_use < 100.0 {
            let _ = write!(buffer, "{}{:4.1}%\x1b[91m ]\x1b[0m", &self.cache.1, mem_use);
        } else {
            let _ = write!(buffer, "{}{:4.0}%\x1b[91m ]\x1b[0m", &self.cache.1, mem_use);
        }

        if swap_use < 100.0 {
            let _ = write!(buffer, "{}{:4.1}%\x1b[91m ]\x1b[0m", &self.cache.2, swap_use);
        } else {
            let _ = write!(buffer, "{}{:4.0}%\x1b[91m ]\x1b[0m", &self.cache.2, swap_use);
        }

        Ok(())
    }
}
