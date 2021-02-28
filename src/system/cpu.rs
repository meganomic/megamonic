use std::sync::{Arc, RwLock, mpsc};

#[derive(Default)]
struct Cpustats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

#[derive(Default)]
pub struct Cpuinfo {
    pub cpu_avg: f32,
    pub totald: u64,
    pub cpu_count: u8,
    idle: u64,
    non_idle: u64,
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
                let prev_idle = self.stats.idle + self.stats.iowait;
                let prev_non_idle = self.stats.user
                    + self.stats.nice
                    + self.stats.system
                    + self.stats.irq
                    + self.stats.softirq
                    + self.stats.steal;
                let prev_total = prev_idle + prev_non_idle;

                let mut error = false;

                for (i, s) in line.split_whitespace().enumerate() {
                    match i {
                        1 => self.stats.user = s.parse::<u64>().unwrap_or_else(|_| {  error = true; 0 }),
                        2 => self.stats.nice = s.parse::<u64>().unwrap_or_else(|_| {  error = true; 0 }),
                        3 => self.stats.system = s.parse::<u64>().unwrap_or_else(|_| {  error = true; 0 }),
                        4 => self.stats.idle = s.parse::<u64>().unwrap_or_else(|_| {  error = true; 0 }),
                        5 => self.stats.iowait = s.parse::<u64>().unwrap_or_else(|_| {  error = true; 0 }),
                        6 => self.stats.irq = s.parse::<u64>().unwrap_or_else(|_| {  error = true; 0 }),
                        7 => self.stats.softirq = s.parse::<u64>().unwrap_or_else(|_| {  error = true; 0 }),
                        8 => { self.stats.steal = s.parse::<u64>().unwrap_or_else(|_| {  error = true; 0 }); break; },
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

                    self.totald = (self.idle + self.non_idle) - prev_total;

                    self.cpu_avg = ((self.non_idle - prev_non_idle) as f32 / self.totald as f32) * 100.0;

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

pub fn start_thread(internal: Arc<RwLock<Cpuinfo>>, barrier: Arc<std::sync::Barrier>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || 'outer: loop {
        match internal.write() {
            Ok(mut val) => {
                barrier.wait();
                val.update();
            },
            Err(_) => break
        }
        match tx.send(3) {
            Ok(_) => (),
            Err(_) => break
        }

        let (lock, cvar) = &*exit;
        if let Ok(mut exitvar) = lock.lock() {
            loop {
                if let Ok(result) = cvar.wait_timeout(exitvar, sleepy) {
                    exitvar = result.0;

                    if *exitvar == true {
                        break 'outer;
                    }

                    if result.1.timed_out() == true {
                        break;
                    }
                } else {
                    break 'outer;
                }
            }
        } else {
            break;
        }
    })
}
