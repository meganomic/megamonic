use crossterm::terminal;
use std::io::Write as ioWrite;
use std::fmt::Write as fmtWrite;
use anyhow::{ ensure, Context, Result };

mod time;
use time::Time;

mod memory;
use memory::Memory;

mod swap;
use swap::Swap;

mod loadavg;
use loadavg::Loadavg;

mod overview;
use overview::Overview;

mod hostinfo;
use hostinfo::Hostinfo;

mod processes;
use processes::Processes;

mod network;
use network::Network;

mod sensors;
use self::sensors::Sensors;

mod gpu;
use gpu::Gpu;


// These are for used with the conversion functions
static UNITS: [&str; 9] = ["b", "Kb", "Mb", "Gb", "Tb", "Pb", "Eb", "Zb", "Yb"];

// A kilobyte is 1024 bytes. Fight me!
const DELIMITER: f64 = 1024_f64;

// ln(1024)
const DELIMITER_LN: f64 = 6.93147180559945308431224475498311221599578857421875;


// Write to stdout without error checking
macro_rules! write_to_stdout {
    ($data:expr) => {
        // Write to stdout
        let mut ret: i32;
        let mut written = 0;

        let d_ptr_orig = $data.as_ptr() as usize;
        let length = $data.len();

        while written < length {
            let d_ptr = (d_ptr_orig + written) as *const u8;

            unsafe {
                asm!("syscall",
                    in("rax") 1, // SYS_WRITE
                    in("rdi") 1,
                    in("rsi") d_ptr,
                    in("rdx") length - written,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") ret,
                );
            }

            // Check if there's an error
            ensure!(!ret.is_negative(), "SYS_WRITE return code: {}", ret);

            written += ret as usize;

        }

        // Make sure it actually works correctly
        debug_assert!(written == length, "written: {}, lenght: {}", written, length);
    };
}

// Customized version of https://github.com/sfackler/rust-log-panics
fn custom_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace_env = if std::env::var_os("RUST_BACKTRACE").is_some() {
            1
        } else {
            0
        };

        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>");

        // Reset the terminal
        print!("\x1b[2J\x1b[?1049l\x1b[?25h\x1b[?7h");

        let _ = terminal::disable_raw_mode();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        println!("thread '{}' panicked at '{}', {}", name, msg, info.location().unwrap());

        static FIRST_PANIC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

        match backtrace_env {
            0 => {
                if FIRST_PANIC.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    println!("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n");
                }
            }
            _ => {
                println!("\n{:?}", backtrace::Backtrace::new());
            },
        }

    }));
}

#[derive(Default)]
pub struct XY {
    pub x: u16,
    pub y: u16
}

pub struct Ui <'ui> {
    pub terminal_size: XY,

    paused: bool,

    buffer: Vec::<u8>,
    system: &'ui super::system::System,

    time: Time <'ui>,
    overview: Overview <'ui>,
    memory: Memory <'ui>,
    swap: Swap <'ui>,
    loadavg: Loadavg <'ui>,
    hostinfo: Hostinfo <'ui>,
    processes: Processes <'ui>,
    network: Network <'ui>,
    sensors: Sensors <'ui>,
    gpu: Gpu <'ui>,
}

impl <'ui> Ui <'ui> {
    pub fn new(system: &'ui super::system::System) -> Result<Self> {
        let (tsizex, tsizey) = terminal::size().context("Can't get terminal size")?;

        let mut ui = Self {
            paused: false,
            buffer: Vec::new(),
            system,
            terminal_size: XY { x: tsizex, y: tsizey },

            time: Time::new(system, XY { x: 1, y: tsizey }),
            overview: Overview::new(system, XY { x: 1, y: 1 }),
            memory: Memory::new(system, XY { x: 18, y: 1 }),
            swap: Swap::new(system, XY { x: 38, y: 1 }),
            loadavg: Loadavg::new(system, XY { x: 58, y: 1 }),
            hostinfo: Hostinfo::new(system, XY { x: 0, y: tsizey }),
            processes: Processes::new(system, XY { x: 26, y: 5 }),
            network: Network::new(system, XY { x: 0, y: 5 }),
            sensors: Sensors::new(system, XY { x: 0, y: 11 }),
            gpu: Gpu::new(system, XY { x: 0, y: 21 }),
        };

        ui.init().context("Error occured while initializting UI")?;

        if let Err(err) = ui.rebuild().context("Error occured while building UI") {
            ui.exit()?;
            return Err(err);
        }

        Ok(ui)
    }

