use crossterm::{
    cursor, terminal, Result, execute, queue,
    style::{Print, SetColors},
};

use std::io::{stdout, Write};

use std::sync::atomic;
mod system;

#[macro_use]
mod ui;

static mut _CUMULATIVE_BENCHMARK: u128 = 0;
static mut _CUMULATIVE_COUNT: u128 = 0;

fn draw_full_ui(stdout: &mut std::io::StdoutLock, system: &system::System, cache: &mut ui::CachedCursor) -> Result<()> {
    cache.clear();

    queue!(stdout, terminal::Clear(terminal::ClearType::All))?;

    if cache.tsizex > 14 && cache.tsizey > 4 {
        draw_overview!(stdout, system, 0, 0, cache);
    }

    if cache.tsizex > 34 && cache.tsizey > 4  {
        draw_memory!(stdout, system, 17, 0, cache);
    }

    if cache.tsizex > 54 && cache.tsizey > 4  {
        draw_swap!(stdout, system, 37, 0, cache);
    }

    if cache.tsizex > 72 && cache.tsizey > 4  {
        draw_loadavg!(stdout, system, 57, 0, cache);
    }

    if cache.tsizex > 50 && cache.tsizey > 8 {
        draw_processes!(stdout, system, 26, 5, cache);
    }

    if cache.tsizex > 23  && cache.tsizey > (cache.network_size + 4) as u16 {
        draw_network!(stdout, system, 0, 5, cache);
    }

    if cache.tsizex > 22 && cache.tsizey > (cache.network_size + cache.sensors_size + 4) as u16 {
        draw_sensors!(stdout, system, 0, 11, cache);
    }

    if cache.tsizex > 22 && cache.tsizey > (cache.network_size + cache.sensors_size + 10) as u16 {
        draw_gpu!(stdout, system, 0, 21, cache);
    }

    if let Ok(timeinfo) = system.time.read() {
        if cache.tsizex > timeinfo.time_string.len() as u16 {
            draw_time!(stdout, system, cache);
        }

        if cache.tsizex > (timeinfo.time_string.len() + system.hostinfo.distname.len() + system.hostinfo.kernel.len()) as u16 + 9 {
            draw_hostinfo!(stdout, system, cache);
        }
    }

    if system.config.topmode.load(std::sync::atomic::Ordering::Relaxed) {
        queue!(stdout,
            cursor::MoveTo(36, 5),
            Print("\x1b[38;5;244mt\x1b[0m")
        )?;
    } else {
        queue!(stdout,
            cursor::MoveTo(36, 5),
            Print(" ")
        )?;
    }

    if system.config.smaps.load(std::sync::atomic::Ordering::Relaxed) {
        queue!(stdout,
            cursor::MoveTo(37, 5),
            Print("\x1b[38;5;244ms\x1b[0m")
        )?;
    } else {
        queue!(stdout,
            cursor::MoveTo(37, 5),
            Print(" ")
        )?;
    }

    if system.config.all.load(std::sync::atomic::Ordering::Relaxed) {
        queue!(stdout,
            cursor::MoveTo(38, 5),
            Print("\x1b[38;5;244ma\x1b[0m")
        )?;
    } else {
        queue!(stdout,
            cursor::MoveTo(38, 5),
            Print(" ")
        )?;
    }

    stdout.flush()?;

    Ok(())
}

