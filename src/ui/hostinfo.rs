use std::io::Write as ioWrite;
//use std::fmt::Write as fmtWrite;

use crate::system::System;
use super::XY;

pub struct Hostinfo <'a> {
    pub system: &'a System,
    pub pos: XY,
    pub size: XY,

    cache: String,
}

impl <'a> Hostinfo <'a> {
    pub fn new(system: &'a System, pos: XY) -> Self {

        Self {
            system,
            cache: String::new(),
            pos,
            size: XY { x: 0, y: 1 }
        }
    }

    pub fn rebuild_cache (&mut self, terminal_size: &XY) {
        let dist_len = self.system.hostinfo.distname.len();
        let kern_len = self.system.hostinfo.kernel.len();

        // +9 is the static parts
        self.size.x = dist_len as u16 + kern_len as u16 + 9;

        self.cache.clear();
        self.cache.push_str(&format!(
                "\x1b[{};{}H\x1b[0K\x1b[{};{}H\x1b[91m[ {}{}\x1b[91m ] [ \x1b[0m{}\x1b[91m ]\x1b[0m",
                terminal_size.y, terminal_size.x.saturating_sub(self.size.x),
                terminal_size.y, terminal_size.x.saturating_sub(self.size.x),
                self.system.hostinfo.ansi_color,
                self.system.hostinfo.distname,
                self.system.hostinfo.kernel
            )
        );
    }

    pub fn draw (&mut self, buffer: &mut Vec::<u8>) {
        let _ = buffer.write(self.cache.as_bytes());
    }
}
