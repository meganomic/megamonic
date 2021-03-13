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
        .setting(clap::AppSettings::ColoredHelp)
        .about("A badly designed multithreaded system monitor")
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
                .help("Report CPU % the same way top does")
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
                .help("Sample frequency in milliseconds. Min: 1000, Max: 5000")
                .default_value("1000")
        )
        .after_help("\x1b[91mEnabling both smaps and all processes is ultra slow.\nEspecially if running as root.\x1b[0m\n\nThese buttons do things:\nq => exit.\na => toggle all processes.\ns => toggle smaps.\nt => toggle \"Top mode\"\nr => rebuild the UI incase its broken\n[space] => pause the UI.")
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

    // Event channel
    let (tx, rx) = std::sync::mpsc::channel();

    // Start monitoring threads
    system.start(tx.clone());

    // Check if there was any errors starting up
    if !system.error.lock().unwrap().is_empty() {
        system.stop();

        for err in system.error.lock().unwrap().iter() {
            eprintln!("{:?}", err);
        }

        return Ok(());
    }

    // UI cache
    let mut cache = ui::CachedCursor { tsizex, tsizey, ..Default::default() };

    // Used to pause the UI
    let mut paused = false;

    // Disable all hotkeys and stuff. ctrl+c wont work.
    terminal::enable_raw_mode()?;

    // Setup the terminal screen
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::DisableLineWrap,
        cursor::Hide,
    )?;

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
            },
            99 => {
                system.stop();
                execute!(
                    stdout,
                    terminal::Clear(terminal::ClearType::All),
                    cursor::MoveTo(cache.tsizex / 2 - 16, cache.tsizey / 2 - 1),
                    Print("Stopping monitoring threads...")
                )?;
                execute!(stdout, terminal::LeaveAlternateScreen, terminal::EnableLineWrap, cursor::Show)?;
                terminal::disable_raw_mode()?;
                for err in system.error.lock().unwrap().iter() {
                    eprintln!("{:?}", err);
                }

                return Ok(());
            },
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

            // Redraw UI if user pressed r
            106 => draw_full_ui(&mut stdout, &system, &mut cache)?,

            // Exit - Someone pressed Q or ctrl+c
            255 => break,

            // If its something else we better exit just in case!
            _ => break,
        }

        stdout.flush()?;
        //_draw_benchmark!(stdout, now, tsizex, tsizey);
    }

    // Stop monitoring threads
    system.stop();

    // Reset terminal
    execute!(stdout, terminal::LeaveAlternateScreen, terminal::EnableLineWrap, cursor::Show)?;

    terminal::disable_raw_mode()?;

    Ok(())
}
