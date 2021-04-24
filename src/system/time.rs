use std::sync::{Arc, mpsc, atomic};
use std::time::{ SystemTime, Duration };

#[derive(Default)]
pub struct Time {
    pub time: atomic::AtomicU64,
}

pub fn start_thread(internal: Arc<Time>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new().name("Time".to_string()).spawn(move || {
        let (lock, cvar) = &*exit;

        'outer: loop {
            let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("Time is messed up!");

            // Used to synchronize the update frequency to system time
            let st_subsec = current_time.subsec_micros();

            internal.time.store(current_time.as_secs(), atomic::Ordering::Release);

            match tx.send(1) {
                Ok(_) => (),
                Err(_) => break,
            }

            // Synchronize with actual time
            // It will be about 0.01s out of phase with actual time.
            // Should be accurate enough.
            let sleepy = if st_subsec > 10000 {
                // Slowly work your way towards ~10000 microseconds after the last Second
                Duration::from_millis(1000) - (Duration::from_micros(st_subsec as u64) / 10)
            } else {
                Duration::from_millis(1000)
            };

            if let Ok(mut exitvar) = lock.lock() {
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
    }).expect("Couldn't spawn Time thread")
}
