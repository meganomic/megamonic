use crossterm::{ cursor, queue, style::Print };
use std::io::Write;
use anyhow::Result;

use crate::system::System as System;
use super::XY as XY;

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

    pub fn update_cache(&mut self) {
        self.cache1.clear();
        self.cache2.clear();
        self.cache3.clear();
        self.cache4.clear();

    }

    pub fn draw_static(&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        queue!(
            stdout,
            cursor::MoveTo(self.pos.x, self.pos.y),
            Print("\x1b[95mGpu\x1b[0m")
        )?;

        Ok(())
    }

    pub fn draw(&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        self.draw_static(stdout)?;

        if let Ok(val) = self.system.gpuinfo.lock() {
            if self.cache1.is_empty() {
                self.cache1.push_str(format!(
                "{}\x1b[1K{}\x1b[1K{}\x1b[1K{}\x1b[1K{}\x1b[37mTemp:         \x1b[91m[ \x1b[92m",
                    cursor::MoveTo(self.pos.x+24, self.pos.y+1),
                    cursor::MoveTo(self.pos.x+24, self.pos.y+2),
                    cursor::MoveTo(self.pos.x+24, self.pos.y+3),
                    cursor::MoveTo(self.pos.x+24, self.pos.y+4),
                    cursor::MoveTo(self.pos.x, self.pos.y+1),
                ).as_str());
            }

            if self.cache2.is_empty() {
                self.cache2.push_str(format!(
                    " C\x1b[91m ]\x1b[0m{}\x1b[37mGpu load:     \x1b[91m[ \x1b[92m",
                    cursor::MoveTo(self.pos.x, self.pos.y+2),
                ).as_str());
            }
            if self.cache3.is_empty() {
                self.cache3.push_str(format!(
                    "%\x1b[91m ]\x1b[0m{}\x1b[37mMem load:     \x1b[91m[ \x1b[92m",
                    cursor::MoveTo(self.pos.x, self.pos.y+3),
                ).as_str());
            }
            if self.cache4.is_empty() {
                self.cache4.push_str(format!(
                    "%\x1b[91m ]\x1b[0m{}\x1b[37mMem use:      \x1b[91m[ \x1b[92m",
                    cursor::MoveTo(self.pos.x, self.pos.y+4),
                ).as_str());
            }

            queue!(
                stdout,

                Print(&self.cache1),

                Print(&format!("{:>3}", val.temp)),
                Print(&self.cache2),

                Print(&format!("{:>4}", val.gpu_load)),
                Print(&self.cache3),

                Print(&format!("{:>4}", val.mem_load)),
                Print(&self.cache4)
            )?;

            if val.mem_used < 100.0 {
                queue!(stdout,
                    Print(&format!("{:>4.1}", val.mem_used)),
                    Print("%\x1b[91m ]\x1b[0m")
                )?;
            } else if val.mem_used >= 100.0 {
                queue!(stdout,
                    Print(&format!("{:>4.0}", val.mem_used)),
                    Print("%\x1b[91m ]\x1b[0m")
                )?;
            }

}
        Ok(())
    }
}
