use std::sync::{Arc, RwLock, atomic};
use std::thread;
use crossterm::event::{read, Event, KeyCode, KeyModifiers};

pub mod cpu;
mod loadavg;
mod memory;
mod swap;
mod sensors;
mod network;
mod processes;
mod gpu;
mod hostinfo;
mod time;

// Holds all the commandline options.
#[derive(Default)]
pub struct Config {
    pub smaps: atomic::AtomicBool,
    pub topmode: atomic::AtomicBool,
    pub all: atomic::AtomicBool,
    pub frequency: atomic::AtomicU64,
    pub strftime_format: String,
}

// Used in the Event thread.
#[derive(Default)]
pub struct Events {
    pub tsizex: u16,
    pub tsizey: u16,
}

#[derive(Default)]
pub struct System {
    // Info gathering structs
    pub cpuinfo: Arc<RwLock<cpu::Cpuinfo>>,
    pub loadavg: Arc<RwLock<loadavg::Loadavg>>,
    pub memoryinfo: Arc<RwLock<memory::Memory>>,
    pub swapinfo: Arc<RwLock<swap::Swap>>,
    pub sensorinfo: Arc<RwLock<sensors::Sensors>>,
    pub networkinfo: Arc<RwLock<network::Network>>,
    pub processinfo: Arc<RwLock<processes::Processes>>,
    pub gpuinfo: Arc<RwLock<gpu::Gpu>>,
    pub hostinfo: hostinfo::Hostinfo,

    pub time: Arc<RwLock<time::Time>>,
    pub events: Arc<RwLock<Events>>,

    pub exit: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,

    // Options
    pub config: Arc<Config>,

    pub threads: Vec<thread::JoinHandle<()>>,
}

impl System {
    // This function starts all the monitoring threads
    pub fn start(&mut self, mtx: std::sync::mpsc::Sender<u8>) {
        // Update frequency
        let sleepy = std::time::Duration::from_millis(self.config.frequency.load(atomic::Ordering::Relaxed));

        // Stagger the threads so they don't all start at the same time
        // They will drift around but it shouldn't matter.
        let stagger = std::time::Duration::from_millis(250);


        // Time loop
        self.threads.push(
            time::start_thread(
                Arc::clone(&self.time),
                Arc::clone(&self.config),
                mtx.clone(),
                Arc::clone(&self.exit)
            )
        );
        /*let internal = Arc::clone(&self.time);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();

        // Used for strftime_format
        let config = Arc::clone(&self.config);

        self.threads.push(thread::spawn(move || {
            // Set locale to whatever the environment is
            libc_strftime::set_locale();

            // Override frequency setting. We always want to update the time
            let sleepy = std::time::Duration::from_millis(1000);

            'outer: loop {
                let current_time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();

                // Used to synchronize the update frequency to system time
                let st_subsec = current_time.subsec_micros();

                match internal.write() {
                    Ok(mut val) => {
                        val.time_string = libc_strftime::strftime_local(config.strftime_format.as_str(), current_time.as_secs() as i64);
                    },
                    Err(_) => break,
                };
                match tx.send(1) {
                    Ok(_) => (),
                    Err(_) => break,
                };
                // Synchronize with actual time
                // It will be about 0.01s out of phase with actual time.
                // Should be accurate enough.
                if st_subsec > 10000 {
                    let (lock, cvar) = &*exit;
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
                }
            }
        }));*/

        // Event loop
        let internal = Arc::clone(&self.events);
        //let internal_updated = Arc::clone(&self.updated);
        let tx = mtx.clone();
        let config = Arc::clone(&self.config);

        self.threads.push(thread::spawn(move || loop {
            if let Ok(ev) = read() {
                match ev {
                    Event::Key(key) => {
                        if key.code == KeyCode::Char('q') || (key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL) {
                            match tx.send(255) {
                                Ok(_) => (),
                                Err(_) => break,
                            };
                            break;
                        } else if key.code == KeyCode::Char(' ') {
                            match tx.send(101) {
                                Ok(_) => (),
                                Err(_) => break,
                            };
                        } else if key.code == KeyCode::Char('t') {
                            if config.topmode.load(atomic::Ordering::Acquire) {
                                config.topmode.store(false, atomic::Ordering::Release);
                            } else {
                                config.topmode.store(true, atomic::Ordering::Release);
                            }

                            match tx.send(102) {
                                Ok(_) => (),
                                Err(_) => break,
                            };
                        } else if key.code == KeyCode::Char('s') {
                            if config.smaps.load(atomic::Ordering::Acquire) {
                                config.smaps.store(false, atomic::Ordering::Release);
                            } else {
                                config.smaps.store(true, atomic::Ordering::Release);
                            }

                            match tx.send(103) {
                                Ok(_) => (),
                                Err(_) => break,
                            };
                        } else if key.code == KeyCode::Char('a') {
                            if config.all.load(atomic::Ordering::Acquire) {
                                config.all.store(false, atomic::Ordering::Release);
                            } else {
                                config.all.store(true, atomic::Ordering::Release);
                            }

                            match tx.send(104) {
                                Ok(_) => (),
                                Err(_) => break,
                            };
                        } else if key.code == KeyCode::Char('r') {
                            match tx.send(106) {
                                Ok(_) => (),
                                Err(_) => break,
                            };
                        }
                    },
                    Event::Resize(width, height) => {
                        if let Ok(mut val) = internal.write() {
                            val.tsizex = width;
                            val.tsizey = height;
                        }

                        match tx.send(105) {
                                Ok(_) => (),
                                Err(_) => break,
                            };
                    },
                    _ => (),
                }
            }
        }));

        // Read /proc/loadavg
        thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            loadavg::start_thread(
                Arc::clone(&self.loadavg),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Read /proc/stat
        thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            cpu::start_thread(
                Arc::clone(&self.cpuinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Read /proc/meminfo
        thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            memory::start_thread(
                Arc::clone(&self.memoryinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Read /proc/swaps
        thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            swap::start_thread(
                Arc::clone(&self.swapinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Sensors
        thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            sensors::start_thread(
                Arc::clone(&self.sensorinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Network
        thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            network::start_thread(
                Arc::clone(&self.networkinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Processes
        thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            processes::start_thread(
                Arc::clone(&self.processinfo),
                Arc::clone(&self.cpuinfo),
                Arc::clone(&self.config),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // GPU
        thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            gpu::start_thread(
                Arc::clone(&self.gpuinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );
    }

    // This function stops all the monitoring threads
    pub fn stop(&mut self) {
        // Notify all threads that they should exit
        {
            let (lock, cvar) = &*self.exit;
            if let Ok(mut exitvar) = lock.lock() {
                *exitvar = true;
                cvar.notify_all();
            }
        }

        while !self.threads.is_empty() {
            if let Some(val) = self.threads.pop() {
                // If the thread is broken just go to the next one
                let _ = val.join();
            }
        }
    }
}
