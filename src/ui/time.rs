use crossterm::{ cursor, queue, style::Print };
use std::io::Write;
use anyhow::{ anyhow, Result };

use crate::system::System as System;
use super::XY as XY;

pub struct Time <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: String,
}

impl <'a> Time <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        // Set locale to whatever the environment is
        libc_strftime::set_locale();

        Self {
            system,
            cache: String::new(),
            pos,
            size: XY { x: 0, y: 1 }
        }
    }

    pub fn update_cache (&mut self) {
        self.cache.clear();
        self.cache.push_str(&format!(
            "{}\x1b[1K{}\x1b[0m",
            cursor::MoveTo(self.size.x, self.pos.y),
            cursor::MoveTo(0, self.pos.y)
        ));
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        let time_string = self.gettime()?;
        queue!(stdout,
            Print(&self.cache),
            Print(&time_string)
        )?;

        Ok(())
    }

    fn gettime(&mut self) -> Result<String> {
        let current_time = if let Ok(timeinfo) = self.system.time.read() {
            timeinfo.time
        } else {
            return Err(anyhow!("Couldn't aquire system.time lock!"));
        };

        let time_string = libc_strftime::strftime_local(&self.system.config.strftime_format, current_time as i64);
        let length = time_string.len() as u16;

        if self.size.x != length {
            self.size.x = length;
            self.update_cache();
        }

        Ok(time_string)
    }
}
