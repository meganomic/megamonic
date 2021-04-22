use anyhow::{ bail, ensure, Context, Result };
use std::sync::{ Arc, Mutex, mpsc, atomic };
use std::io::Read;
use std::fmt::Write as fmtWrite;
use ahash::{ AHashMap, AHashSet };
use std::collections::hash_map::Entry;

mod process;
use super::{cpu, Config};

// Size of 'Processes.buffer_directories' used for getdents64()
const BUF_SIZE: usize = 1024 * 1024;

// CStr pointer to /proc
const PROC_PATH: *const u8 = b"/proc\0".as_ptr();

#[repr(C)]
struct LinuxDirent64T {
    /// 64-bit inode number.
    pub d_ino: u64,

    /// 64-bit offset to next structure.
    pub d_off: u64,

    /// Size of this dirent.
    pub d_reclen: u16,

    /// File type.
    pub d_type: u8,

    /// Filename (null-terminated).
    pub d_name: [u8; 4096],
}

//#[derive(Default)]
pub struct Processes {
    pub processes: AHashMap<u32, process::Process>,
    pub maxpidlen: usize,
    pub rebuild: bool,
    buffer: String,
    buffer_vector_dirs: Vec::<u8>,
    buffer_vector: Vec::<u8>,
    ignored: AHashSet<u32>,
}

impl Processes {
    pub fn update(&mut self, cpuinfo: &Arc<Mutex<cpu::Cpuinfo>>, config: &Arc<Config>) -> Result<()> {
        //let now = std::time::Instant::now();

        let all_processes = config.all.load(atomic::Ordering::Relaxed);

        // Trigger rebuild if 'show all processes' option is changed
        if all_processes != self.rebuild {
            self.rebuild = all_processes;
            self.processes.clear();
            self.ignored.clear();
        }

        // Open directory
        let fd: i32;
        unsafe {
            asm!("syscall",
                in("rax") 2, // SYS_OPEN
                in("rdi") PROC_PATH,
                in("rsi") 16, // O_DIRECTORY
                in("rdx") 0,
                out("rcx") _,
                out("r11") _,
                lateout("rax") fd,
            );
        }

        // Check if there's an error, panic if there is!
        ensure!(!fd.is_negative(), "SYS_OPEN return code: {}", fd);

        loop {
            // getdents64 system call
            let nread: i32;
            unsafe {
                asm!("syscall",
                    in("rax") 217, // SYS_GETDENTS64
                    in("rdi") fd,
                    in("rsi") self.buffer_vector_dirs.as_mut_ptr(),
                    in("rdx") BUF_SIZE,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") nread,
                );
            }

            // If there is an error panic
            ensure!(!nread.is_negative(), "SYS_GETDENTS64 return code: {}", nread);

            // If nread == 0 that means we have read all entries
            if nread == 0 {
                break;
            }

            let buf_box_ptr = self.buffer_vector_dirs.as_ptr() as usize;

            let mut bpos: usize = 0;
            while bpos < nread as usize {
                let d = (buf_box_ptr + bpos) as *mut LinuxDirent64T;
                let d_ref = unsafe { &(*d) };

                bpos += d_ref.d_reclen as usize;

                // If the entry isn't a directory, skip to the next one
                if d_ref.d_type != 4 {
                    continue;
                }

                // Saved so we can get the length of the PID later
                let pid_cstr = d_ref.d_name
                    .split(|v| *v == 0)
                    .next()
                    .context("Something is broken with the getdents64() code!")?;

                // Only directory names made up of numbers will pass
                if let Ok(pid) = btoi::btou(pid_cstr) {
                    if !self.ignored.contains(&pid) {
                        // Don't add it if we already have it
                        if let Entry::Vacant(process_entry) = self.processes.entry(pid) {
                            // If cmdline can't be opened it probably means that the process has terminated, skip it.
                            self.buffer.clear();
                            write!(&mut self.buffer, "/proc/{}/cmdline", pid).context("Error writing to buffer")?;
                            if let Ok(mut f) = std::fs::File::open(&self.buffer) {
                                self.buffer.clear();
                                f.read_to_string(&mut self.buffer).with_context(|| format!("/proc/{}/cmdline", pid))?;
                            } else {
                                continue;
                            };

                            // Limit the results to actual programs unless 'all-processes' is enabled
                            // pid == 1 is weird so make an extra check
                            if !self.buffer.is_empty() & (pid != 1) {
                                // Cancer code that is very hacky and don't work for all cases
                                // For instance, if a directory name has spaces or slashes in it, it breaks.
                                let mut split = self.buffer.split(&['\0', ' '][..]);
                                let executable = split.next()
                                    .context("Parsing error in /proc/[pid]/cmdline")?
                                    .rsplit('/')
                                    .next()
                                    .context("Parsing error in /proc/[pid]/cmdline")?
                                    .to_string();

                                let cmdline: String = split.intersperse(" ").collect();

                                process_entry.insert(
                                    process::Process::new(
                                        pid,
                                        executable,
                                        cmdline,
                                        false
                                    )
                                );
                            } else {
                                // If 'all-processes' is enabled add everything
                                if all_processes {
                                    // If stat can't be opened it means the process has terminated, skip it.
                                    self.buffer.clear();
                                    write!(&mut self.buffer, "/proc/{}/stat", pid).context("Error writing to buffer")?;
                                    let executable = if let Ok(mut f) = std::fs::File::open(&self.buffer) {
                                        self.buffer.clear();
                                        f.read_to_string(&mut self.buffer).with_context(|| format!("/proc/{}/stat", pid))?;
                                        self.buffer[
                                            self.buffer.find('(')
                                            .context("Can't parse /proc/[pid]/stat for exetuable name")?
                                            ..self.buffer.find(')')
                                            .context("Can't parse /proc/[pid]/stat for exetuable name")?
                                        ].to_string()
                                    } else {
                                        continue;
                                    };

                                    process_entry.insert(
                                        process::Process::new(
                                            pid,
                                            executable,
                                            String::new(),
                                            true
                                        )
                                    );
                                } else {
                                    // Otherwise add it to the ignore list
                                    self.ignored.insert(pid);
                                    continue;
                                }
                            }

                            // Save the length of the longest PID
                            let current_pid_len = pid_cstr.len();
                            if self.maxpidlen < current_pid_len {
                                self.maxpidlen = current_pid_len;
                            }
                        }
                    }
                }
            }
        }

        // Close file
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 3, // SYS_CLOSE
                in("rdi") fd,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        // Check if there's an error
        ensure!(ret == 0, "SYS_CLOSE return code: {}", ret);

        let (cpu_count, totald) = if let Ok(val) = cpuinfo.lock() {
            (val.cpu_count as f32, val.totald)
        } else {
            bail!("Cpuinfo lock is poisoned!");
        };

        let topmode = config.topmode.load(atomic::Ordering::Relaxed);
        let smaps = config.smaps.load(atomic::Ordering::Relaxed);

        let buf = &mut self.buffer_vector;

        let mut ret: Result<bool> = Ok(false);
        self.processes.retain(|_,process| {
            let res = process.update(buf, smaps);
            if let Ok(val) = res {
                if val {
                    if topmode {
                        if process.work > totald {
                            process.cpu_avg = 100.0 * cpu_count;
                        } else {
                            process.cpu_avg = (process.work as f32 / totald as f32) * 100.0 *  cpu_count;
                        }
                    } else if process.work > totald {
                        process.cpu_avg = 100.0;
                    } else {
                        process.cpu_avg = (process.work as f32 / totald as f32) * 100.0;
                    }

                    true
                } else {
                    false
                }
            } else {
                ret = res;
                false
            }
        });

        ensure!(ret.is_ok(), "process.update() returned with a failure state!\n{:?}\n", ret);

        //eprintln!("{}", now.elapsed().as_nanos());
        Ok(())
    }

