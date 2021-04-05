use crossterm::cursor;
use std::io::Write;
use anyhow::Result;

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

    pub fn rebuild_cache (&mut self) {
        self.cache.clear();
        self.cache.push_str(&format!(
            "{}\x1b[1K{}\x1b[0m",
            cursor::MoveTo(self.size.x, self.pos.y),
            cursor::MoveTo(0, self.pos.y)
        ));
    }

    pub fn draw (&mut self, stdout: &mut std::io::Stdout) -> Result<()> {
        let time_string = self.gettime();
        write!(stdout, "{}{}", &self.cache, &time_string)?;

        Ok(())
    }

    fn gettime(&mut self) -> String {
        let current_time = self.system.time.time.load(std::sync::atomic::Ordering::Relaxed);

        let time_string = libc_strftime::strftime_local(&self.system.config.strftime_format, current_time as i64);
        let length = time_string.len() as u16;

        if self.size.x != length {
            self.size.x = length;
            self.rebuild_cache();
        }

        time_string
    }
}
