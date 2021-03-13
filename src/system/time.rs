use std::sync::{Arc, RwLock, mpsc};
use super::Config;

#[derive(Default)]
pub struct Time {
    pub time_string: String,
}

pub fn start_thread(internal: Arc<RwLock<Time>>, config: Arc<Config>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        // Set locale to whatever the environment is
        libc_strftime::set_locale();

        // Override frequency setting. We always want to update the time
        let sleepy = std::time::Duration::from_millis(1000);

        let (lock, cvar) = &*exit;

        'outer: loop {
            let current_time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();

            // Used to synchronize the update frequency to system time
            let st_subsec = current_time.subsec_micros();

            match internal.write() {
                Ok(mut val) => {
                    val.time_string = libc_strftime::strftime_local(config.strftime_format.as_str(), current_time.as_secs() as i64);
                },
                Err(_) => break,
            }

            match tx.send(1) {
                Ok(_) => (),
                Err(_) => break,
            }

            // Synchronize with actual time
            // It will be about 0.01s out of phase with actual time.
            // Should be accurate enough.
            if st_subsec > 10000 {
                if let Ok(mut exitvar) = lock.lock() {
                    loop {
                        // Slowly work your way towards ~10000 microseconds after the last Second
                        if let Ok(result) = cvar.wait_timeout(exitvar, sleepy - (std::time::Duration::from_micros(st_subsec as u64) / 10)) {
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
            } else {
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
            }
        }
    })
}
