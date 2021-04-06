use anyhow::{anyhow, Context, Result};
use std::sync::{Arc, Mutex, mpsc};
use std::io::prelude::*;

#[derive(Default)]
pub struct Loadavg {
    pub min1: String,
    pub min5: String,
    pub min15: String,
    buffer: String
}

impl Loadavg {
    pub fn update(&mut self) -> Result<()> {
        self.buffer.clear();
        std::fs::File::open("/proc/loadavg")
            .context("Can't open /proc/loadavg")?
            .read_to_string(&mut self.buffer)
            .context("Can't read /proc/loadavg")?;

        self.min1.clear();
        self.min5.clear();
        self.min15.clear();

        let mut split = self.buffer.split_whitespace();

        self.min1.push_str(split.next().ok_or(anyhow!("Can't parse /proc/loadavg"))?);
        self.min5.push_str(split.next().ok_or(anyhow!("Can't parse /proc/loadavg"))?);
        self.min15.push_str(split.next().ok_or(anyhow!("Can't parse /proc/loadavg"))?);

        Ok(())
    }
}

pub fn start_thread(internal: Arc<Mutex<Loadavg>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
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
            };
            match tx.send(2) {
                Ok(_) => (),
                Err(_) => break,
            };

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
