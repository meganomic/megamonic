use std::arch::asm;

use anyhow::{ Context, Result };
use std::sync::{ Arc, Mutex, mpsc };

use super::{ read_fd, open_file };

#[derive(Default)]
pub struct Bandwidth {
    pub recv: u64,
    pub sent: u64,
    pub total_recv: u64,
    pub total_sent: u64,
}

pub struct Network {
    pub stats: std::collections::BTreeMap<String, Bandwidth>,
    buffer: String,
    fd: i32
}

impl Network {
    pub fn new() -> Result<Self> {
        let fd = open_file("/proc/net/dev\0".as_ptr()).context("Can't open /proc/net/dev")?;

        Ok(Self {
            stats: std::collections::BTreeMap::new(),
            buffer: String::with_capacity(1500),
            fd
        })
    }

    pub fn update(&mut self) -> Result<()> {
        unsafe {
            read_fd(self.fd, self.buffer.as_mut_vec()).context("Can't read /proc/net/dev")?;
        }

        for line in self.buffer.lines().skip(2) {
            let mut bandwidth = Bandwidth::default();

            let mut split = line.split_ascii_whitespace();

            let name = split.next().context("Can't parse name from /proc/net/dev")?.to_string();

            bandwidth.total_recv = split.next()
                .context("Can't parse total_recv from /proc/net/dev")?
                .parse::<u64>()
                .context("Can't parse total_recv from /proc/net/dev")?;

            bandwidth.total_sent = split.nth(7)
                .context("Can't parse total_sent from /proc/net/dev")?
                .parse::<u64>()
                .context("Can't parse total_sent from /proc/net/dev")?;

            // If it hasn't sent and recieved anything it's probably off so don't add it.
            if bandwidth.total_recv + bandwidth.total_sent != 0 {
                self.stats.entry(name)
                    .and_modify(|bw|
                        {
                            bw.recv = bandwidth.total_recv - bw.total_recv;
                            bw.sent = bandwidth.total_sent - bw.total_sent;
                            bw.total_recv = bandwidth.total_recv;
                            bw.total_sent = bandwidth.total_sent;
                        }
                    )
                    .or_insert(bandwidth);
            }
        }

        Ok(())
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        // Close file
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

pub fn start_thread(internal: Arc<Mutex<Network>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new().name("Network".to_string()).spawn(move || {
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

            match tx.send(7) {
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
    }).expect("Couldn't spawn Network thread")
}
