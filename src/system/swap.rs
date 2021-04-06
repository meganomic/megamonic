use anyhow::{anyhow, Context, Result};
use std::sync::{Arc, Mutex, mpsc};
use std::io::prelude::*;

#[derive(Default)]
pub struct Swap {
    pub total: i64,
    pub used: i64,
    pub free: i64,
    buffer: String
}

impl Swap {
    pub fn update(&mut self) -> Result<()> {
        self.buffer.clear();
        std::fs::File::open("/proc/swaps")
            .context("Can't open /proc/swaps")?
            .read_to_string(&mut self.buffer)
            .context("Can't read /proc/swaps")?;

        self.total = 0;
        self.used = 0;

        for line in self.buffer.lines().skip(1) {
                let mut split = line.split_whitespace();
                self.total += split.nth(2).ok_or(anyhow!("Can't parse /proc/swap"))?.parse::<i64>()? * 1024;
                self.used += split.next().ok_or(anyhow!("Can't parse /proc/swap"))?.parse::<i64>()? * 1024;
        }
        self.free = self.total - self.used;

        Ok(())
    }
}

pub fn start_thread(internal: Arc<Mutex<Swap>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
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

            match tx.send(5) {
                Ok(_) => (),
                Err(_) => break,
            }

            //let (lock, cvar) = &*exit;
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
