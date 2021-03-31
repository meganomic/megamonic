use crossterm::{ cursor, queue, style::Print };
use std::io::Write;
use anyhow::Result;

use crate::system::System as System;
use super::XY as XY;

pub struct Overview <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: Vec::<String>,
}

impl <'a> Overview <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        let mut cache = Vec::<String>::new();
        cache.push(String::new());
        cache.push(String::new());
        cache.push(String::new());

        Self {
            system,
            cache,
            pos,
            size: XY { x: 15, y: 4 }
        }
    }

    pub fn update_cache (&mut self) {
        unsafe {
            let cache1 = self.cache.get_unchecked_mut(0);
            cache1.clear();
            cache1.push_str(&format!(
                "{}\x1b[95mOverview\x1b[0m{}\x1b[1K{}\x1b[37mCPU:  \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x, self.pos.y),
                cursor::MoveTo(self.pos.x+16, self.pos.y+1),
                cursor::MoveTo(self.pos.x, self.pos.y+1),
            ));

            let cache2 = self.cache.get_unchecked_mut(1);
            cache2.clear();
            cache2.push_str(&format!(
                "{}\x1b[1K{}\x1b[37mMem:  \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x+16, self.pos.y+2),
                cursor::MoveTo(self.pos.x, self.pos.y+2),
            ));

            let cache3 = self.cache.get_unchecked_mut(2);
            cache3.clear();
            cache3.push_str(&format!(
                "{}\x1b[1K{}\x1b[37mSwap: \x1b[91m[ \x1b[92m",
                cursor::MoveTo(self.pos.x+16, self.pos.y+3),
                cursor::MoveTo(self.pos.x, self.pos.y+3),
            ));
        }
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {

        if let Ok(cpuinfo) = self.system.cpuinfo.read() {
            if cpuinfo.cpu_avg < 100.0 {
                unsafe {
                    queue!(stdout,
                        Print(&self.cache.get_unchecked(0)),
                        Print(&format!("{:4.1}%\x1b[91m ]\x1b[0m", cpuinfo.cpu_avg)),
                    )?;
                }
            } else if cpuinfo.cpu_avg >= 100.0 {
                unsafe {
                    queue!(stdout,
                        Print(&self.cache.get_unchecked(0)),
                        Print(&format!("{:4.0}%\x1b[91m ]\x1b[0m", cpuinfo.cpu_avg)),
                    )?;
                }
            }

            queue!(stdout,
                //cursor::MoveTo(40, 5),
                Print(&format!("\x1b[6;41H               \x1b[6;41H\x1b[38;5;244m{}\x1b[0m", &cpuinfo.governor))
            )?;
        }

        if let Ok(memoryinfo) = self.system.memoryinfo.lock() {
            let mem_use = (memoryinfo.used as f32 / memoryinfo.total as f32) * 100.0;

            if mem_use < 100.0 {
                unsafe {
                    queue!(stdout,
                        Print(&self.cache.get_unchecked(1)),
                        Print(&format!("{:4.1}%\x1b[91m ]\x1b[0m", mem_use)),
                    )?;
                }
            } else if mem_use >= 100.0 {
                unsafe {
                    queue!(stdout,
                        Print(&self.cache.get_unchecked(1)),
                        Print(&format!("{:4.0}%\x1b[91m ]\x1b[0m", mem_use)),
                    )?;
                }
            }
        }

        if let Ok(swapinfo) = self.system.swapinfo.lock() {
            let swap_use = (swapinfo.used as f32 / swapinfo.total as f32) * 100.0;

            if swap_use < 100.0 {
                unsafe {
                    queue!(stdout,
                        Print(&self.cache.get_unchecked(2)),
                        Print(&format!("{:4.1}%\x1b[91m ]\x1b[0m", swap_use)),
                    )?;
                }
            } else if swap_use >= 100.0 {
                unsafe {
                    queue!(stdout,
                        Print(&self.cache.get_unchecked(2)),
                        Print(&format!("{:4.0}%\x1b[91m ]\x1b[0m", swap_use)),
                    )?;
                }
            }
        }

        Ok(())
    }
}
