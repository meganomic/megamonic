use anyhow::{ ensure, Result };
use std::ffi::CString;

#[inline(always)]
fn open_and_read(buffer: &mut Vec::<u8>, path: *const i8) -> Result<()> {
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
    ensure!(fd >= 0);

    // Read file into buffer
    let n_read: i32;
    unsafe {
        asm!("syscall",
            in("rax") 0, // SYS_READ
            in("rdi") fd,
            in("rsi") buffer.as_mut_ptr(),
            in("rdx") buffer.capacity(),
            out("rcx") _,
            out("r11") _,
            lateout("rax") n_read,
         );
    }

    // Check if there's an error
    assert!(n_read > 0, "SYS_READ return code: {}", n_read);

    // Set buffer length to however many bytes was read
    unsafe {
        buffer.set_len(n_read as usize);
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
    assert!(ret == 0, "SYS_CLOSE return code: {}", ret);

    Ok(())
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

    pub alive: bool,
}

impl Process {
    pub fn new(pid: u32, executable: String, cmdline: String, not_executable: bool) -> Self {
        Self {
            pid,
            executable,
            cmdline,
            stat_file: unsafe { CString::from_vec_unchecked(format!("/proc/{}/stat", pid).into_bytes()) },
            smaps_file: unsafe { CString::from_vec_unchecked(format!("/proc/{}/smaps_rollup", pid).into_bytes()) },
            alive: true,
            not_executable,
            ..Default::default()
        }
    }

    pub fn update(&mut self, buffer: &mut Vec::<u8>, smaps: bool) {
        //let now = std::time::Instant::now();

        if open_and_read(buffer, self.stat_file.as_ptr()).is_err() {
            self.alive = false;
            return;
        }

        let old_total = self.total;


        // Find position of first ')' character
        let pos = memchr::memchr(41, buffer.as_slice()).expect("The stat_file is funky! It has no ')' character!");

        //let mut shoe = buffer.split(|v| *v == 41).last().unwrap();
        //eprintln!("SHOE: {:?}", shoe);


        // Split on ')' then on ' '
        let mut split = buffer.split_at(pos).1.split(|v| *v == 32);
        //eprintln!("{:?}", split.nth(1).unwrap());

            //eprintln!("KORV: {:?}", korv);
        self.utime = btoi::btou(split.nth(11).expect("Can't parse 'utime' from /proc/[pid]/stat")).expect("Can't parse utime!");

        //eprintln!("utime: {:?}", self.utime);

        self.stime = btoi::btou(split.next().expect("Can't parse 'stime' from /proc/[pid]/stat")).expect("Can't parse stime!");

            //eprintln!("stime: {:?}", self.stime);

        self.cutime = btoi::btou(split.next().expect("Can't parse 'cutime' from /proc/[pid]/stat")).expect("Can't parse cutime!");

            //eprintln!("cutime: {:?}", self.cutime);

        self.cstime = btoi::btou(split.next().expect("Can't parse 'cstime' from /proc/[pid]/stat")).expect("Can't parse cstime!");

            //eprintln!("cstime: {:?}", self.cstime);

        self.rss = btoi::btou::<i64>(split.nth(7).expect("Can't parse 'rss' from /proc/[pid]/stat")).expect("Can't parse rss!") * 4096;

            //eprintln!("rss: {:?}", self.rss);

        self.total = self.utime + self.stime + self.cutime + self.cstime;

        // If old_total is 0 it means we don't have anything to compare to. So work is 0.
        self.work = if old_total != 0 {
            self.total - old_total
        } else {
            0
        };

        if smaps {
            if open_and_read(buffer, self.smaps_file.as_ptr()).is_ok() {
                let data = unsafe { std::str::from_utf8_unchecked(&buffer) };
                self.pss = btoi::btou::<i64>(data.lines()
                    .nth(2)
                    .expect("Can't parse 'pss' from /proc/[pid]/smaps_rollup")
                    .split_ascii_whitespace()
                    .nth(1)
                    .expect("Can't parse 'pss' from /proc/[pid]/smaps_rollup").as_bytes())
                    .expect("Can't convert 'pss' to a number")
                    * 1024;
            } else {
                self.pss = -1;
            }

        }

        //eprintln!("{}", now.elapsed().as_nanos());
    }
}
