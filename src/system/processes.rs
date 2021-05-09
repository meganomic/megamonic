use anyhow::{ bail, ensure, Context, Result };
use std::sync::{ Arc, Mutex, mpsc, atomic };
use std::io::Read;
use std::fmt::Write as fmtWrite;
use ahash::{ AHashMap, AHashSet };
use std::collections::hash_map::Entry;

pub mod process;
use super::{ cpu, Config, uring::{ Uring, UringError, IOOPS::* } };

// Size of 'Processes.buffer_directories' used for getdents64()
const BUF_SIZE: usize = 1024 * 1024;

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
    // List of all processes
    pub processes: AHashMap<u32, process::Process>,

    // The length of the longest PID in the list
    pub maxpidlen: usize,

    // Used to clear self.processes if modes are changed
    pub rebuild: bool,

    fd: i32,

    // Buffers to avoid allocations
    buffer: String,
    buffer_vector_dirs: Vec::<u8>,

    sorted: Vec::<usize>,

    // If all_processes isn't enabled, ignore the PIDs in this list
    ignored: AHashSet<u32>,

    // io_uring
    uring: Uring,
}

impl Processes {
    pub fn new() -> Result<Self> {
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 2, // SYS_OPEN
                in("rdi") "/proc\0".as_ptr(),
                in("rsi") 16, // O_DIRECTORY
                //in("rdx") 0, // This is the mode. It is not used in this case
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        // Check if there's an error
        ensure!(!ret.is_negative(), "SYS_OPEN return code: {}", ret);

        Ok(Self {
            processes: AHashMap::default(),
            maxpidlen: 0,
            rebuild: false,
            fd: ret,
            buffer: String::new(),
            buffer_vector_dirs: Vec::with_capacity(BUF_SIZE),
            ignored: AHashSet::default(),
            sorted: Vec::new(),

            // Create io_uring with default size (500)
            uring: Uring::new(0).expect("Can't make a io_uring"),
        })
    }