    fn init(&mut self) -> Result<()> {
        // Disable all hotkeys and stuff.
        terminal::enable_raw_mode()?;

        // Setup the terminal screen
        write_to_stdout!("\x1b[?1049h\x1b[2J\x1b[?25l"); //\x1b[?7l");

        // Initialize custom panic hook
        custom_panic_hook();

        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        // Reset the terminal screen
        write_to_stdout!("\x1b[2J\x1b[?1049l\x1b[?25h"); //\x1b[?7h");

        terminal::disable_raw_mode()?;

        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        if self.paused {
            self.paused = false;
        } else {
            self.paused = true;
        }
    }

    pub fn rebuild_cache(&mut self) -> Result<()> {
        self.time.pos.y = self.terminal_size.y;
        self.time.rebuild_cache();

        self.hostinfo.rebuild_cache(&self.terminal_size);

        self.loadavg.rebuild_cache();
        self.overview.rebuild_cache();
        self.memory.rebuild_cache();
        self.swap.rebuild_cache();
        self.processes.rebuild_cache(&self.terminal_size);

        // The following objects rely on the position of the previous ones
        // So don't do anything silly.
        self.network.rebuild_cache()?;

        self.sensors.pos.y = self.network.size.y + self.network.pos.y;
        self.sensors.rebuild_cache()?;

        self.gpu.pos.y = self.sensors.pos.y + self.sensors.size.y;
        self.gpu.rebuild_cache();

        Ok(())
    }

    pub fn update (&mut self, item: u8) -> Result<()> {
        if !self.paused {
            match item {
                // Time
                1 => {
                    if self.terminal_size.x > self.time.size.x {
                        self.time.draw(&mut self.buffer);
                    }
                },

                // Load average
                2 => {
                    if self.terminal_size.x > (self.loadavg.size.x + self.loadavg.pos.x) && self.terminal_size.y > self.loadavg.size.y  {
                        self.loadavg.draw(&mut self.buffer)?;
                    }
                },

                // Overview
                3 => {
                    if self.terminal_size.x > (self.overview.size.x + self.overview.pos.x) && self.terminal_size.y > (self.overview.size.y + self.overview.pos.y) {
                        self.overview.draw(&mut self.buffer)?;
                    }

                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        if let Ok(cpuinfo) = self.system.cpuinfo.lock() {
                            write!(self.buffer, "\x1b[6;41H\x1b[0K\x1b[38;5;244m{}\x1b[0m", &cpuinfo.governor)?;
                        }
                    }
                },

                // Memory
                4 => {
                    if self.terminal_size.x > (self.memory.pos.x + self.memory.size.x) && self.terminal_size.y > (self.memory.pos.y + self.memory.size.y) {
                        self.memory.draw(&mut self.buffer)?;
                    }

                    if self.terminal_size.x > (self.swap.pos.x + self.swap.size.x) && self.terminal_size.y > (self.swap.pos.y + self.swap.size.y)  {
                        self.swap.draw(&mut self.buffer)?;
                    }
                },

                // Swap
                /*5 => {
                    if self.terminal_size.x > (self.swap.pos.x + self.swap.size.x) && self.terminal_size.y > (self.swap.pos.y + self.swap.size.y)  {
                        self.swap.draw(&mut self.stdout)?;
                    }
                },*/

                // Sensors
                6 => {
                    if self.terminal_size.x > self.sensors.size.x && self.terminal_size.y > (self.network.size.y + self.sensors.size.y + self.overview.size.y) as u16 {
                        if self.sensors.draw(&mut self.buffer)? {
                            self.rebuild()?;
                        }
                    }
                },

                // Network
                7 => {
                    if self.terminal_size.x > self.network.size.x  && self.terminal_size.y > (self.network.size.y + self.overview.size.y) as u16 {
                        if self.network.draw(&mut self.buffer)? {
                            self.rebuild()?;
                        }
                    }
                },

                // Process list
                8 => {
                    //let now = std::time::Instant::now();
                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        self.processes.draw(&mut self.buffer, &self.terminal_size)?;
                    }
                    //eprintln!("{}", now.elapsed().as_micros());
                    //_draw_benchmark!(self.stdout, now, self.terminal_size.x, self.terminal_size.y);
                },

                // Gpu
                9 => {
                    if self.terminal_size.x > (self.gpu.pos.x + self.gpu.size.x) && self.terminal_size.y > (self.gpu.pos.y + self.gpu.size.y)  {
                        //let now = std::time::Instant::now();
                        self.gpu.draw(&mut self.buffer)?;
                        //eprintln!("{}", now.elapsed().as_nanos());
                        //_draw_benchmark!(self.stdout, now, self.terminal_size.x, self.terminal_size.y);
                    }
                },

                // Topmode
                10 => {
                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        self.toggle_topmode();
                    }
                },

