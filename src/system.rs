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

// Used for the Time thread.
#[derive(Default)]
pub struct Time {
    pub exit: bool,
    pub time_string: String,
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

    pub time: Arc<RwLock<Time>>,
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

        // Stagger the threads so they don't all run at the same time
        let stagger = std::time::Duration::from_millis(250);


        // Time loop
        let internal = Arc::clone(&self.time);
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
                        if val.exit == true {
                            break;
                        }
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

                    //let mut exitvar = lock.lock().unwrap();

                        loop {
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

                    // Slowly work your way towards ~10000 microseconds after the last Second
                    //thread::sleep(sleepy - (std::time::Duration::from_micros(st_subsec as u64) / 10));
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
        }));

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
        let internal = Arc::clone(&self.loadavg);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();

        self.threads.push(thread::spawn(move || 'outer: loop {
            match internal.write() {
                Ok(mut val) => {
                    if val.exit == true {
                        break;
                    }
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
        }));


        // Read /proc/stat
        thread::sleep(stagger);  // Stagger the threads
        let internal = Arc::clone(&self.cpuinfo);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();

        self.threads.push(thread::spawn(move || 'outer: loop {
            match internal.write() {
                Ok(mut val) => {
                    /*if val.exit == true {
                        break;
                    }*/
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
        }));

        // Read /proc/meminfo
        thread::sleep(stagger);  // Stagger the threads
        let internal = Arc::clone(&self.memoryinfo);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();

        self.threads.push(thread::spawn(move || 'outer: loop {
            match internal.write() {
                Ok(mut val) => {
                    if val.exit == true {
                        break;
                    }
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
        }));

        // Read /proc/swaps
        thread::sleep(stagger);  // Stagger the threads
        let internal = Arc::clone(&self.swapinfo);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();

        self.threads.push(thread::spawn(move || 'outer: loop {
            match internal.write() {
                Ok(mut val) => {
                    if val.exit == true {
                        break;
                    }
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
        }));

        // Sensors
        thread::sleep(stagger);  // Stagger the threads
        let internal = Arc::clone(&self.sensorinfo);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();

        self.threads.push(thread::spawn(move || 'outer: loop {
            match internal.write() {
                Ok(mut val) => {
                    if val.exit == true {
                        break;
                    }
                    val.update();
                },
                Err(_) => break,
            };
            match tx.send(6) {
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
        }));

        // Network
        thread::sleep(stagger);  // Stagger the threads
        let internal = Arc::clone(&self.networkinfo);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();

        self.threads.push(thread::spawn(move || 'outer: loop {
            match internal.write() {
                Ok(mut val) => {
                    if val.exit == true {
                        break;
                    }
                    val.update();
                },
                Err(_) => break,
            };
            match tx.send(7) {
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
        }));

        // Processes
        thread::sleep(stagger);  // Stagger the threads
        let internal_cpuinfo = Arc::clone(&self.cpuinfo);
        let internal = Arc::clone(&self.processinfo);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();
        let config = Arc::clone(&self.config);

        self.threads.push(thread::spawn(move || 'outer: loop {
            match internal.write() {
                Ok(mut val) => {
                    if val.exit == true {
                        break;
                    }
                    //let now = std::time::Instant::now();
                    val.update(&internal_cpuinfo, &config);
                    //eprintln!("{}", now.elapsed().as_micros());
                },
                Err(_) => break,
            };
            match tx.send(8) {
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
        }));

        // GPU, the nvidia wrapper is weird so I put the code here.
        thread::sleep(stagger);  // Stagger the threads
        let internal = Arc::clone(&self.gpuinfo);
        let exit = Arc::clone(&self.exit);
        let tx = mtx.clone();

        self.threads.push(thread::spawn(move || {
            // Setup device
            if let Ok(nvml) = nvml_wrapper::NVML::init() {
                if let Ok(device) = nvml.device_by_index(0) {
                    'outer: loop {
                        match internal.write() {
                            Ok(mut val) => {
                                if val.exit == true {
                                    break;
                                }
                                if let Ok(temp) = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu) {
                                    val.temp = temp;
                                }
                                if let Ok(util) = device.utilization_rates() {
                                    val.gpu_load = util.gpu;
                                    val.mem_load = util.memory;
                                }
                                if let Ok(mem) = device.memory_info() {
                                    val.mem_used = (mem.used as f32 / mem.total as f32) * 100.0;
                                }
                            },
                            Err(_) => break,
                        };
                        match tx.send(9) {
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
                    }
                }
            }
        }));
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
