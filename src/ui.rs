/*use crossterm::{
    cursor, terminal, Result, execute, queue,
    style::{Print, SetColors},
};
use std::io::Write as IoWrite;
use std::fmt;
use std::fmt::Write as FmtWrite;

pub struct XY {
    x: u16,
    y: u16
}

pub struct Time <'layout> {
    pub position: XY,
    pub system: &'layout crate::system::System,

    size: XY,
    len: u16,
    cache: String,
    buffer: String,
}

impl <'layout> Time <'layout> {
    pub fn new(system: &'layout crate::system::System, position: XY) -> Self {
        Time {
            position,
            size: XY { x: 0, y: 0},
            len: 0,
            cache: String::new(),
            system: &system,
            buffer: String::new()
        }
    }
    pub fn draw(&mut self, stdout: &mut std::io::StdoutLock) {
        // Current time ~662
        if let Ok(timeinfo) = &self.system.time.read() {
            let new_len = timeinfo.time_string.len() as u16;
            if self.cache.is_empty() || self.len != new_len {
                self.len = new_len;
                self.cache.push_str(format!(
                    "{}\x1b[1K{}\x1b[0m",
                    cursor::MoveTo(self.position.x + self.len, self.position.y),
                    cursor::MoveTo(self.position.x, self.position.y),
                ).as_str());
            }

            //write!(self.buffer, "{}{}", self.cache, timeinfo.time_string.as_str()).unwrap();

            /*queue!(stdout,
                Print(&self.cache),
                Print(timeinfo.time_string.as_str())
            ).unwrap();*/
        }
    }
}*/

/*impl <'layout> fmt::Display for Time <'layout> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.buffer)
    }
}*/

/*pub struct Layout <'layout> {
    pub terminal_size: XY,
    pub stdout: &'layout mut std::io::Stdout,
    time: Time<'layout>,
}

impl <'layout> Layout <'layout> {
    pub fn new(stdout: &'layout mut std::io::Stdout, system: &'layout crate::system::System) -> Self {
        let (terminal_size_x, terminal_size_y) = terminal::size().unwrap();
        Layout {
            stdout,
            terminal_size: XY { x: terminal_size_x, y: terminal_size_y},
            time: Time::new(&system, XY {x: 0, y: terminal_size_y})
        }
    }

    pub fn update(&self, item: u8) {
        match item {
            // Time
            1 => {
                    //if let Ok(timeinfo) = system.time.read() {
                        if self.terminal_size.x > self.time.len {
                            /*queue!(self.stdout,
                                Print(&self.time.buffer),
                                //Print(timeinfo.time_string.as_str())
                            ).unwrap();*/
                        }
                    //}
            },
            _ => (),
        }
    }
}*/

/*macro_rules! draw_time {
    ($stdout:expr, $system:expr, $cache:expr) => {
        // Current time ~662
        if let Ok(timeinfo) = $system.time.read() {
            if $cache.time.is_empty() {
                $cache.time.push_str(format!(
                    "{}\x1b[1K{}\x1b[0m",
                    cursor::MoveTo(0 + timeinfo.time_string.len() as u16, $cache.tsizey),
                    cursor::MoveTo(0, $cache.tsizey),
                ).as_str());
            }

            queue!($stdout,
                Print(&$cache.time),
                Print(timeinfo.time_string.as_str())
            )?;
        }
    }
}*/

// Used for caching static parts of the UI
#[derive(Default)]
pub struct CachedCursor {
    pub tsizex: u16,
    pub tsizey: u16,

    pub overview_title: bool,
    pub overview1: String,
    pub overview2: String,
    pub overview3: String,

    pub sensors_title: bool,
    pub sensors_size: u16,
    pub sensors: Vec::<String>,
    pub sensors2: Vec::<u8>,

    pub memory_title: bool,
    pub memory1: String,
    pub memory2: String,
    pub memory3: String,

    pub swap_total: i64,
    pub swap_free: i64,
    pub swap_total_str: String,
    pub swap_used_str: String,
    pub swap_free_str: String,
    pub swap_title: bool,
    pub swap1: String,
    pub swap2: String,
    pub swap3: String,

    pub load_title: bool,
    pub load_size: usize,
    pub load1: String,
    pub load2: String,
    pub load3: String,

