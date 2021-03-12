use std::sync::{Arc, RwLock, mpsc};

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

                if idx == 3 {
                    break 'outer;
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

pub fn start_thread(internal: Arc<RwLock<Memory>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || 'outer: loop {
        match internal.write() {
            Ok(mut val) => {
                val.update();
            },
            Err(_) => break,
        };
        match tx.send(4) {
            Ok(_) => (),
            Err(_) => break,
        };
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
