use std::sync::{Arc, RwLock, mpsc};

#[derive(Default)]
pub struct Swap {
    pub total: i64,
    pub used: i64,
    pub free: i64,
}

impl Swap {
    pub fn update(&mut self) {
        if let Ok(swapinfo) = std::fs::read_to_string("/proc/swaps") {
            self.total = 0;
            self.used = 0;

            'outer: for (idx, line) in swapinfo.lines().enumerate() {
                if idx != 0 {
                    for (i, s) in line.split_whitespace().enumerate() {
                        match i {
                            2 => {
                                if let Ok(total) = s.parse::<i64>() {
                                    self.total += total * 1024;  // convert from KB to B
                                } else {
                                    self.total = -1;
                                    break 'outer;
                                }
                            },
                            3 => {
                                if let Ok(used) = s.parse::<i64>() {
                                    self.used += used * 1024;  // convert from KB to B
                                } else {
                                    self.used = -1;
                                    break 'outer;
                                }
                            },
                            _ => (),
                        }
                    }
                }
            }

            if self.total != -1 && self.used != -1 {
                self.free = self.total - self.used;
            } else {
                self.total = -1;
                self.used = -1;
                self.free = -1;
            }
        } else {
            self.total = -1;
            self.used = -1;
            self.free = -1;
        }
    }
}

pub fn start_thread(internal: Arc<RwLock<Swap>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || 'outer: loop {
        match internal.write() {
            Ok(mut val) => {
                val.update();
            },
            Err(_) => break,
        };
        match tx.send(5) {
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
