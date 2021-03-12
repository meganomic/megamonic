use std::sync::{Arc, RwLock, mpsc};

#[derive(Default)]
pub struct Loadavg {
    pub min1: String,
    pub min5: String,
    pub min15: String,
}

impl Loadavg {
    pub fn update(&mut self) {
        if let Ok(procloadavg) = std::fs::read_to_string("/proc/loadavg") {
            self.min1.clear();
            self.min5.clear();
            self.min15.clear();


            for (i, s) in procloadavg.split_whitespace().enumerate() {
                match i {
                    0 => self.min1.push_str(s),
                    1 => self.min5.push_str(s),
                    2 => { self.min15.push_str(s); break; },
                    _ => (),
                }
            }
        } else {
            self.min1.push_str("Error");
            self.min5.push_str("Error");
            self.min15.push_str("Error");
        }
    }
}

pub fn start_thread(internal: Arc<RwLock<Loadavg>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || 'outer: loop {
        match internal.write() {
            Ok(mut val) => {
                val.update();
            },
            Err(_) => break,
        };
        match tx.send(2) {
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
