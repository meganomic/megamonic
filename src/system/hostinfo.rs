pub struct Hostinfo {
    pub distname: String,
    pub kernel: String,
    pub ansi_color: String,
}

impl Default for Hostinfo {
    fn default() -> Self {
        let mut distname = String::new();
        let mut ansi_color = String::from("\x1b[97m");

        let osrelease = std::fs::read_to_string("/etc/os-release").unwrap_or_else(|_| "NAME=\"Not Found\"".to_string());

        for line in osrelease.lines() {
            if line.starts_with("NAME") {
                if let Some(pos) = line.find('"') {
                    distname.push_str(&line[pos+1..line.len()-1].trim());
                }
            } else if line.starts_with("ANSI_COLOR") {
                if let Some(pos) = line.find('"') {
                    ansi_color.clear();
                    let color = &line[pos+1..line.len()-1]; //.unwrap_or("\x1b[97m").to_string();
                    ansi_color.push_str("\x1b[");
                    ansi_color.push_str(color);
                    ansi_color.push('m');

                }
            }
        }

        let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .unwrap_or_else(|_|"Not Found".to_string())
            .trim()
            .to_string();

        Hostinfo {
            distname,
            kernel,
            ansi_color,
        }
    }
}
