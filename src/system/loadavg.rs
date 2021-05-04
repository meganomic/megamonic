use anyhow::{ Context, Result, bail };
use std::sync::{ Arc, Mutex, mpsc };
use std::io::Read;

use super::read_fd;

pub struct Loadavg {
    pub min1: String,
    pub min5: String,
    pub min15: String,
    buffer: String,
    fd: i32,
}

impl Loadavg {
    pub fn new() -> Result<Self> {
        // Open file
        let fd: i32;
        unsafe {
            asm!("syscall",
                in("rax") 2, // SYS_OPEN
                in("rdi") "/proc/loadavg\0".as_ptr(),
                in("rsi") 0, // O_RDONLY
                //in("rdx") 0, // This is the mode. It is not used in this case
                out("rcx") _,
                out("r11") _,
                lateout("rax") fd,
            );
        }

        // If there's an error it's 99.999% certain it's because the process has terminated
        if fd.is_negative() {
            bail!("Can't open /proc/loadavg");
        }

        Ok(Self {
            min1: String::new(),
            min5: String::new(),
            min15: String::new(),
            buffer: String::with_capacity(100),
            fd
        })
    }

    pub fn update(&mut self) -> Result<()> {
        unsafe {
            read_fd(self.fd, self.buffer.as_mut_vec()).context("Can't read /proc/loadavg")?;
        }

        self.min1.clear();
        self.min5.clear();
        self.min15.clear();

        let mut split = self.buffer.split_ascii_whitespace();

        self.min1.push_str(split.next().context("Can't parse /proc/loadavg: 1")?);
        self.min5.push_str(split.next().context("Can't parse /proc/loadavg: 2")?);
        self.min15.push_str(split.next().context("Can't parse /proc/loadavg: 3")?);

        Ok(())
    }
}

impl Drop for Loadavg {
    fn drop(&mut self) {
        // Close file
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 3, // SYS_CLOSE
                in("rdi") self.fd,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        // Check if there's an error
        assert!(ret == 0, "SYS_CLOSE return code: {}", ret);
    }
}

pub fn start_thread(internal: Arc<Mutex<Loadavg>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new().name("Load Average".to_string()).spawn(move || {
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

            match tx.send(2) {
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
    }).expect("Couldn't spawn Load Average thread")

}
