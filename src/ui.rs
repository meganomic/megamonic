use crossterm::{terminal, queue, cursor, style::Print};
use std::io::Write;
use anyhow::Result;

mod time;
use time::Time as Time;

mod memory;
use memory::Memory as Memory;

mod swap;
use swap::Swap as Swap;

mod loadavg;
use loadavg::Loadavg as Loadavg;

mod overview;
use overview::Overview as Overview;

mod hostinfo;
use hostinfo::Hostinfo as Hostinfo;

mod processes;
use processes::Processes as Processes;

mod network;
use network::Network as Network;

mod sensors;
use self::sensors::Sensors as Sensors;

mod gpu;
use gpu::Gpu as Gpu;

static mut _CUMULATIVE_BENCHMARK: u128 = 0;
static mut _CUMULATIVE_COUNT: u128 = 0;

macro_rules! _draw_benchmark {
    ($stdout:expr, $now:expr, $x:expr, $y:expr) => {
        // update benchmark
        let shoe = $now.elapsed().as_nanos();
        unsafe {
        _CUMULATIVE_BENCHMARK += shoe;
        _CUMULATIVE_COUNT += 1;
        let tid = (_CUMULATIVE_BENCHMARK / _CUMULATIVE_COUNT).to_string() + " " + _CUMULATIVE_COUNT.to_string().as_str();

        queue!(
            $stdout,
            // Clear line to end
            cursor::MoveTo($x - 5 - tid.len() as u16, $y),
            Print("\x1b[0K"),

            cursor::MoveTo(
                $x - 4 - tid.len() as u16,
                $y
            ),
            Print(&format!("μs: {}", tid))
        )?;
        }
    }
}

#[derive(Default)]
pub struct XY {
    pub x: u16,
    pub y: u16
}

