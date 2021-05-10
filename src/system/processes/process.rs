use anyhow::{ ensure, Context, Result };
use std::ffi::CString;

#[derive(Default)]
pub struct Process {
    pub cpu_avg: f32,

    pub cmdline: String,
    pub executable: String,

    //stat_file: CString,
    smaps_file: CString,

    // /proc/stat
    pub pid: u32,        // 1
    utime: u64,      // 14
    stime: u64,      // 15
    cutime: u64,     // 16
    cstime: u64,     // 17

    // /proc/smaps_rollup
    pub rss: i64,
    pub pss: i64,

    pub work: u64,
    pub total: u64,
    // /proc/task
    //pub tasks : std::collections::HashSet<u32>,

    pub not_executable: bool,

    pub stat_fd: i32,
    pub smaps_fd: i32,

    pub buffer_stat: Vec::<u8>,
    pub buffer_smaps: Vec::<u8>,
}

impl Process {
    pub fn new(pid: u32, executable: String, cmdline: String, not_executable: bool) -> Result<Self> {
        let stat_file = unsafe { CString::from_vec_unchecked(format!("/proc/{}/stat", pid).into_bytes()) };

        // Open file
        let fd: i32;
        unsafe {
            asm!("syscall",
                in("rax") 2, // SYS_OPEN
                in("rdi") stat_file.as_ptr(),
                in("rsi") 0, // O_RDONLY
                //in("rdx") 0, // This is the mode. It is not used in this case
                out("rcx") _,
                out("r11") _,
                lateout("rax") fd,
            );
        }

        ensure!(!fd.is_negative());

        Ok(Self {
            pid,
            executable,
            cmdline,
            //stat_file,
            smaps_file: unsafe { CString::from_vec_unchecked(format!("/proc/{}/smaps_rollup", pid).into_bytes()) },
            not_executable,
            buffer_stat: Vec::<u8>::with_capacity(500),
            buffer_smaps: Vec::<u8>::with_capacity(1000),
            pss: -1,
            stat_fd: fd,
            ..Default::default()
        })
    }

    pub fn get_smaps_fd(&mut self) -> i32 {
        // Only need to open it once
        if self.smaps_fd == 0 {
            // Open file
            let fd: i32;
            unsafe {
                asm!("syscall",
                    in("rax") 2, // SYS_OPEN
                    in("rdi") self.smaps_file.as_ptr(),
                    in("rsi") 0, // O_RDONLY
                    //in("rdx") 0, // This is the mode. It is not used in this case
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") fd,
                );
            }

            self.smaps_fd = fd;
        }

        self.smaps_fd
    }

    pub fn update_stat(&mut self) -> Result<()> {
        //let now = std::time::Instant::now();

        // Need to keep the old total so we have something to compare to
        let old_total = self.total;

        // Find position of first ')' character
        let pos = memchr::memchr(41, self.buffer_stat.as_slice()).context("The stat_file is funky! It has no ')' character!")?;

        //let mut shoe = buffer.split(|v| *v == 41).last().unwrap();
        //eprintln!("SHOE: {:?}", shoe);


        // Split on ')' then on ' '
        let mut split = self.buffer_stat.split_at(pos).1.split(|v| *v == 32);
        //eprintln!("{:?}", split.nth(1).unwrap());

            //eprintln!("KORV: {:?}", korv);
        self.utime = btoi::btou(split.nth(11).context("Can't parse 'utime' from /proc/[pid]/stat")?).context("Can't convert utime to a number!")?;

        //eprintln!("utime: {:?}", self.utime);

        self.stime = btoi::btou(split.next().context("Can't parse 'stime' from /proc/[pid]/stat")?).context("Can't convert stime to a number!")?;

            //eprintln!("stime: {:?}", self.stime);

        self.cutime = btoi::btou(split.next().context("Can't parse 'cutime' from /proc/[pid]/stat")?).context("Can't convert cutime to a number!")?;

            //eprintln!("cutime: {:?}", self.cutime);

        self.cstime = btoi::btou(split.next().context("Can't parse 'cstime' from /proc/[pid]/stat")?).context("Can't convert cstime to a number!")?;

            //eprintln!("cstime: {:?}", self.cstime);

        self.rss = btoi::btou::<i64>(split.nth(7).context("Can't parse 'rss' from /proc/[pid]/stat")?).context("Can't convert rss to a number!")? * 4096;

            //eprintln!("rss: {:?}", self.rss);

        self.total = self.utime + self.stime + self.cutime + self.cstime;

        // If old_total is 0 it means we don't have anything to compare to. So work is 0.
        self.work = if old_total != 0 {
            self.total - old_total
        } else {
            0
        };

        // Returning true means the process will not be removed from the list
        Ok(())

        //eprintln!("{}", now.elapsed().as_nanos());
    }

    pub fn update_smaps(&mut self) -> Result<()> {
         // If smaps_fd isn't above 0 it means we couldn't open/read it so set pss == -1
        if self.smaps_fd > 0 {
            // Should maybe skip converting to str. I'll have to benchmark it
            let data = unsafe { std::str::from_utf8_unchecked(&self.buffer_smaps) };
            self.pss = btoi::btou::<i64>(data.lines()
                .nth(2)
                .context("Can't parse 'pss' from /proc/[pid]/smaps_rollup, before whitespace")?
                .split_ascii_whitespace()
                .nth(1)
                .context("Can't parse 'pss' from /proc/[pid]/smaps_rollup, after whitespace")?.as_bytes())
                .context("Can't convert 'pss' to a number")?
                * 1024;
        } else {
            self.pss = -1;
        }

        Ok(())
    }

    pub fn disable_smaps(&mut self) {
        // If smaps is turned On and then Off we should close the file
        if self.smaps_fd > 0 {
            unsafe {
                asm!("syscall",
                    in("rax") 3, // SYS_CLOSE
                    in("rdi") self.smaps_fd,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") _,
                );
            }

            self.smaps_fd = 0;
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // Close any open FDs when it's dropped
        if self.stat_fd > 0 {
            unsafe {
                asm!("syscall",
                    in("rax") 3, // SYS_CLOSE
                    in("rdi") self.stat_fd,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") _,
                );
            }
        }

        if self.smaps_fd > 0 {
            unsafe {
                asm!("syscall",
                    in("rax") 3, // SYS_CLOSE
                    in("rdi") self.smaps_fd,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") _,
                );
            }
        }
    }
}
