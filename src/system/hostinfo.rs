use crossterm::style::{Colored, Color};

pub struct Hostinfo {
    pub distname: String,
    pub kernel: String,
    pub ansi_color: Colored,
}

impl Default for Hostinfo {
    fn default() -> Self {
        let mut distname = String::new();
        let mut ansi_color = Colored::ForegroundColor(Color::White);

        let osrelease = std::fs::read_to_string("/etc/os-release").unwrap_or("NAME=\"Not Found\"".to_string());

        for line in osrelease.lines() {
            if line.starts_with("NAME") {
                if let Some(pos) = line.find('"') {
                    distname.push_str(&line[pos+1..line.len()-1].trim());
                }
            } else if line.starts_with("ANSI_COLOR") {
                if let Some(pos) = line.find('"') {
                    ansi_color = Colored::parse_ansi(&line[pos+1..line.len()-1]).unwrap_or(Colored::ForegroundColor(Color::White));
                    }
            }
        }

        let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .unwrap_or("Not Found".to_string())
            .trim()
            .to_string();

        Hostinfo {
            distname,
            kernel,
            ansi_color,
        }
    }
}
