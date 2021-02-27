#[derive(Default, Clone)]
pub struct Memory {
    pub total: i64,
    pub free: i64,
    pub used: i64,
}

impl Memory {
    pub fn update(&mut self) {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            'outer: for (idx, line) in meminfo.lines().enumerate() {
                if idx == 0 {
                    if let Ok(total) = line.split_whitespace().nth(1).unwrap_or_default().parse::<i64>() {
                        self.total = total * 1024;  // convert from KB to B
                    } else {
                        self.total = -1;
                    }
                }

                if idx == 2 {
                    for (i, s) in line.split_whitespace().enumerate() {
                        if i == 1 {
                            if let Ok(free) = s.parse::<i64>() {
                                self.free = free * 1024;  // convert from KB to B
                                break 'outer;
                            } else {
                                self.free = -1;
                                break 'outer;
                            }
                        }
                    }
                }
            }
            self.used = self.total - self.free;

        } else {
            self.total = -1;
            self.free = -1;
            self.used = -1;
        }
    }
}