    pub processes_title: bool,
    pub processes_pidlen: usize,
    pub processes0: String,
    pub processes1: Vec::<String>,
    pub processes2: std::collections::HashMap<u32, String>,

    pub network_title: bool,
    pub network_size: u16,
    pub network0: String,
    pub network1: Vec::<String>,
    pub network2: Vec::<String>,
    pub network3: Vec::<String>,
    pub network4: Vec::<String>,

    pub gpu_title: bool,
    pub gpu1: String,
    pub gpu2: String,
    pub gpu3: String,
    pub gpu4: String,

    pub time: String,
    pub hostinfo: String,
}

impl CachedCursor {
    pub fn clear(&mut self) {
        self.overview_title = false;
        self.overview1.clear();
        self.overview2.clear();
        self.overview3.clear();

        self.sensors_title = false;
        self.sensors_size = 0;
        self.sensors.clear();
        self.sensors2.clear();

        self.memory_title = false;
        self.memory1.clear();
        self.memory2.clear();
        self.memory3.clear();

        self.swap_title = false;
        self.swap_total = 0;
        self.swap_free = 0;
        self.swap_total_str.clear();
        self.swap_used_str.clear();
        self.swap_free_str.clear();
        self.swap1.clear();
        self.swap2.clear();
        self.swap3.clear();

        self.load_title = false;
        self.load_size = 0;
        self.load1.clear();
        self.load2.clear();
        self.load3.clear();

        self.processes_title = false;
        self.processes_pidlen = 0;
        self.processes0.clear();
        self.processes1.clear();
        self.processes2.clear();

        self.network_title = false;
        self.network_size = 0;
        self.network0.clear();
        self.network1.clear();
        self.network2.clear();
        self.network3.clear();
        self.network4.clear();

        self.gpu_title = false;
        self.gpu1.clear();
        self.gpu2.clear();
        self.gpu3.clear();
        self.gpu4.clear();

        self.time.clear();
        self.hostinfo.clear();
    }
}

