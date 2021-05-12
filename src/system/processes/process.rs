use anyhow::{ ensure, Context, Result };
use std::ffi::CString;
use core::arch::x86_64::*;
use std::alloc;

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

    index: Vec::<usize>
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

        // This is to ensure that the vector is aligned to 32 bytes for my asm
        let layout = alloc::Layout::from_size_align(512, 32).expect("Can't create aligned layout!");

        let ptr = unsafe { alloc::alloc_zeroed(layout) };

        let buffer_stat = unsafe { Vec::from_raw_parts(ptr as *mut u8, 0, 512) };

        Ok(Self {
            pid,
            executable,
            cmdline,
            //stat_file,
            smaps_file: unsafe { CString::from_vec_unchecked(format!("/proc/{}/smaps_rollup", pid).into_bytes()) },
            not_executable,
            buffer_stat,
            buffer_smaps: Vec::<u8>::with_capacity(1024),
            pss: -1,
            stat_fd: fd,
            index: Vec::with_capacity(128),
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

    pub unsafe fn update_stat(&mut self) -> Result<()> {
        //let now = std::time::Instant::now();

        // Need to keep the old total so we have something to compare to
        let old_total = self.total;

        find_all(&mut self.index, self.buffer_stat.as_slice());



        let idx = if self.index.len() == 51 {
            self.index.as_slice()
        } else {
            let idx_adjust = self.index.len().checked_sub(51).context("Index is too small!")?;
            self.index.split_at(idx_adjust).1
        };

//         eprintln!("\npid: {}", self.pid);
//         unsafe { eprintln!("buffer: {}", std::str::from_utf8_unchecked(self.buffer_stat.as_slice())); }
//         eprintln!("index: {:?}", idx);
//         eprintln!("index_len: {:?}", index.len());

        self.utime = btoi::btou(&self.buffer_stat[*idx.get_unchecked(11)+1..*idx.get_unchecked(12)]).context("Can't convert utime to a number!").with_context(||format!("pid: {}", self.pid))?;

//             eprintln!("utime: {:?}", self.utime);

        self.stime = btoi::btou(&self.buffer_stat[*idx.get_unchecked(12)+1..*idx.get_unchecked(13)]).context("Can't convert stime to a number!").with_context(||format!("pid: {}", self.pid))?;

//             eprintln!("stime: {:?}", self.stime);

        self.cutime = btoi::btou(&self.buffer_stat[*idx.get_unchecked(13)+1..*idx.get_unchecked(14)]).context("Can't convert cutime to a number!").with_context(||format!("pid: {}", self.pid))?;

//             eprintln!("cutime: {:?}", self.cutime);

        self.cstime = btoi::btou(&self.buffer_stat[*idx.get_unchecked(14)+1..*idx.get_unchecked(15)]).context("Can't convert cstime to a number!").with_context(||format!("pid: {}", self.pid))?;

//             eprintln!("cstime: {:?}", self.cstime);

        self.rss = btoi::btou::<i64>(&self.buffer_stat[*idx.get_unchecked(22)+1..*idx.get_unchecked(23)]).context("Can't convert rss to a number!").with_context(||format!("pid: {}", self.pid))? * 4096;

//             eprintln!("rss: {:?}", self.rss);

        self.total = self.utime + self.stime + self.cutime + self.cstime;

        //eprintln!("total: {:?}, old_total: {:?}", self.total, old_total);

        // If old_total is 0 it means we don't have anything to compare to. So work is 0.
        //self.work = self.total.saturating_sub(old_total);
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

// Code stolen from https://github.com/BurntSushi/memchr and adapted to my needs
unsafe fn find_all(positions: &mut Vec::<usize>, haystack: &[u8]) {
    let start_ptr = haystack.as_ptr();
    let end_ptr = start_ptr.add(haystack.len());
    let mut ptr = start_ptr;

    // Set all bytes in register to 32, aka [space]
    let vn1 = _mm256_set1_epi8(32);

    // Load 32 bytes from buffer
    let a = _mm256_load_si256(ptr as *const __m256i);

    // Compare against vn1 and save the mask
    let mut mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(vn1, a)) as u32;

    // The index of the current item in positions
    let mut idx = 0;

    loop {
        // If mask is zero it means there are no matches
        if mask != 0 {
            // Saved index of match in buffer in positions
            *positions.get_unchecked_mut(idx) = ptr as usize + mask.trailing_zeros() as usize - start_ptr as usize;
            idx += 1;

            // Zero lowest set bit in the mask
            mask = _blsr_u32(mask);
        } else {
            ptr = ptr.add(32);

            // Load the next 32 bytes from buffer
            let a = _mm256_load_si256(ptr as *const __m256i);

            // Compare and save mask
            mask = _mm256_movemask_epi8(_mm256_cmpeq_epi8(vn1, a)) as u32;

            // If the previous load, loaded things outside our allocated memory, break
            if ptr.add(32) > end_ptr {
                break;
            }

        }
    }

    // Deal with any remaining bytes
    let mut byte_ptr = ptr as usize + mask.trailing_zeros() as usize;

    while byte_ptr < end_ptr as usize && mask != 0 {
        *positions.get_unchecked_mut(idx) = byte_ptr - start_ptr as usize;
        idx += 1;

        mask = _blsr_u32(mask);
        byte_ptr = ptr as usize + mask.trailing_zeros() as usize;
    }

    assert!(idx < positions.capacity(), "Index too large");

    positions.set_len(idx);
}