pub struct Ui <'ui> {
    pub terminal_size: XY,

    paused: bool,

    stdout: std::io::Stdout,
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
    pub fn new(system: &'ui super::system::System) -> Self {
        let (tsizex, tsizey) = terminal::size().expect("Can't get terminal size");

        Self {
            paused: false,
            stdout: std::io::stdout(),
            system,
            terminal_size: XY { x: tsizex, y: tsizey },

            time: Time::new(system, XY { x: 0, y: tsizey }),
            overview: Overview::new(system, XY { x: 0, y: 0 }),
            memory: Memory::new(system, XY { x: 17, y: 0 }),
            swap: Swap::new(system, XY { x: 37, y: 0 }),
            loadavg: Loadavg::new(system, XY { x: 57, y: 0 }),
            hostinfo: Hostinfo::new(system, XY { x: 0, y: tsizey }),
            processes: Processes::new(system, XY { x: 26, y: 5 }),
            network: Network::new(system, XY { x: 0, y: 5 }),
            sensors: Sensors::new(system, XY { x: 0, y: 11 }),
            gpu: Gpu::new(system, XY { x: 0, y: 21 }),
        }
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

        self.loadavg.rebuild_cache()?;
        self.overview.rebuild_cache();
        self.memory.rebuild_cache();
        self.swap.rebuild_cache();
        self.processes.rebuild_cache(&self.terminal_size);
        self.processes.draw_static(&mut self.stdout)?;

        // The following objects rely on the position of the previous ones
        // So don't do anything silly.
        self.network.rebuild_cache();
        self.network.draw_static(&mut self.stdout)?;

        self.sensors.pos.y = self.network.size.y + self.network.pos.y;
        self.sensors.rebuild_cache();
        self.sensors.draw_static(&mut self.stdout)?;

        self.gpu.pos.y = self.sensors.pos.y + self.sensors.size.y;
        self.gpu.rebuild_cache();
        self.gpu.draw_static(&mut self.stdout)?;

        Ok(())
    }

    pub fn update (&mut self, item: u8) -> Result<()> {
        if !self.paused {
            match item {
                // Time
                1 => {
                    if self.terminal_size.x > self.time.size.x {
                        self.time.draw(&mut self.stdout)?;
                    }
                },

                // Load average
                2 => {
                    if self.terminal_size.x > (self.loadavg.size.x + self.loadavg.pos.x) && self.terminal_size.y > self.loadavg.size.y  {
                        self.loadavg.draw(&mut self.stdout)?;
                    }
                },

                // Overview
                3 => {
                    if self.terminal_size.x > (self.overview.size.x + self.overview.pos.x) && self.terminal_size.y > (self.overview.size.y + self.overview.pos.y) {
                        self.overview.draw(&mut self.stdout)?;
                    }

                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        if let Ok(cpuinfo) = self.system.cpuinfo.read() {
                            queue!(self.stdout,
                                //cursor::MoveTo(40, 5),
                                Print(&format!("\x1b[6;41H\x1b[0K\x1b[38;5;244m{}\x1b[0m", &cpuinfo.governor))
                            )?;
                        }
                    }
                },

                // Memory
                4 => {
                    if self.terminal_size.x > (self.memory.pos.x + self.memory.size.x) && self.terminal_size.y > (self.memory.pos.y + self.memory.size.y) {
                        self.memory.draw(&mut self.stdout)?;
                    }
                },

                // Swap
                5 => {
                    if self.terminal_size.x > (self.swap.pos.x + self.swap.size.x) && self.terminal_size.y > (self.swap.pos.y + self.swap.size.y)  {
                        self.swap.draw(&mut self.stdout)?;
                    }
                },

                // Sensors
                6 => {
                    if self.terminal_size.x > self.sensors.size.x && self.terminal_size.y > (self.network.size.y + self.sensors.size.y + self.overview.size.y) as u16 {
                        self.sensors.draw(&mut self.stdout)?;
                    }
                },

                // Network
                7 => {
                    if self.terminal_size.x > self.network.size.x  && self.terminal_size.y > (self.network.size.y + self.overview.size.y) as u16 {
                        let sy = self.network.size.y;

                        self.network.draw(&mut self.stdout)?;

                        // If network size changes rebuild the UI
                        if sy != self.network.size.y {
                            self.rebuild()?;
                        }
                    }
                },

                // Process list
                8 => {
                    //let now = std::time::Instant::now();
                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        self.processes.draw(&mut self.stdout, &self.terminal_size)?;
                    }
                    //eprintln!("{}", now.elapsed().as_micros());
                    //_draw_benchmark!(self.stdout, now, self.terminal_size.x, self.terminal_size.y);
                },

                // Gpu
                9 => {
                    if self.terminal_size.x > (self.gpu.pos.x + self.gpu.size.x) && self.terminal_size.y > (self.gpu.pos.y + self.gpu.size.y)  {
                        //let now = std::time::Instant::now();
                        self.gpu.draw(&mut self.stdout)?;
                        //eprintln!("{}", now.elapsed().as_nanos());
                        //_draw_benchmark!(self.stdout, now, self.terminal_size.x, self.terminal_size.y);
                    }
                },

                // Topmode
                10 => {
                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        self.toggle_topmode()?;
                    }
                },

                // smaps
                11 => {
                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        self.toggle_smaps()?;
                    }
                },

                // all_processes
                12 => {
                    if self.terminal_size.x > (self.processes.pos.x + 22) && self.terminal_size.y > (self.processes.pos.y + 3) {
                        self.toggle_all_processes()?;
                    }
                },

                _ => (),
            }
        }

        Ok(())
    }

    pub fn rebuild(&mut self) -> Result<()> {
        queue!(self.stdout, terminal::Clear(terminal::ClearType::All))?;

        self.rebuild_cache()?;

        for i in 1..=12 {
            self.update(i)?;
        }

        if self.terminal_size.x > (self.hostinfo.size.x + self.time.size.x) {
            self.hostinfo.draw(&mut self.stdout)?;
        }

        Ok(())
    }

    fn toggle_topmode(&mut self) -> Result<()> {
        if self.system.config.topmode.load(std::sync::atomic::Ordering::Relaxed) {
            queue!(self.stdout,
                cursor::MoveTo(36, 5),
                Print("\x1b[38;5;244mt\x1b[0m")
            )?;
        } else {
            queue!(self.stdout,
                cursor::MoveTo(36, 5),
                Print(" ")
            )?;
        }

        Ok(())
    }

    fn toggle_smaps(&mut self) -> Result<()> {
        if self.system.config.smaps.load(std::sync::atomic::Ordering::Relaxed) {
            queue!(self.stdout,
                cursor::MoveTo(37, 5),
                Print("\x1b[38;5;244ms\x1b[0m")
            )?;
        } else {
            queue!(self.stdout,
                cursor::MoveTo(37, 5),
                Print(" ")
            )?;
        }

        Ok(())
    }

    fn toggle_all_processes(&mut self) -> Result<()> {
        if self.system.config.all.load(std::sync::atomic::Ordering::Relaxed) {
            queue!(self.stdout,
                cursor::MoveTo(38, 5),
                Print("\x1b[38;5;244ma\x1b[0m")
            )?;
        } else {
            queue!(self.stdout,
                cursor::MoveTo(38, 5),
                Print(" ")
            )?;
        }

        Ok(())
    }
}

// Convert to pretty bytes with specified right alignment
pub fn convert_with_padding(num: i64, padding: usize) -> String {
    if num == -1 {
        return format!("Error");
    }
    if num == 0 {
        return format!("{:>pad$.0} b", num, pad=padding+1);
    }
    // convert it to a f64 type to we can use ln() and stuff on it.
    let num = num as f64;

    let units = ["b", "Kb", "Mb", "Gb", "Tb", "Pb", "Eb", "Zb", "Yb"];

    // A kilobyte is 1024 bytes. Fight me!
    let delimiter = 1024_f64;

    // Magic that makes no sense to me
    let exponent = std::cmp::min(
        (num.ln() / delimiter.ln()).floor() as i32,
        (units.len() - 1) as i32,
    );
    let pretty_bytes = num / delimiter.powi(exponent as i32);
    let unit = units[exponent as usize];

    // Different behaviour for different units
    match unit {
        "b" => format!("{:>pad$.0} {}", pretty_bytes, unit, pad=padding+1),
        "Kb" | "Mb" => format!("{:>pad$.0} {}", pretty_bytes, unit, pad=padding),
        "Gb" => {
            if pretty_bytes >= 10.0 { format!("{:>pad$.1} {}", pretty_bytes, unit, pad=padding) }
            else { format!("{:>pad$.2} {}", pretty_bytes, unit, pad=padding) }
        },
        _ => format!("{:>pad$.1} {}", pretty_bytes, unit, pad=padding),
    }
}




