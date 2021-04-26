use std::sync::{Arc, Mutex, atomic, Condvar};
use std::thread;

mod cpu;
mod loadavg;
mod memory;
mod sensors;
mod network;
pub mod processes;
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
    pub cpuinfo: Arc<Mutex<cpu::Cpuinfo>>,
    pub loadavg: Arc<Mutex<loadavg::Loadavg>>,
    pub memoryinfo: Arc<Mutex<memory::Memory>>,
    pub sensorinfo: Arc<Mutex<sensors::Sensors>>,
    pub networkinfo: Arc<Mutex<network::Network>>,
    pub processinfo: Arc<Mutex<processes::Processes>>,
    pub gpuinfo: Arc<Mutex<gpu::Gpu>>,
    pub hostinfo: hostinfo::Hostinfo,

    pub time: Arc<time::Time>,
    pub events: Arc<Mutex<events::Events>>,

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

        // Read /proc/stat
        self.threads.push(
            cpu::start_thread(
                Arc::clone(&self.cpuinfo),
                 mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // Processes
        self.threads.push(
            processes::start_thread(
                Arc::clone(&self.processinfo),
                Arc::clone(&self.cpuinfo),
                Arc::clone(&self.config),
                mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // Time loop
        self.threads.push(
            time::start_thread(
                Arc::clone(&self.time),
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
            )
        );

        // Read /proc/loadavg
        self.threads.push(
            loadavg::start_thread(
                Arc::clone(&self.loadavg),
                mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // Read /proc/meminfo
        self.threads.push(
            memory::start_thread(
                Arc::clone(&self.memoryinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // Sensors
        self.threads.push(
            sensors::start_thread(
                Arc::clone(&self.sensorinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                sleepy
            )
        );

        // Network
        self.threads.push(
            network::start_thread(
                Arc::clone(&self.networkinfo),
                mtx.clone(),
                Arc::clone(&self.exit),
                Arc::clone(&self.error),
                sleepy
            )
        );

        // GPU
        self.threads.push(
            gpu::start_thread(
                Arc::clone(&self.gpuinfo),
                mtx,
                Arc::clone(&self.exit),
                sleepy
            )
        );
    }

    // This function stops all the monitoring threads
    pub fn stop(&mut self) {
        // Notify all threads that they should exit
        let (lock, cvar) = &*self.exit;
        if let Ok(mut exitvar) = lock.lock() {
            *exitvar = true;
            cvar.notify_all();
        }

        while !self.threads.is_empty() {
            if let Some(val) = self.threads.pop() {
                // If the thread is broken just go to the next one
                let _ = val.join();
            }
        }
    }
}
