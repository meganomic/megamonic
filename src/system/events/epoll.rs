#[repr(C)]
#[derive(Default)]
pub struct SignalfdSiginfo {
    pub ssi_signo: u32,    /* Signal number */
    ssi_errno: i32,    /* Error number (unused) */
    ssi_code: i32,     /* Signal code */
    ssi_pid: u32,      /* PID of sender */
    ssi_uid: u32,      /* Real UID of sender */
    ssi_fd: i32,       /* File descriptor (SIGIO) */
    ssi_tid: u32,      /* Kernel timer ID (POSIX timers) */
    ssi_band: u32,     /* Band event (SIGIO) */
    ssi_overrun: u32,  /* POSIX timer overrun count */
    ssi_trapno: u32,   /* Trap number that caused signal */
    ssi_status: i32,   /* Exit status or signal (SIGCHLD) */
    ssi_int: i32,      /* Integer sent by sigqueue(3) */
    ssi_ptr: u64,      /* Pointer sent by sigqueue(3) */
    ssi_utime: u64,    /* User CPU time consumed (SIGCHLD) */
    ssi_stime: u64,    /* System CPU time consumed
                    (SIGCHLD) */
    ssi_addr: u64,     /* Address that generated signal
                              (for hardware-generated signals) */
    ssi_addr_lsb: u16, /* Least significant bit of address
                              (SIGBUS; since Linux 2.6.37) */
    __pad2: u16,
    ssi_syscall: i32,
    ssi_call_addr: u64,
    ssi_arch: u32,
    __pad: [u8; 28],      /* Pad size to 128 bytes (allow for
                        additional fields in the future) */
}

// Custom sigaddset(), portability is for casuals
fn sigmask(sig: i32) -> u64 {
    1u64 << ((sig - 1) % 64)
}

fn sigword(sig: i32) -> u64 {
    ((sig - 1) / 64) as u64
}

fn sigaddset(set: &mut [u64; 8], sig: i32) {
    let mask = sigmask(sig);
    let word = sigword(sig);

    set[word as usize] |= mask;
}

pub struct SignalFD {
    pub fd: i32,
}

impl SignalFD {
    pub fn new() -> Self {
        // Custom verison
        let mut set: [u64; 8] = [0; 8];

        sigaddset(&mut set, 28); // SIGWINCH
        sigaddset(&mut set, 2); // SIGINT

        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 14, // SYS_SIGNALFD4
                in("rdi") 2, // SIG_SETMASK
                in("rsi") set.as_ptr(), //&sigset as *const libc::sigset_t, // 	sigset_t __user * nset
                in("rdx") 0, // sigset_t __user * oset
                in("r10") set.len(), // size_t sigsetsize
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(ret == 0);

        // Make a signalfd file
        let fd: i32;
        unsafe {
            asm!("syscall",
                in("rax") 289, // SYS_SIGNALFD4
                in("rdi") -1, // -1 == create a new signalfd
                in("rsi") set.as_ptr(), //&sigset as *const libc::sigset_t, // user_mask
                in("rdx") set.len(), // sizemask = u64 == 8, u32 == 4
                in("r10") 0, // flags
                out("rcx") _,
                out("r11") _,
                lateout("rax") fd,
            );
        }

        assert!(!fd.is_negative());

        /* libc version
        let mut sigset: libc::sigset_t = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };

        // Initialize sigset_t
        unsafe { libc::sigemptyset(&mut sigset) };

        // Set SIGWINCH to be handled by us
        unsafe { libc::sigaddset(&mut sigset, libc::SIGWINCH) };
        unsafe { libc::sigaddset(&mut sigset, libc::SIGINT) };


        // Disable default handling of SIGWINCH
        unsafe { libc::pthread_sigmask(libc::SIG_BLOCK, &sigset, 0 as *mut libc::sigset_t) };


        let fd = unsafe { libc::signalfd(-1, &sigset, 0) };
        */

        Self {
            fd
        }
    }

    pub fn close(&self) {
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

        assert!(!ret.is_negative());
    }
}

#[repr(C)]
pub union epoll_data_t {
    ptr: usize,
    pub fd: i32,
    uint32_t: u32,
    uint64_t: u64,
}

#[repr(C)]
pub struct EpollEvent {
    pub events: u32,      /* Epoll events */
    pub data: epoll_data_t        /* User data variable */
}

pub struct Epoll {
    fd: i32,
}

impl Epoll {
    pub fn new() -> Self {
        // Create epoll fd
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 291, // SYS_EPOLL_CREATE1
                in("rdi") 0, // Flags
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());

        Self {
            fd: ret
        }
    }

    pub fn add(&mut self, fd: i32) {
        let event = EpollEvent {
            events: 1, // EPOLLIN == 1
            data: epoll_data_t {
                fd
            }
        };

        // Add fd to the epoll interest list
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 233, // SYS_EPOLL_CTL
                in("rdi") self.fd, // int epfd
                in("rsi") 1, // EPOLL_CTL_ADD == 1
                in("rdx") fd, // FD to monitor
                in("r10") &event as *const EpollEvent, // struct epoll_event __user * event
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());
    }

    pub fn close(&self) {
        // Close epoll fd
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

        assert!(!ret.is_negative());
    }

    pub fn wait(&self) -> EpollEvent {
        let mut event = EpollEvent {
            events: 0,
            data: epoll_data_t {
                fd: 0
            }
        };

        // Wait for a epoll event
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 232, // SYS_EPOLL_WAIT
                in("rdi") self.fd, // epoll fd
                in("rsi") &mut event as *mut EpollEvent, // epoll event
                in("rdx") 1, // maxevents
                in("r10") -1, // timeout, -1 == forever
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret, // Numbers of events
            );
        }

        assert!(!ret.is_negative());

        event
    }
}
