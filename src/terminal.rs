#![allow(dead_code)]

const NCSS: usize = 32;

const TCGETS: u32 = 0x5401;
const TCSETS: u32 = 0x5402;
const TIOCGWINSZ: u32 = 0x5413;
const TIOCSTI: u32 = 0x5412;

/* c_iflag bits */
const IGNBRK: u32 =  0000001;
const BRKINT: u32 =  0000002;
const IGNPAR: u32 =  0000004;
const PARMRK: u32 =  0000010;
const INPCK: u32 =   0000020;
const ISTRIP: u32 =  0000040;
const INLCR: u32 =   0000100;
const IGNCR: u32 =   0000200;
const ICRNL: u32 =   0000400;
const IUCLC: u32 =   0001000;
const IXON: u32 =    0002000;
const IXANY: u32 =   0004000;
const IXOFF: u32 =   0010000;
const IMAXBEL: u32 = 0020000;
const IUTF8: u32 =   0040000;


/* c_lflag bits */
const ISIG: u32 =    0000001;
const ICANON: u32 =  0000002;
const XCASE: u32 =   0000004;
const ECHO: u32 =    0000010;
const ECHOE: u32 =   0000020;
const ECHOK: u32 =   0000040;
const ECHONL: u32 =  0000100;
const NOFLSH: u32 =  0000200;
const TOSTOP: u32 =  0000400;
const ECHOCTL: u32 = 0001000;
const ECHOPRT: u32 = 0002000;
const ECHOKE: u32 =  0004000;
const FLUSHO: u32 =  0010000;
const PENDIN: u32 =  0040000;
const IEXTEN: u32 =  0100000;
const EXTPROC: u32 = 0200000;

#[repr(C)]
struct Termios {
    c_iflag: u32,           /* input mode flags */
    c_oflag: u32,            /* output mode flags */
    c_cflag: u32,            /* control mode flags */
    c_lflag: u32,           /* local mode flags */
    c_line: u8,                        /* line discipline */
    c_cc: [u8; NCSS],            /* control characters */
    c_ispeed: u32,           /* input speed */
    c_ospeed: u32           /* output speed */
}

#[repr(C)]
#[derive(Default)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,   /* unused */
    ws_ypixel: u16,   /* unused */
}

pub struct Terminal {
    org_termios: Termios,
    fd: i32,
}

impl Terminal {
    // Open tty fd and save original settings
    pub fn new() -> Self {
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

        let termios = Termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_line: 0,
            c_cc: [0; NCSS],
            c_ispeed: 0,
            c_ospeed: 0
        };

        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 16, // SYS_IOCTL
                in("rdi") fd,
                in("rsi") TCGETS,
                in("rdx") &termios as *const Termios,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());

        Terminal {
            org_termios: termios,
            fd
        }
    }

    // Enable raw mode
    pub fn enable_raw_mode(&self) {
        let termios = Termios {
            c_iflag: self.org_termios.c_iflag & !(IGNBRK | BRKINT | PARMRK | ISTRIP
                    | INLCR | IGNCR | ICRNL | IXON),
            c_oflag: self.org_termios.c_oflag & !1,
            c_cflag: self.org_termios.c_cflag & !(60 | 400) | 60,
            c_lflag: self.org_termios.c_lflag & !(ECHO | ECHONL | ICANON | ISIG | IEXTEN),
            c_line: self.org_termios.c_line,
            c_cc: self.org_termios.c_cc,
            c_ispeed: self.org_termios.c_ispeed,
            c_ospeed: self.org_termios.c_ospeed
        };

        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 16, // SYS_IOCTL
                in("rdi") self.fd,
                in("rsi") TCSETS, // O_RDONLY
                in("rdx") &termios as *const Termios,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());
    }

    // Reset to original tty settings and close the fd
    pub fn disable_raw_mode(&self) {
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 16, // SYS_IOCTL
                in("rdi") self.fd,
                in("rsi") TCSETS, // O_RDONLY
                in("rdx") &self.org_termios as *const Termios,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());

        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 3, // SYS_CLOSE
                in("rdi") self.fd,
                //in("rsi") 0, // O_RDONLY
                //in("rdx") 0, // This is the mode. It is not used in this case
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());
    }

    // Send char to terminal input stream
    pub fn send_char(&self, c: &str) {
        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 16, // SYS_IOCTL
                in("rdi") self.fd,
                in("rsi") TIOCSTI, // O_RDONLY
                in("rdx") c.as_ptr(),
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());
    }

    // Get the size of the terminal
    pub fn gettermsize(&self) -> (u16, u16) {
        let mut winsize = Winsize::default();

        let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 16, // SYS_IOCTL
                in("rdi") self.fd,
                in("rsi") TIOCGWINSZ, // O_RDONLY
                in("rdx") &mut winsize as *mut Winsize,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());

        (winsize.ws_row, winsize.ws_col)
    }
}
