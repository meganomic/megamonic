#[repr(C)]
#[derive(Default)]
struct SignalfdSiginfo {
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

pub struct SignalFD {
    pub fd: i32,
}

impl SignalFD {
    pub fn new() -> Self {
        // sigset_t is just a u64 on 64bit linux.
        let mut set: u64 = 0;

        set |= 1u64 << 27; // SIGWINCH == 28 - 1 = 27
        set |= 1u64 << 1; // SIGINT == 2 - 1 = 1

        // Block the signals we want to handle ourselves
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 14, // SYS_RT_SIGPROCMASK
                in("rdi") 2, // SIG_SETMASK
                in("rsi") &set as *const u64, // sigset_t __user * nset
                in("rdx") 0, // sigset_t __user * oset
                in("r10") 8, // size_t sigsetsize aka how many bytes is the set
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
                in("rax") 282, // SYS_SIGNALFD4
                in("rdi") -1, // -1 == create a new signalfd
                in("rsi") &set as *const u64, //&sigset as *const libc::sigset_t, // user_mask
                in("rdx") 8, // sizemask aka how many bytes is the set
                //in("r10") 0, // flags
                out("rcx") _,
                out("r11") _,
                lateout("rax") fd,
            );
        }

        assert!(!fd.is_negative());

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

    pub fn read(&self) -> u32 {
        // Buffer to hold the signal data
        let mut data = SignalfdSiginfo::default();

        // Read signal info from signalfd
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 0, // SYS_READ
                in("rdi") self.fd,
                in("rsi") &mut data as *mut SignalfdSiginfo,
                in("rdx") 128,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());

        data.ssi_signo
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
