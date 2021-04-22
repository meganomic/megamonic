use anyhow::{ ensure, Context, Result };
use std::ffi::CString;

// Open file 'path' and read it into 'buffer'
fn open_and_read(buffer: &mut Vec::<u8>, path: *const i8) -> Result<bool> {
    // Clear the buffer
    buffer.clear();

    // Open file
    let fd: i32;
    unsafe {
        asm!("syscall",
            in("rax") 2, // SYS_OPEN
            in("rdi") path,
            in("rsi") 0, // O_RDONLY
            in("rdx") 0,
            out("rcx") _,
            out("r11") _,
            lateout("rax") fd,
         );
    }

    // If there's an error it's 99.999% certain it's because the process has terminated
    if fd.is_negative() {
        return Ok(false);
    }

    // Read file into buffer
    let mut n_read = 0;
    let d_ptr_orig = buffer.as_mut_ptr() as usize;

    let mut ret: i32 = 1;

    let mut read_error = false;

    // Continue reading until there is nothing left
    while ret > 0 {
        let d_ptr = (d_ptr_orig + n_read as usize) as *const u8;

        unsafe {
            asm!("syscall",
                in("rax") 0, // SYS_READ
                in("rdi") fd,
                in("rsi") d_ptr,
                in("rdx") buffer.capacity() - n_read,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        if ret.is_negative() {
            read_error = true;
            break;
        }

        n_read += ret as usize;
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

    // Check if there's an error, panic if there is!
    ensure!(ret == 0, "SYS_CLOSE return code: {}", ret);

    ensure!(!read_error, "SYS_READ return code: {}", n_read);

    // Set buffer length to however many bytes was read
    unsafe {
        buffer.set_len(n_read as usize);
    }

    Ok(true)
}

#[derive(Default)]
pub struct Process {
    pub cpu_avg: f32,

    pub cmdline: String,
    pub executable: String,

    stat_file: CString,
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
}

impl Process {
    pub fn new(pid: u32, executable: String, cmdline: String, not_executable: bool) -> Self {
        Self {
            pid,
            executable,
            cmdline,
            stat_file: unsafe { CString::from_vec_unchecked(format!("/proc/{}/stat", pid).into_bytes()) },
            smaps_file: unsafe { CString::from_vec_unchecked(format!("/proc/{}/smaps_rollup", pid).into_bytes()) },
            not_executable,
            ..Default::default()
        }
    }

    pub fn update(&mut self, buffer: &mut Vec::<u8>, smaps: bool) -> Result<bool> {
        //let now = std::time::Instant::now();

        let ret = open_and_read(buffer, self.stat_file.as_ptr());

        // If ret is Ok(false) it means the stat file couldn't be opened
        // Which means the process has terminated
        // Returning false means the process will be removed from the list
        if let Ok(false) = ret {
            return ret;
        } else if ret.is_err() {
            return ret.context("open_and_read returned with a failure code!");
        }

        // Need to keep the old total so we have something to compare to
        let old_total = self.total;

                // Find position of first ')' character
        let pos = memchr::memchr(41, buffer.as_slice()).context("The stat_file is funky! It has no ')' character!")?;

        //let mut shoe = buffer.split(|v| *v == 41).last().unwrap();
        //eprintln!("SHOE: {:?}", shoe);


        // Split on ')' then on ' '
        let mut split = buffer.split_at(pos).1.split(|v| *v == 32);
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

        if smaps {
            if let Ok(true) = open_and_read(buffer, self.smaps_file.as_ptr()) {
                // Should maybe skip converting to str. I'll have to benchmark it
                let data = unsafe { std::str::from_utf8_unchecked(&buffer) };
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

        }

        // Returning true means the process will not be removed from the list
        Ok(true)

        //eprintln!("{}", now.elapsed().as_nanos());
    }
}