// Special handling for 0 memory for processe list
pub fn convert_with_padding_proc(num: i64, padding: usize) -> String {
    if num == -1 {
        return format!("Error");
    }
    if num == 0 {
        return format!("  {:>pad$}", "-", pad=padding+1);
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

// Convert function for network with special handling
pub fn convert_speed(num: i64, freq: u64) -> String {
    if num == -1 {
        return format!("Error");
    }
    if num == 0 {
        return format!("{:>5.0} b/s\x1b[38;5;244m ]\x1b[37m Rx\x1b[0m", num);
    }
    // convert it to a f64 type to we can use ln() and stuff on it.
    let num = num as f64 / (freq as f64 / 1000.0);

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

    // Different behaviour for different units 7
    match unit {
        "b" => format!("{:>5.0} {}/s\x1b[91m ]\x1b[37m Tx\x1b[0m", pretty_bytes, unit),
        "Kb" => format!("{:>4.0} {}/s\x1b[91m ]\x1b[37m Tx\x1b[0m", pretty_bytes, unit),
        _ => format!("{:>4.1} {}/s\x1b[91m ]\x1b[37m Tx\x1b[0m", pretty_bytes, unit),
    }
}

macro_rules! _draw_benchmark {
    ($stdout:expr, $now:expr, $x:expr, $y:expr) => {
        // update benchmark
        let shoe = $now.elapsed().as_nanos();//.to_string();
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

macro_rules! draw_loadavg {
    ($stdout:expr, $system:expr, $x:expr, $y:expr, $cache:expr) => {
        if !$cache.load_title {
            queue!(
                $stdout,
                cursor::MoveTo($x, $y),
                Print("\x1b[95mLoad\x1b[0m")
            )?;
            $cache.load_title = true;
        }
       // Load Average ~3000 -> ~2660 -> ~1570
        if let Ok(loadavg) = $system.loadavg.read() {
            if $cache.load1.is_empty() {
                $cache.load1 = format!(
                    "{}\x1b[0K\x1b[37m1 min:  \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x, $y+1)
                );
            }

            if $cache.load2.is_empty() {
                $cache.load2 = format!(
                    "\x1b[91m ]\x1b[0m{}\x1b[0K\x1b[37m5 min:  \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x, $y+2)
                );
            }

            if $cache.load3.is_empty() {
                $cache.load3 = format!(
                    "\x1b[91m ]\x1b[0m{}\x1b[0K\x1b[37m15 min: \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x, $y+3)
                );
            }

            let len = loadavg.min1.len().max(loadavg.min5.len().max(loadavg.min15.len()));

            queue!(
                $stdout,
                Print(&$cache.load1),

                Print(&format!("{:>pad$}", &loadavg.min1, pad=len)),

                Print(&$cache.load2),

                Print(&format!("{:>pad$}", &loadavg.min5, pad=len)),

                Print(&$cache.load3),

                Print(&format!("{:>pad$}", &loadavg.min15, pad=len)),
                Print("\x1b[91m ]\x1b[0m")
            )?;
        }
    }
}

macro_rules! draw_memory {
    ($stdout:expr, $system:expr, $x:expr, $y:expr, $cache:expr) => {
        // Memory ~5860 -> ~5440 -> ~4750
        if !$cache.memory_title {
            queue!(
                $stdout,
                cursor::MoveTo($x, $y),
                Print("\x1b[95mMemory\x1b[0m")
            )?;
            $cache.memory_title = true;
        }
        if let Ok(val) = $system.memoryinfo.read() {
            if $cache.memory1.is_empty() {
                $cache.memory1.push_str(format!(
                    "{}                    {}\x1b[37mTotal: \x1b[38;5;244m[ \x1b[37m",
                    cursor::MoveTo($x, $y+1),
                    cursor::MoveTo($x, $y+1)
                ).as_str());
            }

            if $cache.memory2.is_empty() {
                $cache.memory2.push_str(format!(
                    "\x1b[38;5;244m ]\x1b[0m{}                    {}\x1b[37mUsed:  \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x, $y+2),
                    cursor::MoveTo($x, $y+2)
                ).as_str());
            }

            if $cache.memory3.is_empty() {
                $cache.memory3.push_str(format!(
                    "\x1b[91m ]\x1b[0m{}                    {}\x1b[37mFree:  \x1b[38;5;244m[ \x1b[37m",
                    cursor::MoveTo($x, $y+3),
                    cursor::MoveTo($x, $y+3)
                ).as_str());
            }

            //let now = std::time::Instant::now();
            queue!(
                $stdout,
                Print(&$cache.memory1),
                Print(&ui::convert_with_padding(val.total, 4)),

                Print(&$cache.memory2),
                Print(&ui::convert_with_padding(val.used, 4)),

                Print(&$cache.memory3),
                Print(&ui::convert_with_padding(val.free, 4)),
                Print("\x1b[38;5;244m ]\x1b[0m")
            )?;
            //eprintln!("{}", now.elapsed().as_nanos());
        }
    }
}

macro_rules! draw_swap {
    ($stdout:expr, $system:expr, $x:expr, $y:expr, $cache:expr) => {
        // Swap ~8500 -> 6700 -> 5276 -> 4843
        if !$cache.swap_title {
            queue!(
                $stdout,
                cursor::MoveTo($x, $y),
                Print("\x1b[95mSwap\x1b[0m")
            )?;
            $cache.swap_title = true;
        }

        if let Ok(val) = $system.swapinfo.read() {
            if $cache.swap1.is_empty() {
                $cache.swap1.push_str(format!(
                    "{}                  {}\x1b[37mTotal: \x1b[38;5;244m[ \x1b[37m",
                    cursor::MoveTo($x, $y+1),
                    cursor::MoveTo($x, $y+1)
                ).as_str());
            }

            if $cache.swap2.is_empty() {
                $cache.swap2.push_str(format!(
                    "\x1b[38;5;244m ]\x1b[0m{}                  {}\x1b[37mUsed:  \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x, $y+2),
                    cursor::MoveTo($x, $y+2)
                ).as_str());
            }

            if $cache.swap3.is_empty() {
                $cache.swap3.push_str(format!(
                    "\x1b[91m ]\x1b[0m{}                  {}\x1b[37mFree:  \x1b[38;5;244m[ \x1b[37m",
                    cursor::MoveTo($x, $y+3),
                    cursor::MoveTo($x, $y+3)
                ).as_str());
            }

            if $cache.swap_total != val.total {
                $cache.swap_total = val.total;
                $cache.swap_total_str = ui::convert_with_padding(val.total, 4);
            }

            if $cache.swap_free != val.free {
                $cache.swap_free = val.free;
                $cache.swap_free_str = ui::convert_with_padding(val.free, 4);
                $cache.swap_used_str = ui::convert_with_padding(val.used, 4);
            }

            queue!(
                $stdout,
                Print(&$cache.swap1),

                Print(&$cache.swap_total_str),
                Print(&$cache.swap2),

                Print(&$cache.swap_used_str),
                Print(&$cache.swap3),

                Print(&$cache.swap_free_str),
                Print("\x1b[38;5;244m ]\x1b[0m")
            )?;
        }
    }
}

macro_rules! draw_sensors {
    ($stdout:expr, $system:expr, $x:expr, $y:expr, $cache:expr) => {
        // Sensors  12200 -> 9100 -> 4800
        let y: u16 = ($cache.network_size + 5);

        if !$cache.sensors_title {
            queue!(
                $stdout,
                cursor::MoveTo($x, y),
                Print("\x1b[95mSensors\x1b[0m")
            )?;
            $cache.sensors_title = true;
        }

        let mut count = 0;
        if let Ok(sensorinfo) = $system.sensorinfo.read() {
            for (idx, (key, val)) in sensorinfo.chips.iter().enumerate() {
                if (y + 1 + idx as u16) < (y + 8) {
                    if $cache.sensors.len() <= idx {
                        $cache.sensors.push(format!(
                            "{}\x1b[1K{}\x1b[37m{}{}\x1b[91m[ \x1b[92m",
                            cursor::MoveTo($x+23, y + 1 + idx as u16),
                            cursor::MoveTo($x, y + 1 + idx as u16),
                            key,
                            cursor::MoveTo($x+15, y + 1 + idx as u16),
                        ));
                        $cache.sensors2.push(0);
                    }

                    unsafe {
                        if *$cache.sensors2.get_unchecked(idx) != *val {
                            queue!(
                                $stdout,
                                Print(&$cache.sensors.get_unchecked(idx)),
                                Print(&format!("{}", val)),
                                Print(" C\x1b[91m ]\x1b[0m"),
                            )?;
                            $cache.sensors2[idx] = *val;

                        }
                    }
                }
                count = idx;
            }
            $cache.sensors_size = count as u16 + 2;
        }
    }
}

macro_rules! draw_network {
    ($stdout:expr, $system:expr, $x:expr, $y:expr, $cache:expr) => {
        // Network perf gov: 6000 -> 5700 -> 5170 -> 4950 -> 4670 -> 4560
        if !$cache.network_title {
            queue!(
                $stdout,
                cursor::MoveTo($x, $y),
                Print("\x1b[95mNetwork\x1b[0m")
            )?;
            $cache.network_title = true;
        }

        if let Ok(networkinfo) = $system.networkinfo.read() {
            let freq = $system.config.frequency.load(atomic::Ordering::Relaxed);
            let mut count = 0;

            for (key, val) in networkinfo.stats.iter() {
                if $cache.network1.len() <= count / 2 {
                    $cache.network1.push(format!(
                        "{}\x1b[1K{}\x1b[37m{:<8}\x1b[91m[ \x1b[92m",
                        cursor::MoveTo($x+25, $y + 1 + count as u16),
                        cursor::MoveTo($x, $y + 1 + count as u16),
                        key
                    ));
                }

                if $cache.network2.len() <= count / 2 {
                    $cache.network2.push(format!(
                        "{}\x1b[1K{}\x1b[37m{:<8}\x1b[38;5;244m[ \x1b[37m",
                        cursor::MoveTo($x+25, $y + 1 + count as u16),
                        cursor::MoveTo($x, $y + 1 + count as u16),
                        key
                    ));
                }

                if $cache.network3.len() <= count / 2 {
                    $cache.network3.push(format!(
                        "{}\x1b[91m{:>10}\x1b[92m",
                        cursor::MoveTo($x, $y + 2 + count as u16),
                        "[ ",
                    ));
                }

                if $cache.network4.len() <= count / 2 {
                    $cache.network4.push(format!(
                        "{}\x1b[38;5;244m{:>10}\x1b[37m",
                        cursor::MoveTo($x, $y + 2 + count as u16),
                        "[ ",

                    ));
                }

                unsafe {
                    if val.recv != 0 {
                        queue!($stdout,
                            Print(&$cache.network1.get_unchecked(count / 2)),
                            Print(&ui::convert_speed(val.recv, freq)),

                        )?;

                    } else {
                        queue!($stdout,
                            Print(&$cache.network2.get_unchecked(count / 2)),
                            Print(&ui::convert_speed(val.recv, freq))

                        )?;
                    }

                    if val.sent != 0 {
                        queue!($stdout,
                            Print(&$cache.network3.get_unchecked(count / 2)),
                            Print(&ui::convert_speed(val.sent, freq)),

                        )?;
                    } else {
                        queue!($stdout,
                            Print(&$cache.network4.get_unchecked(count / 2)),
                            Print(&ui::convert_speed(val.sent, freq))

                        )?;
                    }
                }
                count += 2;
            }
            $cache.network_size = count as u16 + 2;
        }
    }
}

macro_rules! draw_processes {
    ($stdout:expr, $system:expr, $x:expr, $y:expr, $cache:expr) => {
        // Processes perf gov: 59100 -> 48000 -> 38200 -> 33600 -> 32850 -> 30000
        if !$cache.processes_title {
            queue!(
                $stdout,
                cursor::MoveTo($x, $y),
                Print("\x1b[95mProcesses\x1b[0m")
            )?;
            $cache.processes_title = true;
        }

        if let Ok(processinfo) = $system.processinfo.read() {
        //for (idx, (_, val)) in processinfo.cpu_sort_combined($system.cpuinfo.read().unwrap().totald, $system.cpuinfo.read().unwrap().cpu_count).iter().enumerate() {
            //let now = std::time::Instant::now();
            let (pidlen, vector) = processinfo.cpu_sort();

            //draw_benchmark!($stdout, now, $tsizex, $tsizey);
            //for (idx, (_, val)) in processinfo.cpu_sort().iter().enumerate() {
            for (idx, (_, val)) in vector.iter().enumerate() {
                if $cache.processes1.len() <= idx {
                    $cache.processes1.push(format!(
                        "{}\x1b[0K{}",
                        cursor::MoveTo($x, $y + 1 + idx as u16),
                        cursor::MoveTo($x, $y + 1 + idx as u16),
                    ));
                }

                unsafe {
                    queue!($stdout,
                        Print(&$cache.processes1.get_unchecked(idx)),
                    )?;
                }

                if val.cpu_avg > 0.0 && val.cpu_avg < 99.5 {
                    queue!($stdout,
                        Print(&format!("\x1b[91m[ \x1b[92m{:>4.1}%\x1b[91m ] \x1b[0m\x1b[91m[ \x1b[92m", val.cpu_avg)),
                    )?;
                } else if val.cpu_avg >= 99.5 {
                    queue!($stdout,
                        Print(&format!("\x1b[91m[ \x1b[92m{:>4.0}%\x1b[91m ] \x1b[0m\x1b[91m[ \x1b[92m", val.cpu_avg)),
                    )?;
                } else {
                    queue!($stdout,
                        Print(&format!("\x1b[38;5;244m[ \x1b[37m{:>4.1}%\x1b[38;5;244m ] \x1b[0m\x1b[91m[ \x1b[92m", val.cpu_avg)),
                    )?;
                }

                if $system.config.smaps.load(atomic::Ordering::Relaxed) {
                    // Check if there actually is a PSS value
                    // If there isn't it probably requires root access, use RSS instead
                    if val.pss != -1 {
                        queue!(
                            $stdout,
                            Print(&format!("\x1b[94m{}\x1b[0m", &ui::convert_with_padding_proc(val.pss, 4))),
                        )?;
                    } else {
                        queue!(
                            $stdout,
                            Print(&ui::convert_with_padding_proc(val.rss, 4)),
                        )?;
                    }
                } else {
                    queue!(
                        $stdout,
                        Print(&ui::convert_with_padding_proc(val.rss, 4)),
                    )?;
                }

                // Update cache if the length of PID increases
                if pidlen > $cache.processes_pidlen {
                    $cache.processes2.clear();
                }

                if !$cache.processes2.contains_key(&val.pid) {
                    let mut maxchars = String::new();

                    if val.not_executable {
                        maxchars.push_str(&format!("\x1b[91m ] \x1b[0m\x1b[37m{:>pad$}\x1b[0m \x1b[96m", val.pid, pad=pidlen));
                        maxchars.push_str(val.executable.as_str());

                    } else {
                        maxchars.push_str(&format!("\x1b[91m ] \x1b[0m\x1b[37m{:>pad$}\x1b[0m \x1b[92m", val.pid, pad=pidlen));
                        maxchars.push_str(val.executable.as_str());

                        maxchars.push_str("\x1b[38;5;244m");
                        maxchars.push_str(val.cmdline.as_str());
                    }

                    maxchars.truncate(($cache.tsizex - $x + 15) as usize);
                    maxchars.push_str("\x1b[0m");

                    $cache.processes2.insert(val.pid, maxchars);
                }

                queue!(
                    $stdout,
                    Print(&$cache.processes2.get(&val.pid).unwrap()),
                )?;

                // Stop if there isn't any more room in the UI
                if ($cache.tsizey - $y - 1 - idx as u16) == 3 {
                    break;
                }

            }
            // Save the length of the longest PID in the cache so we can check if it changes
            // In which case we need to rebuild the cache
            $cache.processes_pidlen = pidlen;
        }
    }
}

macro_rules! draw_time {
    ($stdout:expr, $system:expr, $cache:expr) => {
        // Current time ~662
        if let Ok(timeinfo) = $system.time.read() {
            if $cache.time.is_empty() {
                $cache.time.push_str(format!(
                    "{}\x1b[1K{}\x1b[0m",
                    cursor::MoveTo(0 + timeinfo.time_string.len() as u16, $cache.tsizey),
                    cursor::MoveTo(0, $cache.tsizey),
                ).as_str());
            }

            queue!($stdout,
                Print(&$cache.time),
                Print(timeinfo.time_string.as_str())
            )?;
        }
    }
}

macro_rules! draw_overview {
    ($stdout:expr, $system:expr, $x:expr, $y:expr, $cache:expr) => {
        // Print the infos ~7200 -> ~7100 -> ~6700
        if !$cache.overview_title {
            queue!(
                $stdout,
                cursor::MoveTo($x, $y),
                Print("\x1b[95mOverview\x1b[0m")
            )?;
            $cache.overview_title = true;
        }

        if $cache.overview1.is_empty() {
            $cache.overview1.push_str(format!(
                "{}\x1b[1K{}\x1b[37mCPU:  \x1b[91m[ \x1b[92m",
                cursor::MoveTo($x+16, $y+1),
                cursor::MoveTo($x, $y+1),
            ).as_str());
        }
        if $cache.overview2.is_empty() {
            $cache.overview2.push_str(format!(
                "{}\x1b[1K{}\x1b[37mMem:  \x1b[91m[ \x1b[92m",
                cursor::MoveTo($x+16, $y+2),
                cursor::MoveTo($x, $y+2),
            ).as_str());
        }
        if $cache.overview3.is_empty() {
            $cache.overview3.push_str(format!(
                "{}\x1b[1K{}\x1b[37mSwap: \x1b[91m[ \x1b[92m",
                cursor::MoveTo($x+16, $y+3),
                cursor::MoveTo($x, $y+3),
            ).as_str());
        }

        if let Ok(cpuinfo) = $system.cpuinfo.read() {
            if cpuinfo.cpu_avg < 100.0{
                queue!($stdout,
                    Print(&$cache.overview1),
                    Print(&format!("{:4.1}%\x1b[91m ]\x1b[0m", cpuinfo.cpu_avg)),
                )?;
            } else if cpuinfo.cpu_avg >= 100.0 {
                queue!($stdout,
                    Print(&$cache.overview1),
                    Print(&format!("{:4.0}%\x1b[91m ]\x1b[0m", cpuinfo.cpu_avg)),
                )?;
            }
        }



        if let Ok(memoryinfo) = $system.memoryinfo.read() {
            let mem_use = (memoryinfo.used as f32 / memoryinfo.total as f32) * 100.0;

            if mem_use < 100.0 {
                queue!($stdout,
                    Print(&$cache.overview2),
                    Print(&format!("{:4.1}%\x1b[91m ]\x1b[0m", mem_use)),
                )?;
            } else if mem_use >= 100.0 {
                queue!($stdout,
                    Print(&$cache.overview2),
                    Print(&format!("{:4.0}%\x1b[91m ]\x1b[0m", mem_use)),
                )?;
            }
        }


        if let Ok(swapinfo) = $system.swapinfo.read() {
            let swap_use = (swapinfo.used as f32 / swapinfo.total as f32) * 100.0;

            if swap_use < 100.0{
                queue!($stdout,
                    Print(&$cache.overview3),
                    Print(&format!("{:4.1}%\x1b[91m ]\x1b[0m", swap_use)),
                )?;
            } else if swap_use >= 100.0 {
                queue!($stdout,
                    Print(&$cache.overview3),
                    Print(&format!("{:4.0}%\x1b[91m ]\x1b[0m", swap_use)),
                )?;
            }
        }
    }
}

macro_rules! draw_gpu {
    ($stdout:expr, $system:expr, $x:expr, $y:expr, $cache:expr) => {
        // 12381 -> 7581 -> 3976 -> ~3300 -> ~3150
        let y = $cache.network_size + $cache.sensors_size + 5;
        if !$cache.gpu_title {
            queue!(
                $stdout,
                cursor::MoveTo($x, y),
                Print("\x1b[95mGpu\x1b[0m")
            )?;
            $cache.gpu_title = true;
        }

        if let Ok(val) = $system.gpuinfo.read() {
            if $cache.gpu1.is_empty() {
                $cache.gpu1.push_str(format!(
                "{}\x1b[1K{}\x1b[1K{}\x1b[1K{}\x1b[1K{}\x1b[37mTemp:         \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x+24, y+1),
                    cursor::MoveTo($x+24, y+2),
                    cursor::MoveTo($x+24, y+3),
                    cursor::MoveTo($x+24, y+4),
                    cursor::MoveTo($x, y+1),
                ).as_str());
            }

            if $cache.gpu2.is_empty() {
                $cache.gpu2.push_str(format!(
                    " C\x1b[91m ]\x1b[0m{}\x1b[37mGpu load:     \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x, y+2),
                ).as_str());
            }
            if $cache.gpu3.is_empty() {
                $cache.gpu3.push_str(format!(
                    "%\x1b[91m ]\x1b[0m{}\x1b[37mMem load:     \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x, y+3),
                ).as_str());
            }
            if $cache.gpu4.is_empty() {
                $cache.gpu4.push_str(format!(
                    "%\x1b[91m ]\x1b[0m{}\x1b[37mMem use:      \x1b[91m[ \x1b[92m",
                    cursor::MoveTo($x, y+4),
                ).as_str());
            }

            queue!(
                $stdout,

                Print(&$cache.gpu1),

                Print(&format!("{:>3}", val.temp)),
                Print(&$cache.gpu2),

                Print(&format!("{:>4}", val.gpu_load)),
                Print(&$cache.gpu3),

                Print(&format!("{:>4}", val.mem_load)),
                Print(&$cache.gpu4)
                )?;

            if val.mem_used < 100.0 {
                queue!($stdout,
                    Print(&format!("{:>4.1}", val.mem_used)),
                    Print("%\x1b[91m ]\x1b[0m")
                )?;
            } else if val.mem_used >= 100.0 {
                queue!($stdout,
                    Print(&format!("{:>4.0}", val.mem_used)),
                    Print("%\x1b[91m ]\x1b[0m")
                )?;
            }
        }
    }
}

macro_rules! draw_hostinfo {
    ($stdout:expr, $system:expr, $cache:expr) => {
        if $cache.hostinfo.is_empty() {
            let dist_len = $system.hostinfo.distname.len();
            let kern_len = $system.hostinfo.kernel.len();

            $cache.hostinfo.push_str(format!(
                "{}\x1b[0K{}\x1b[91m[ {}{}\x1b[91m ] [ \x1b[0m{}\x1b[91m ]\x1b[0m",
                cursor::MoveTo((($cache.tsizex - 9) as u16).saturating_sub((dist_len + kern_len) as u16), $cache.tsizey),
                cursor::MoveTo(
                    (($cache.tsizex - 9) as u16).saturating_sub((dist_len + kern_len) as u16),
                    $cache.tsizey
                ),
                SetColors($system.hostinfo.ansi_color.into()),
                $system.hostinfo.distname,
                $system.hostinfo.kernel
            ).as_str());
        }

        queue!(
            $stdout,

            Print(&$cache.hostinfo),
        )?;
    }
}