    pub fn cpu_sort(&self) -> (usize, Vec::<&process::Process>) {
        let mut sorted = Vec::new();

        for val in self.processes.values() {
            // Multiply it so it can be sorted
            sorted.push(val);
        }

        // Sort by amount of Work, if equal sort by Total Work
        sorted.sort_by(|a, b| {
            b.work.cmp(&a.work)
                .then(b.total.cmp(&a.total))
        });

        (self.maxpidlen, sorted)
    }
}

impl Default for Processes {
    fn default() -> Self {
        Self {
            processes: AHashMap::default(),
            maxpidlen: 0,
            rebuild: false,
            buffer: String::new(),
            buffer_vector_dirs: Vec::with_capacity(BUF_SIZE),
            buffer_vector: Vec::with_capacity(1000),
            ignored: AHashSet::default(),
        }
    }
}

pub fn start_thread(internal: Arc<Mutex<Processes>>, cpuinfo: Arc<Mutex<cpu::Cpuinfo>>, config: Arc<Config>, tx: mpsc::Sender::<u8>, exit: Arc<(Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new().name("Processes".to_string()).spawn(move || {
        let (lock, cvar) = &*exit;
        'outer: loop {
            match internal.lock() {
                Ok(mut val) => {
                    if let Err(err) = val.update(&cpuinfo, &config) {
                        let mut errvec = error.lock().expect("Error lock couldn't be aquired!");

                        errvec.push(err);
                        let _ = tx.send(99);

                        break;
                    }
                },
                Err(_) => break,
            }

            match tx.send(8) {
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
    }).expect("Couldn't spawn Processes thread")
}
