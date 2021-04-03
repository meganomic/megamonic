use crossterm::cursor;
use std::io::Write;
use anyhow::Result;

use crate::system::System as System;
use super::XY as XY;

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
        self.cache.0.push_str(&format!(
            "{}\x1b[95mOverview\x1b[0m{}\x1b[1K{}\x1b[37mCPU:  \x1b[91m[ \x1b[92m",
            cursor::MoveTo(self.pos.x, self.pos.y),
            cursor::MoveTo(self.pos.x+16, self.pos.y+1),
            cursor::MoveTo(self.pos.x, self.pos.y+1),
        ));

        self.cache.1.clear();
        self.cache.1.push_str(&format!(
                "{}\x1b[1K{}\x1b[37mMem:  \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x+16, self.pos.y+2),
                cursor::MoveTo(self.pos.x, self.pos.y+2),
            ));

        self.cache.2.clear();
        self.cache.2.push_str(&format!(
                "{}\x1b[1K{}\x1b[37mSwap: \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x+16, self.pos.y+3),
                cursor::MoveTo(self.pos.x, self.pos.y+3),
            ));
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        if let Ok(cpuinfo) = self.system.cpuinfo.read() {
            if cpuinfo.cpu_avg < 100.0 {
                    write!(stdout, "{}{:4.1}%\x1b[91m ]\x1b[0m", &self.cache.0, cpuinfo.cpu_avg)?;
            } else if cpuinfo.cpu_avg >= 100.0 {
                    write!(stdout, "{}{:4.0}%\x1b[91m ]\x1b[0m", &self.cache.0, cpuinfo.cpu_avg)?;
            }
        }

        if let Ok(memoryinfo) = self.system.memoryinfo.lock() {
            let mem_use = (memoryinfo.used as f32 / memoryinfo.total as f32) * 100.0;

            if mem_use < 100.0 {
                    write!(stdout, "{}{:4.1}%\x1b[91m ]\x1b[0m", &self.cache.1, mem_use)?;
            } else if mem_use >= 100.0 {
                    write!(stdout, "{}{:4.0}%\x1b[91m ]\x1b[0m", &self.cache.1, mem_use)?;
            }
        }

        if let Ok(swapinfo) = self.system.swapinfo.lock() {
            let swap_use = (swapinfo.used as f32 / swapinfo.total as f32) * 100.0;

            if swap_use < 100.0 {
                    write!(stdout, "{}{:4.1}%\x1b[91m ]\x1b[0m", &self.cache.2, swap_use)?;
            } else if swap_use >= 100.0 {
                    write!(stdout, "{}{:4.0}%\x1b[91m ]\x1b[0m", &self.cache.2, swap_use)?;
            }
        }

        Ok(())
    }
}
