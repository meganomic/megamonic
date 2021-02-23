#[derive(Default)]
struct Cpustats {
    user: i64,
    nice: i64,
    system: i64,
    idle: i64,
    iowait: i64,
    irq: i64,
    softirq: i64,
    steal: i64,
}

#[derive(Default)]
pub struct Cpuinfo {
    pub cpu_avg: f32,
    pub exit: bool,
    prev_idle: i64,
    idle: i64,
    prev_non_idle: i64,
    non_idle: i64,
    prev_total: i64,
    pub totald: i64,
    pub cpu_count: u8,
    stats: Cpustats,
}

impl Cpuinfo {
    pub fn update(&mut self) {
        //     prev_idle = previdle + previowait
        //     Idle = idle + iowait
        //
        //     prev_non_idle = prevuser + prevnice + prevsystem + previrq + prevsoftirq + prevsteal
        //     non_idle = user + nice + system + irq + softirq + steal
        //
        //     prev_total = prev_idle + prev_non_idle
        //     Total = Idle + non_idle
        //
        //     # differentiate: actual value minus the previous one
        //     totald = Total - prev_total
        //     idled = Idle - prev_idle
        //
        //     CPU_Percentage = (totald - idled)/totald
        if let Ok(procstat) = std::fs::read_to_string("/proc/stat") {
            if self.cpu_count == 0 {
                for (idx, line) in procstat.lines().enumerate() {
                    if idx > 0 {
                        if line.starts_with("cpu") {
                            self.cpu_count += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
            if let Some(line) = procstat.lines().nth(0) {
                //let cpu0stats = String::from(line);

                // Save previous stats
                self.prev_idle = self.stats.idle + self.stats.iowait;
                self.prev_non_idle = self.stats.user
                    + self.stats.nice
                    + self.stats.system
                    + self.stats.irq
                    + self.stats.softirq;
                self.prev_total = self.prev_idle + self.prev_non_idle;

                let mut error = false;

                for (i, s) in line.split_whitespace().enumerate() {
                    match i {
                        1 => self.stats.user = s.parse::<i64>().unwrap_or_else(|_| {  error = true; -1 }),
                        2 => self.stats.nice = s.parse::<i64>().unwrap_or_else(|_| {  error = true; -1 }),
                        3 => self.stats.system = s.parse::<i64>().unwrap_or_else(|_| {  error = true; -1 }),
                        4 => self.stats.idle = s.parse::<i64>().unwrap_or_else(|_| {  error = true; -1 }),
                        5 => self.stats.iowait = s.parse::<i64>().unwrap_or_else(|_| {  error = true; -1 }),
                        6 => self.stats.irq = s.parse::<i64>().unwrap_or_else(|_| {  error = true; -1 }),
                        7 => self.stats.softirq = s.parse::<i64>().unwrap_or_else(|_| {  error = true; -1 }),
                        8 => { self.stats.steal = s.parse::<i64>().unwrap_or_else(|_| {  error = true; -1 }); break; },
                        _ => (),
                    }
                }

                if !error {
                    self.idle = self.stats.idle + self.stats.iowait;
                    self.non_idle = self.stats.user
                        + self.stats.nice
                        + self.stats.system
                        + self.stats.irq
                        + self.stats.softirq
                        + self.stats.steal;

                    self.totald = (self.idle + self.non_idle) - self.prev_total;
                    //self.totald = totald;//self.non_idle - self.prev_non_idle + self.idle - self.prev_idle;

                    self.cpu_avg = (self.non_idle - self.prev_non_idle) as f32 / self.totald as f32 * 100.0;
                    //self.cpu_avg = (totald - (self.idle - self.prev_idle)) as f32 / totald as f32 * 100.0;
                } else {
                    self.cpu_avg = -1.0;
                }
            } else {
                self.cpu_avg = -1.0;
            }
        } else {
            self.cpu_avg = -1.0;
        }
    }
}