fn main() -> Result<()> {
    let options = clap::App::new("Megamonic")
        .about("A silly system monitor")
        .version("0.1.0")
        .arg(
            clap::Arg::with_name("smaps")
                .short("s")
                .long("enable-smaps")
                .help("Enable use of PSS value instead of RSS for memory reporting\nRequires root for some processes (very slow)")
        )
        .arg(
            clap::Arg::with_name("strftime")
                .long("strftime")
                .help("Strftime format string")
                .default_value("%c")
        )
        .arg(
            clap::Arg::with_name("topmode")
                .short("t")
                .long("enable-top-mode")
                .help("Report CPU % the same way 'top' does")
        )
        .arg(
            clap::Arg::with_name("all")
                .short("a")
                .long("enable-all-processes")
                .help("Shows all processes, including kernel threads and other stuff (slow)")
        )
        .arg(
            clap::Arg::with_name("frequency")
                .short("f")
                .long("frequency")
                .help("Update frequency in milliseconds. Min: 1000, Max: 5000")
                .default_value("1000")
        )
        .after_help("\x1b[91mEnabling both smaps and all processes is ultra slow.\nEspecially if running as root.\x1b[0m\n\nYou can toggle some things by pressing these buttons:\nPress 'q' to exit.\nPress 'a' to toggle all processes.\nPress 's' to toggle smaps.\nPress 't' to toggle \"Top mode\"\nPress 'r' to rebuild the UI incase it's broken\nPress [space] to pause the UI.")
        .get_matches();

    let stdout_l = stdout();
    let mut stdout = stdout_l.lock();

    // Initialize System and set the configuration options
    let mut system = system::System {
        config: std::sync::Arc::new(system::Config {
            smaps: atomic::AtomicBool::new(options.is_present("smaps")),
            topmode: atomic::AtomicBool::new(options.is_present("topmode")),
            all: atomic::AtomicBool::new(options.is_present("all")),
            frequency: atomic::AtomicU64::new(options.value_of("frequency").unwrap_or("1000").parse::<u64>().map_or(1000, |v| if v > 5000 { 5000 } else if v < 1000 { 1000 } else { v })),
            strftime_format: options.value_of("strftime").unwrap_or("%c").to_string(),
        }),
        ..Default::default()
    };

    // Size of the terminal window
    let (tsizex, tsizey) = terminal::size()?;

    // Disable all hotkeys and stuff. ctrl+c won't work.
    terminal::enable_raw_mode()?;

    // Setup the terminal screen and display a loading message
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::DisableLineWrap,
        cursor::Hide,
        cursor::MoveTo(tsizex / 2 - 16, tsizey / 2 - 1),
        Print("Launching monitoring threads...")
    )?;

    // Event channel
    let (tx, rx) = std::sync::mpsc::channel();

    // Start monitoring threads
    system.start(tx.clone());

    // Show some info so the user knows what's going on
    execute!(
        stdout,
        cursor::MoveTo(tsizex / 2 - 9, tsizey / 2),
        Print("Gathering data...")
    )?;

    // UI cache
    let mut cache = ui::CachedCursor { tsizex, tsizey, ..Default::default() };

    // Used to pause the UI
    let mut paused = false;

    // No point in drawing the UI before we have any data.
    std::thread::sleep(std::time::Duration::from_secs(1));

    //let mut shoe = ui::Layout::new(&mut stdout, &system);
    //let shoe = ui::Time::new(&system);

    draw_full_ui(&mut stdout, &system, &mut cache)?;

    // Main loop
    for event in rx.iter() {
        match event {
            // Time
            1 => {
                if !paused {
                    if let Ok(timeinfo) = system.time.read() {
                        if cache.tsizex > timeinfo.time_string.len() as u16 {
                            draw_time!(stdout, &system, cache);
                        }
                    }
                }
            },
            // Loadavg
            2 => {
                if !paused {
                    if cache.tsizex > 72 && cache.tsizey > 4  {
                        draw_loadavg!(stdout, &system, 57, 0, cache);
                    }
                }
            },
            // CPU stats
            3 => {
                if !paused {
                    if cache.tsizex > 14 && cache.tsizey > 4 {
                        draw_overview!(stdout, &system, 0, 0, cache);
                    }
                }
            },
            // Memory
            4 => {
                if !paused {
                    if cache.tsizex > 34 && cache.tsizey > 4  {
                        draw_memory!(stdout, &system, 17, 0, cache);
                    }
                }
            },
            // Swap
            5 => {
                if !paused {
                    if cache.tsizex > 54 && cache.tsizey > 4  {
                        draw_swap!(stdout, &system, 37, 0, cache);
                    }
                }
            },
            // Sensors
            6 => {
                if !paused {
                    if cache.tsizex > 22 && cache.tsizey > (cache.network_size + cache.sensors_size + 4) as u16 {
                        //let now = std::time::Instant::now();
                        draw_sensors!(stdout, &system, 0, 11, cache);
                        //_draw_benchmark!(stdout, now, tsizex, tsizey);
                    }
                }
            },
            // Network
            7 => {
                if !paused {
                    if cache.tsizex > 23  && cache.tsizey > (cache.network_size + 4) as u16 {
                        let old_size = cache.network_size;
                        draw_network!(stdout, &system, 0, 5, cache);

                        // Redraw full UI if network adapters are added/removed.
                        if old_size != cache.network_size {
                            draw_full_ui(&mut stdout, &system, &mut cache)?;
                        }
                    }
                }
            },
            // Processes
            8 => {
                if !paused {
                    if cache.tsizex > 50 && cache.tsizey > 8 {
                        draw_processes!(stdout, &system, 26, 5, cache);
                    }
                }
            },
            // GPU
            9 => {
                if !paused {
                    if cache.tsizex > 22 && cache.tsizey > (cache.network_size + cache.sensors_size + 10) as u16 {
                        draw_gpu!(stdout, &system, 0, 21, cache);
                    }
                }
            },
            // Distribution + kernel info
            10 => {
                if !paused {
                    if let Ok(timeinfo) = system.time.read() {
                        if cache.tsizex > (timeinfo.time_string.len() + system.hostinfo.distname.len() + system.hostinfo.kernel.len()) as u16 + 9 {
                            draw_hostinfo!(stdout, &system, cache);
                        }
                    }
                }
            }
            // Pause
            101 => {
                if paused {
                    paused = false;
                } else {
                    paused = true;
                }
            },
            // topmode
            102 => {
                if system.config.topmode.load(std::sync::atomic::Ordering::Relaxed) {
                    queue!(stdout,
                        cursor::MoveTo(36, 5),
                        Print("\x1b[38;5;244mt\x1b[0m")
                    )?;
                } else {
                    queue!(stdout,
                        cursor::MoveTo(36, 5),
                        Print(" ")
                    )?;
                }
            },
            // smaps
            103 => {
                if system.config.smaps.load(std::sync::atomic::Ordering::Relaxed) {
                    queue!(stdout,
                        cursor::MoveTo(37, 5),
                        Print("\x1b[38;5;244ms\x1b[0m")
                    )?;
                } else {
                    queue!(stdout,
                        cursor::MoveTo(37, 5),
                        Print(" ")
                    )?;
                }
            },
            // all_processes
            104 => {
                if system.config.all.load(std::sync::atomic::Ordering::Relaxed) {
                    queue!(stdout,
                        cursor::MoveTo(38, 5),
                        Print("\x1b[38;5;244ma\x1b[0m")
                    )?;
                } else {
                    queue!(stdout,
                        cursor::MoveTo(38, 5),
                        Print(" ")
                    )?;
                }
            },
            // resize
            105 => {
                if let Ok(val) = system.events.read() {
                    cache.tsizex = val.tsizex;
                    cache.tsizey = val.tsizey;
                }

                draw_full_ui(&mut stdout, &system, &mut cache)?;
            },

            // Redraw UI if user pressed 'r'
            106 => draw_full_ui(&mut stdout, &system, &mut cache)?,

            // Exit - Someone pressed Q or ctrl+c
            255 => break,

            // If it's something else we better exit just in case!
            _ => break,
        }

        stdout.flush()?;
        //_draw_benchmark!(stdout, now, tsizex, tsizey);
    }

    // Show exit message
    execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(cache.tsizex / 2 - 16, cache.tsizey / 2 - 1),
        Print("Stopping monitoring threads...")
    )?;

    // Stop monitoring threads
    system.stop();

    // Reset terminal
    execute!(stdout, terminal::LeaveAlternateScreen, terminal::EnableLineWrap, cursor::Show)?;

    terminal::disable_raw_mode()?;

    Ok(())
}
