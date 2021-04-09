use anyhow::{anyhow, Context, Result};
use std::sync::{Arc, Mutex, mpsc};
use std::io::prelude::*;

#[derive(Default, Clone)]
pub struct Memory {
    pub total: i64,
    pub free: i64,
    pub used: i64,
    buffer: String
}

impl Memory {
    pub fn update(&mut self) -> Result<()> {
        self.buffer.clear();
        std::fs::File::open("/proc/meminfo")
            .context("Can't open /proc/meminfo")?
            .read_to_string(&mut self.buffer)
            .context("Can't read /proc/meminfo")?;

        let mut lines = self.buffer.lines();

        self.total = lines.next()
            .ok_or_else(||anyhow!("Can't parse /proc/meminfo: 1"))?
            .split_whitespace()
            .nth(1)
            .ok_or_else(||anyhow!("Can't parse /proc/meminfo: 2"))?
            .parse::<i64>()
            .context("Can't parse /proc/meminfo: 3")?
            * 1024;

        self.free = lines.nth(1)
            .ok_or_else(||anyhow!("Can't parse /proc/meminfo: 1"))?
            .split_whitespace()
            .nth(1)
            .ok_or_else(||anyhow!("Can't parse /proc/meminfo: 2"))?
            .parse::<i64>()
            .context("Can't parse /proc/meminfo: 3")?
            * 1024;

        self.used = self.total - self.free;

        Ok(())
    }
}

pub fn start_thread(internal: Arc<Mutex<Memory>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let (lock, cvar) = &*exit;
        'outer: loop {
            match internal.lock() {
                Ok(mut val) => {
                    if let Err(err) = val.update() {
                        let mut errvec = error.lock().expect("Error lock couldn't be aquired!");
                        errvec.push(err);

                        match tx.send(99) {
                            Ok(_) => (),
                            Err(_) => break,
                        }

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
    })
}
