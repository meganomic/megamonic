use std::arch::asm;

use anyhow::{ Context, Result };
use std::sync::{ Arc, Mutex, mpsc, atomic, atomic::Ordering };

use super::{ read_fd, open_file };

pub struct Memory {
    // pub mem_total: u64,
    pub mem_total: atomic::AtomicU64,
    pub mem_free: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_free: u64,
    pub swap_used: u64,
    buffer: String,
    fd: i32
}

impl Memory {
    pub fn new() -> Result<Self> {
        let fd = open_file("/proc/meminfo\0".as_ptr()).context("Can't open /proc/meminfo")?;

        Ok(Self {
            mem_total: atomic::AtomicU64::new(0),
            mem_free: 0,
            mem_used: 0,
            swap_total: 0,
            swap_free: 0,
            swap_used: 0,
            buffer: String::with_capacity(2000),
            fd
        })
    }
    pub fn update(&mut self) -> Result<()> {
        /*self.buffer.clear();
        std::fs::File::open("/proc/meminfo")
            .context("Can't open /proc/meminfo")?
            .read_to_string(&mut self.buffer)
            .context("Can't read /proc/meminfo")?;*/

        unsafe {
            read_fd(self.fd, self.buffer.as_mut_vec()).context("Can't read /proc/meminfo")?;
        }

        let mut lines = self.buffer.lines();

        self.mem_total.store(lines.next()
            .context("Can't parse mem_total /proc/meminfo: 1")?
            .split_ascii_whitespace()
            .nth(1)
            .context("Can't parse mem_total /proc/meminfo: 2")?
            .parse::<u64>()
            .context("Can't parse mem_total /proc/meminfo: 3")?
            * 1024, Ordering::Relaxed);

        self.mem_free = lines.nth(1)
            .context("Can't parse mem_free /proc/meminfo: 1")?
            .split_ascii_whitespace()
            .nth(1)
            .context("Can't parse mem_free /proc/meminfo: 2")?
            .parse::<u64>()
            .context("Can't parse mem_free /proc/meminfo: 3")?
            * 1024;

        self.swap_total = lines.nth(11)
            .context("Can't parse swap_total /proc/meminfo: 1")?
            .split_ascii_whitespace()
            .nth(1)
            .context("Can't parse swap_total /proc/meminfo: 2")?
            .parse::<u64>()
            .context("Can't parse swap_total /proc/meminfo: 3")?
            * 1024;

        self.swap_free = lines.next()
            .expect("Can't parse swap_free /proc/meminfo: 1")
            .split_ascii_whitespace()
            .nth(1)
            .context("Can't parse swap_free /proc/meminfo: 2")?
            .parse::<u64>()
            .context("Can't parse swap_free /proc/meminfo: 3")?
            * 1024;

        self.mem_used = self.mem_total.load(Ordering::Relaxed) - self.mem_free;

        self.swap_used = self.swap_total - self.swap_free;

        Ok(())
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        // Close any open FDs when it's dropped
        if self.fd > 0 {
            unsafe {
                asm!("syscall",
                    in("rax") 3, // SYS_CLOSE
                    in("rdi") self.fd,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") _,
                );
            }
        }
    }
}

pub fn start_thread(internal: Arc<Mutex<Memory>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new().name("Memory".to_string()).spawn(move || {
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
                Err(_) => break,
            }

            match tx.send(4) {
                Ok(_) => (),
                Err(_) => break,
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
    }).expect("Couldn't spawn Memory thread")
}
