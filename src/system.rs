use std::sync::{Arc, RwLock, Mutex, atomic, Condvar};
use std::thread;

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
mod events;

// Holds all the commandline options.
#[derive(Default)]
pub struct Config {
    pub smaps: atomic::AtomicBool,
    pub topmode: atomic::AtomicBool,
    pub all: atomic::AtomicBool,
    pub frequency: atomic::AtomicU64,
    pub strftime_format: String,
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
    pub events: Arc<RwLock<events::Events>>,

    pub exit: Arc<(Mutex<bool>, Condvar)>,

    // Options
    pub config: Arc<Config>,

    pub threads: Vec<thread::JoinHandle<()>>,
    pub error: Arc<Mutex<Vec::<anyhow::Error>>>,
}

impl System {
    // This function starts all the monitoring threads
    pub fn start(&mut self, mtx: std::sync::mpsc::Sender<u8>) {
        // Update frequency
        let sleepy = std::time::Duration::from_millis(self.config.frequency.load(atomic::Ordering::Relaxed));

        // Stagger the threads so they don't all start at the same time
        // They will drift around but it shouldn't matter.
        //let stagger = std::time::Duration::from_millis(250);

        // Time loop
        self.threads.push(
            time::start_thread(
                Arc::clone(&self.time),
                Arc::clone(&self.config),
                mtx.clone(),
                Arc::clone(&self.exit)
            )
        );

        // Event loop
        self.threads.push(
            events::start_thread(
                Arc::clone(&self.events),
                Arc::clone(&self.config),
                mtx.clone(),
                Arc::clone(&self.exit)
            )
        );

        // Read /proc/loadavg
        //thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            loadavg::start_thread(
                Arc::clone(&self.loadavg),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Read /proc/stat
        let cpu_barrier = Arc::new(std::sync::Barrier::new(2));
        //thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            cpu::start_thread(
                Arc::clone(&self.cpuinfo),
                Arc::clone(&cpu_barrier),
                mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // Read /proc/meminfo
        //thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            memory::start_thread(
                Arc::clone(&self.memoryinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // Read /proc/swaps
        //thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            swap::start_thread(
                Arc::clone(&self.swapinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // Sensors
        //thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            sensors::start_thread(
                Arc::clone(&self.sensorinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Network
        //thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            network::start_thread(
                Arc::clone(&self.networkinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // Processes
        //thread::sleep(stagger);  // Stagger the threads
        self.threads.push(
            processes::start_thread(
                Arc::clone(&self.processinfo),
                Arc::clone(&self.cpuinfo),
                Arc::clone(&cpu_barrier),
                Arc::clone(&self.config),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // GPU
        //thread::sleep(stagger);  // Stagger the threads
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
