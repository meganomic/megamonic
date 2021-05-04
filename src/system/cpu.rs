use anyhow::{ anyhow, Context, Result };
use std::sync::{ Arc, Mutex, mpsc };
use std::io::Read;

use super::{ read_fd, open_file };

#[derive(Default)]
struct Cpustats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

pub struct Cpuinfo {
    pub cpu_avg: f32,
    pub totald: u64,
    pub cpu_count: u8,
    pub governor: String,
    buffer: String,
    idle: u64,
    non_idle: u64,
    stats: Cpustats,
    cpu_fd: i32,
    gov_fd: i32,
}

impl Cpuinfo {
    pub fn new() -> Result<Self> {
        let cpu_fd = open_file("/proc/stat\0".as_ptr()).context("Can't open /proc/stat")?;

        let mut buffer = String::with_capacity(5000);

        unsafe {
            read_fd(cpu_fd, buffer.as_mut_vec()).context("Can't read /proc/stat")?;
        }

        let mut cpu_count = 0;
        for line in buffer.lines().skip(1) {
                if line.starts_with("cpu") {
                    cpu_count += 1;
                } else {
                    break;
                }
        }

        let gov_fd = open_file("/sys/devices/system/cpu/cpufreq/policy0/scaling_governor\0".as_ptr()).context("Can't open /sys/devices/system/cpu/cpufreq/policy0/scaling_governor")?;

        Ok(Self {
            cpu_avg: 0.0,
            totald: 0,
            cpu_count,
            governor: String::with_capacity(100),
            buffer,
            idle: 0,
            non_idle: 0,
            stats: Cpustats::default(),
            cpu_fd,
            gov_fd
        })
    }
    pub fn update(&mut self) -> Result<()> {
        unsafe {
            read_fd(self.gov_fd, self.governor.as_mut_vec()).context("Can't read /sys/devices/system/cpu/cpufreq/policy0/scaling_governor")?;
        }

        unsafe {
            read_fd(self.cpu_fd, self.buffer.as_mut_vec()).context("Can't read /proc/stat")?;
        }

        let line = self.buffer.lines().next().ok_or_else(||anyhow!("Can't parse /proc/stat"))?;

        // Save previous stats
        let prev_idle = self.stats.idle + self.stats.iowait;
        let prev_non_idle = self.stats.user
            + self.stats.nice
            + self.stats.system
            + self.stats.irq
            + self.stats.softirq
            + self.stats.steal;

        for (i, s) in line.split_ascii_whitespace().skip(1).enumerate() {
            match i {
                0 => self.stats.user = s.parse::<u64>().context("Can't parse 'user'")?,
                1 => self.stats.nice = s.parse::<u64>().context("Can't parse 'nice'")?,
                2 => self.stats.system = s.parse::<u64>().context("Can't parse 'system'")?,
                3 => self.stats.idle = s.parse::<u64>().context("Can't parse 'idle'")?,
                4 => self.stats.iowait = s.parse::<u64>().context("Can't parse 'iowait'")?,
                5 => self.stats.irq = s.parse::<u64>().context("Can't parse 'irq'")?,
                6 => self.stats.softirq = s.parse::<u64>().context("Can't parse 'softirq'")?,
                7 => self.stats.steal = s.parse::<u64>().context("Can't parse 'steal'")?,
                _ => break,
            }
        }

        self.idle = self.stats.idle + self.stats.iowait;
        self.non_idle = self.stats.user
            + self.stats.nice
            + self.stats.system
            + self.stats.irq
            + self.stats.softirq
            + self.stats.steal;

        // This is saved for use in process.rs to calculate cpu usage
        self.totald = self.idle + self.non_idle - prev_idle - prev_non_idle;

        self.cpu_avg = ((self.non_idle - prev_non_idle) as f32 / self.totald as f32) * 100.0;

        Ok(())
    }
}

impl Drop for Cpuinfo {
    fn drop(&mut self) {
        // Close any open FDs when it's dropped
        if self.cpu_fd != 0 {
            unsafe {
                asm!("syscall",
                    in("rax") 3, // SYS_CLOSE
                    in("rdi") self.cpu_fd,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") _,
                );
            }
        }

        if self.gov_fd != 0 {
            unsafe {
                asm!("syscall",
                    in("rax") 3, // SYS_CLOSE
                    in("rdi") self.gov_fd,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") _,
                );
            }
        }
    }
}

pub fn start_thread(internal: Arc<Mutex<Cpuinfo>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new().name("Cpu".to_string()).spawn(move || {
        let (lock, cvar) = &*exit;
        'outer: loop {
            match internal.lock() {
                Ok(mut val) => {
                    if let Err(err) = val.update() {
                        let mut errvec = error.lock().expect("Error lock couldn't be aquired!");

                        errvec.push(err);
                        let _ = tx.send(99);

                        break;
                    }
                },
                Err(_) => break
            }

            match tx.send(3) {
                Ok(_) => (),
                Err(_) => break
            }

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
    }).expect("Couldn't spawn Cpu thread")
}
