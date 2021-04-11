use std::sync::{Arc, mpsc, atomic};

#[derive(Default)]
pub struct Time {
    pub time: atomic::AtomicU64,
}

pub fn start_thread(internal: Arc<Time>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        // Override frequency setting. We always want to update the time
        let sleepy = std::time::Duration::from_millis(1000);

        let (lock, cvar) = &*exit;

        'outer: loop {
            let current_time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();

            // Used to synchronize the update frequency to system time
            let st_subsec = current_time.subsec_micros();

            internal.time.store(current_time.as_secs(), std::sync::atomic::Ordering::Relaxed);

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

                            if *exitvar {
                                break 'outer;
                            }

                            if result.1.timed_out() {
                                break;
                            }
                        } else {
                            break 'outer;
                        }
                    }
                } else {
                    break;
                }
            } else if let Ok(mut exitvar) = lock.lock() {
                loop {
                    if let Ok(result) = cvar.wait_timeout(exitvar, sleepy) {
                        exitvar = result.0;

                        if *exitvar {
                            break 'outer;
                        }

                        if result.1.timed_out() {
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
    })
}
