const NCSS: usize = 19;

const TCGETS: u32 =     0x5401;
const TCSETS: u32 =     0x5402;
const TIOCGWINSZ: u32 = 0x5413;
const TIOCSTI: u32 =    0x5412;

/* c_iflag bits */
const IGNBRK: u32 =  0o1;
const BRKINT: u32 =  0o2;
const PARMRK: u32 =  0o10;
const ISTRIP: u32 =  0o40;
const INLCR: u32 =   0o100;
const IGNCR: u32 =   0o200;
const ICRNL: u32 =   0o400;
const IXON: u32 =    0o2000;


/* c_lflag bits */
const ICANON: u32 =  0o2;
const ECHO: u32 =    0o10;
const ECHONL: u32 =  0o100;
const IEXTEN: u32 =  0o100000;

/* c_oflag bits */
const OPOST: u32 = 0o1;

/* c_cflag bits */
const CSIZE: u32 =  0o60;
const CS8: u32 =    0o60;
const PARENB: u32 = 0o400;

static mut TTYTERMIOS: Termios = Termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_line: 0,
            c_cc: [0; NCSS],
        };

static mut TTYFD: i32 = 0;


#[repr(C)]
struct Termios {
    c_iflag: u32,           /* input mode flags */
    c_oflag: u32,            /* output mode flags */
    c_cflag: u32,            /* control mode flags */
    c_lflag: u32,           /* local mode flags */
    c_line: u8,                        /* line discipline */
    c_cc: [u8; NCSS],            /* control characters */
}

#[repr(C)]
#[derive(Default)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,   /* unused */
    ws_ypixel: u16,   /* unused */
}

// Make sure statics are initialized
macro_rules! check_statics {
    () => {
        if unsafe { TTYFD == 0 } {
            init();
        }
    }
}

// Initialize the statics
fn init() {
    // Open tty fd
    let fd: i32;
    unsafe {
        asm!("syscall",
            in("rax") 2, // SYS_OPEN
            in("rdi") "/dev/tty\0".as_ptr(),
            in("rsi") 2, // O_RDWR
            //in("rdx") 0, // This is the mode. It is not used in this case
            out("rcx") _,
            out("r11") _,
            lateout("rax") fd,
        );
    }

    assert!(!fd.is_negative());

    // Save tty fd
    unsafe {
        TTYFD = fd;
    }

    // Save original tty settings to TTYTERMIOS
    let ret: i32;
    unsafe {
        asm!("syscall",
            in("rax") 16, // SYS_IOCTL
            in("rdi") fd,
            in("rsi") TCGETS,
            in("rdx") &TTYTERMIOS as *const Termios,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }

    assert!(!ret.is_negative());
}

// Enable raw mode
pub fn enable_raw_mode() {
    check_statics!();

    // Enable raw mode settings, but still send signals. c_flag | ISIG
    let termios = unsafe {
        Termios {
            c_iflag: TTYTERMIOS.c_iflag & !(IGNBRK | BRKINT | PARMRK | ISTRIP
                    | INLCR | IGNCR | ICRNL | IXON),
            c_oflag: TTYTERMIOS.c_oflag & !OPOST,
            c_lflag: TTYTERMIOS.c_lflag & !(ECHO | ECHONL | ICANON | IEXTEN),
            c_cflag: TTYTERMIOS.c_cflag & !(CSIZE | PARENB) | CS8,
            c_line: TTYTERMIOS.c_line,
            c_cc: TTYTERMIOS.c_cc,
        }
    };

    // Set tty with our new settings
    let ret: i32;
    unsafe {
        asm!("syscall",
            in("rax") 16, // SYS_IOCTL
            in("rdi") TTYFD,
            in("rsi") TCSETS,
            in("rdx") &termios as *const Termios,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }

    assert!(!ret.is_negative());
}

// Reset tty settings to original settings and close tty fd
pub fn disable_raw_mode() {
    check_statics!();

    // Set tty settings to our saved original values
    let ret: i32;
    unsafe {
        asm!("syscall",
            in("rax") 16, // SYS_IOCTL
            in("rdi") TTYFD,
            in("rsi") TCSETS,
            in("rdx") &TTYTERMIOS as *const Termios,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }

    assert!(!ret.is_negative());

    // Close the tty fd
    let ret: i32;
    unsafe {
        asm!("syscall",
            in("rax") 3, // SYS_CLOSE
            in("rdi") TTYFD,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }

    assert!(!ret.is_negative());
}

// Send char to terminal input stream, used to exit the Event thread
pub fn send_char(c: &str) {
    check_statics!();

    let ret: i32;
    unsafe {
        asm!("syscall",
            in("rax") 16, // SYS_IOCTL
            in("rdi") TTYFD,
            in("rsi") TIOCSTI,
            in("rdx") c.as_ptr(),
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }

    assert!(!ret.is_negative());
}

// Get the size of the terminal
pub fn gettermsize() -> (u16, u16) {
    check_statics!();

    let mut winsize = Winsize::default();

    let ret: i32;
    unsafe {
        asm!("syscall",
            in("rax") 16, // SYS_IOCTL
            in("rdi") TTYFD,
            in("rsi") TIOCGWINSZ,
            in("rdx") &mut winsize as *mut Winsize,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
        );
    }

    assert!(!ret.is_negative());

    (winsize.ws_col, winsize.ws_row)
}
