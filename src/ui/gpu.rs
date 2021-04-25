use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::{ bail, Result };

use crate::system::System;
use super::XY;

pub struct Gpu <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache1: String,
    cache2: String,
    cache3: String,
    cache4: String,
}

impl <'a> Gpu <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        Self {
            system,
            cache1: String::new(),
            cache2: String::new(),
            cache3: String::new(),
            cache4: String::new(),
            pos,
            size: XY { x: 23, y: 5 }
        }
    }

    pub fn rebuild_cache(&mut self) {
        self.cache1.clear();
        self.cache2.clear();
        self.cache3.clear();
        self.cache4.clear();

        let _ = write!(self.cache1,
        "\x1b[{};{}H\x1b[95mGpu\x1b[0m\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[37mTemp:         \x1b[91m[ \x1b[92m",
            self.pos.y, self.pos.x,
            self.pos.y + 1, self.pos.x + 24,
            self.pos.y + 2, self.pos.x + 24,
            self.pos.y + 3, self.pos.x + 24,
            self.pos.y + 4, self.pos.x + 24,
            self.pos.y + 1, self.pos.x,
        );


        let _ = write!(self.cache2,
            " C\x1b[91m ]\x1b[0m\x1b[{};{}H\x1b[37mGpu load:     \x1b[91m[ \x1b[92m",
            self.pos.y + 2, self.pos.x,
        );

        let _ = write!(self.cache3,
            "%\x1b[91m ]\x1b[0m\x1b[{};{}H\x1b[37mMem load:     \x1b[91m[ \x1b[92m",
            self.pos.y + 3, self.pos.x,
        );

        let _ = write!(self.cache4,
            "%\x1b[91m ]\x1b[0m\x1b[{};{}H\x1b[37mMem use:      \x1b[91m[ \x1b[92m",
            self.pos.y + 4, self.pos.x,
        );


    }

    // 2550 -> 2050
    pub fn draw(&mut self, buffer: &mut Vec::<u8>) -> Result<()> {
        if let Ok(val) = self.system.gpuinfo.lock() {
            if val.mem_used < 100.0 {
                write!(buffer, "{}{:>3}{}{:>4}{}{:>4}{}{:>4.1}%\x1b[91m ]\x1b[0m", &self.cache1, val.temp, &self.cache2, val.gpu_load, &self.cache3, val.mem_load, &self.cache4, val.mem_used)?;
            } else {
                write!(buffer, "{}{:>3}{}{:>4}{}{:>4}{}{:>4.0}%\x1b[91m ]\x1b[0m", &self.cache1, val.temp, &self.cache2, val.gpu_load, &self.cache3, val.mem_load, &self.cache4, val.mem_used)?;
            }
        } else {
            bail!("gpuinfo lock is poisoned!");
        }

        Ok(())
    }
}
