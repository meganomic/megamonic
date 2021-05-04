use std::sync::{Arc, Mutex, atomic, Condvar};
use std::thread;
use anyhow::{ Result, Context };

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

//#[derive(Default)]
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

    pub exit: Arc<(Mutex<bool>, Condvar)>,

    // Options
    pub config: Arc<Config>,

    pub threads: Vec<thread::JoinHandle<()>>,
    pub error: Arc<Mutex<Vec::<anyhow::Error>>>,
}

impl System {
    pub fn new(config: Config) -> Result<Self> {
        let processes = processes::Processes::new().context("Can't initialize Procesess")?;

        let system = Self {
            cpuinfo: Arc::new(Mutex::new(cpu::Cpuinfo::default())),
            loadavg: Arc::new(Mutex::new(loadavg::Loadavg::default())),
            memoryinfo: Arc::new(Mutex::new(memory::Memory::default())),
            sensorinfo: Arc::new(Mutex::new(sensors::Sensors::default())),
            networkinfo: Arc::new(Mutex::new(network::Network::default())),
            processinfo: Arc::new(Mutex::new(processes)),
            gpuinfo: Arc::new(Mutex::new(gpu::Gpu::default())),
            hostinfo: hostinfo::Hostinfo::default(),

            time: Arc::new(time::Time::default()),

            exit: Arc::new((Mutex::new(false), Condvar::new())),

            config: Arc::new(config),

            threads: Vec::new(),
            error: Arc::new(Mutex::new(Vec::new())),
        };

        Ok(system)
    }
    // This function starts all the monitoring threads
    pub fn start(&mut self, mtx: std::sync::mpsc::Sender<u8>) {
        // Set up the signals for the Event thread
        // This needs to be done BEFORE any other child threads are spawned
        // so the rules for signal handling are inherited to all child threads
        self.threads.push(
            events::start_thread(
                Arc::clone(&self.config),
                mtx.clone(),
            )
        );

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