    pub fn update(&mut self, cpuinfo: &Arc<Mutex<cpu::Cpuinfo>>, config: &Arc<Config>) -> Result<()> {
        //let now = std::time::Instant::now();

        let all_processes = config.all.load(atomic::Ordering::Relaxed);

        // Trigger rebuild if 'show all processes' option is changed
        if all_processes != self.rebuild {
            self.rebuild = all_processes;
            self.processes.clear();
            self.ignored.clear();
        }

        // Seek to beginning of directory fd
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 8, // SYS_SEEK
                in("rdi") self.fd,
                in("rsi") 0, // offset
                in("rdx") 0, // SET_SEEK
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        ensure!(!ret.is_negative(), "SYS_SEEK return code: {}", ret);

        loop {
            // getdents64 system call
            let nread: i32;
            unsafe {
                asm!("syscall",
                    in("rax") 217, // SYS_GETDENTS64
                    in("rdi") self.fd,
                    in("rsi") self.buffer_vector_dirs.as_mut_ptr(),
                    in("rdx") BUF_SIZE,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") nread,
                );
            }

            // Make sure there is no error
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

                // Set position to next LinuxDirent64T entry
                bpos += d_ref.d_reclen as usize;

                // If the entry isn't a directory, skip to the next one
                if d_ref.d_type != 4 {
                    continue;
                }

                // Saved so we can get the length of the PID later
                let pid_cstr = d_ref.d_name
                    .split(|v| *v == 0)
                    .next()
                    .context("Can't parse d_ref.d_name!")?;

                // Only directory names made up of numbers will pass
                if let Ok(pid) = btoi::btou(pid_cstr) {
                    if !self.ignored.contains(&pid) {
                        // Don't add it if we already have it
                        if let Entry::Vacant(process_entry) = self.processes.entry(pid) {
                            // Avoiding allocations is cool kids!
                            self.buffer.clear();
                            let _ = write!(&mut self.buffer, "/proc/{}/cmdline", pid);

                            // If cmdline can't be opened it probably means that the process has terminated, skip it.
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

                                // If it's not Ok() then the stat_file couldn't be opened
                                // which means the process has terminated
                                if let Ok(process) =
                                    process::Process::new(
                                        pid,
                                        executable,
                                        cmdline,
                                        false
                                    )
                                {
                                    process_entry.insert(process);
                                }
                            } else {
                                // If 'all-processes' is enabled add everything
                                if all_processes {
                                    // If stat can't be opened it means the process has terminated, skip it.
                                    self.buffer.clear();
                                    let _ = write!(&mut self.buffer, "/proc/{}/stat", pid);

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

                                    // If it's not Ok() then the stat_file couldn't be opened
                                    // which means the process has terminated
                                    if let Ok(process) =
                                        process::Process::new(
                                            pid,
                                            executable,
                                            String::new(),
                                            true
                                        )
                                    {
                                        process_entry.insert(process);
                                    }
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



        let (cpu_count, totald) = if let Ok(val) = cpuinfo.lock() {
            (val.cpu_count as f32, val.totald)
        } else {
            bail!("Cpuinfo lock is poisoned!");
        };

        let topmode = config.topmode.load(atomic::Ordering::Relaxed);
        let csmaps = config.smaps.load(atomic::Ordering::Relaxed);

        if !csmaps {
            for process in self.processes.values_mut() {
                process.disable_smaps();
            }
        }

        //let now = std::time::Instant::now();

        // Reset counting variables
        self.uring.reset();

        // If the io_uring ringbuffer is too small make a new instance with a bigger one
        if csmaps && (self.processes.len() * 2) > self.uring.entries {
            self.uring = Uring::new((self.processes.len() * 2) + 100)?;
        } else if self.processes.len() > self.uring.entries {
            self.uring = Uring::new(self.processes.len() + 100)?;
        }

        for data in self.processes.values_mut() {
            self.uring.add_to_queue((data.pid, 0), &mut data.buffer_stat, data.stat_fd, IORING_OP_READ);
            if csmaps {
                let fd = data.get_smaps_fd();
                if !fd.is_negative() {
                    self.uring.add_to_queue((data.pid, 1), &mut data.buffer_smaps, fd, IORING_OP_READ);
                }
            }
        }

        let ret = self.uring.submit_all().expect("Can't submit io_uring jobs to the kernel!");

        loop {
            match self.uring.spin_next() {
                Ok((res, pid, smaps)) => {
                    if let Entry::Occupied(mut entry) = self.processes.entry(pid as u32) {
                        if smaps == 1 {
                            if !res.is_negative() {
                                let process = entry.into_mut();
                                unsafe {
                                    process.buffer_smaps.set_len(res as usize);
                                }
                                process.update_smaps()?;
                            }
                            continue;

                        }
                        if res.is_negative() {
                            entry.remove_entry();
                        } else {
                            let process = entry.get_mut();
                            unsafe {
                                process.buffer_stat.set_len(res as usize);
                            }

                            let res = process.update_stat();

                            if let Ok(val) = res {
                                // Calculate CPU % usage
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

                            } else {
                                entry.remove_entry();
                                res.context("process.update() returned with a failure state!")?;
                            }
                        }
                    }
                },
                Err(UringError::JobComplete) => {
                    break;
                },
                _ => {
                    bail!("I dunno");
                },
            }
        }


        //eprintln!("{}", now.elapsed().as_nanos());
        Ok(())
    }

    // Make a list of all processes and sort by amount of Work done
    // For use with displaying it in the terminal
    pub fn cpu_sort(&mut self) -> (usize, &Vec::<usize>) {
        // This pointer cancer is because I don't want to allocate
        // a new vector every single time this function is called
        self.sorted.clear();

        for val in self.processes.values() {
            self.sorted.push(val as *const process::Process as usize);
        }

        // Sort by amount of Work, if equal sort by Total Work
        self.sorted.sort_by(|a, b| {
            let a = unsafe { &*(*a as *const process::Process) };
            let b = unsafe { &*(*b as *const process::Process) };
            b.work.cmp(&a.work)
                .then(b.total.cmp(&a.total))
        });

        (self.maxpidlen, &self.sorted)
    }
}

impl Drop for Processes {
    fn drop(&mut self) {
        // Close file
        if self.fd != 0 {
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
