use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;

use crate::system::System;
use super::XY;

pub struct Time <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: String,
}

impl <'a> Time <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {
        // Set locale to whatever the environment is
        libc_strftime::tz_set();
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
        let _ = write!(self.cache,
            "\x1b[{};{}H\x1b[1K\x1b[{};{}H\x1b[0m",
            self.pos.y, self.size.x,
            self.pos.y, self.pos.x
        );
    }

    pub fn draw (&mut self, buffer: &mut Vec::<u8>) {
        let time_string = self.gettime();

        let _ = buffer.write_vectored(&[
            std::io::IoSlice::new(self.cache.as_bytes()),
            std::io::IoSlice::new(time_string.as_bytes()),
        ]);
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