                // smaps
                11 => {
                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        self.toggle_smaps();
                    }
                },

                // all_processes
                12 => {
                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        self.toggle_all_processes();
                    }
                },

                _ => (),
            }

            write_to_stdout!(self.buffer);

            self.buffer.clear();
        }

        Ok(())
    }

    pub fn rebuild(&mut self) -> Result<()> {
        // Clear screen
        let _ = write!(self.buffer, "\x1b[2J");

        self.rebuild_cache()?;

        if self.terminal_size.x > (self.hostinfo.size.x + self.time.size.x) {
            self.hostinfo.draw(&mut self.buffer);
        }

        for i in 1..=12 {
            self.update(i)?;
        }

        Ok(())
    }

    fn toggle_topmode(&mut self) {
        if self.system.config.topmode.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = write!(self.buffer, "\x1b[6;37H\x1b[38;5;244mt\x1b[0m");
        } else {
            let _ = write!(self.buffer, "\x1b[6;37H ");
        }
    }

    fn toggle_smaps(&mut self) {
        if self.system.config.smaps.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = write!(self.buffer, "\x1b[6;38H\x1b[38;5;244ms\x1b[0m");
        } else {
            let _ = write!(self.buffer, "\x1b[6;38H ");
        }
    }

    fn toggle_all_processes(&mut self) {
        if self.system.config.all.load(std::sync::atomic::Ordering::Relaxed) {
            let _ = write!(self.buffer, "\x1b[6;39H\x1b[38;5;244ma\x1b[0m");
        } else {
            let _ = write!(self.buffer, "\x1b[6;39H ");
        }
    }
}

// Taken from https://github.com/banyan/rust-pretty-bytes/blob/master/src/converter.rs
// And customized for my use
// Convert to pretty bytes with specified right alignment
pub fn convert_with_padding(buffer: &mut String, num: u64) {
    buffer.clear();

    if num != 0 {
        // convert it to a f64 type to we can use ln() and stuff on it.
        let num = num as f64;

        //static UNITS: [&str; 9] = ["b", "Kb", "Mb", "Gb", "Tb", "Pb", "Eb", "Zb", "Yb"];

        // A kilobyte is 1024 bytes. Fight me!
        //let delimiter = 1024_f64;

        // Magic that makes no sense to me
        let exponent = (num.ln() / DELIMITER_LN).floor() as i32;
        let pretty_bytes = num / DELIMITER.powi(exponent);


        // Different behaviour for different units
        match exponent {
            3 => {
                if pretty_bytes >= 10.0 { let _ = write!(buffer, "{:>4.1} Gb", pretty_bytes); }
                else { let _ = write!(buffer, "{:>4.2} Gb", pretty_bytes); }
            },
            2 => { let _ = write!(buffer, "{:>4.0} Mb", pretty_bytes); },
            1 => { let _ = write!(buffer, "{:>4.0} Kb", pretty_bytes); },
            0 => { let _ = write!(buffer, "{:>5.0} b", pretty_bytes); },
            _ => {
                let unit = UNITS[exponent as usize];
                let _ = write!(buffer, "{:>4.1} {}", pretty_bytes, unit);
            },
        };
    } else {
        let _ = write!(buffer, "{:>5.0} b", num);
    }
}
