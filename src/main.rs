#![feature(asm)]

use anyhow::{ bail, ensure, Context, Result };
use clap::{ App, AppSettings, Arg, value_t };
use std::sync::atomic;

mod system;

mod ui;

fn main() -> Result<()> {
    let options = App::new("Megamonic")
        .setting(AppSettings::ColoredHelp)
        .about("A badly designed multithreaded system monitor")
        .version("0.1.0")
        .arg(
            Arg::with_name("smaps")
                .short("s")
                .long("enable-smaps")
                .help("Enable use of PSS value instead of RSS for memory reporting. Requires root for some processes (very slow)")
        )
        .arg(
            Arg::with_name("strftime")
                .long("strftime")
                .help("Strftime format string")
                .default_value("%c")
        )
        .arg(
            Arg::with_name("topmode")
                .short("t")
                .long("enable-top-mode")
                .help("Report CPU % the same way top does")
        )
        .arg(
            Arg::with_name("all")
                .short("a")
                .long("enable-all-processes")
                .help("Shows all processes, including kernel threads and other stuff (slow)")
        )
        .arg(
            Arg::with_name("frequency")
                .short("f")
                .long("frequency")
                .help("Sample frequency in milliseconds. Min: 1000, Max: 3000")
                .default_value("1000")
        )
        .after_help("\x1b[91mEnabling both smaps and all processes is ultra slow.\nEspecially if running as root.\x1b[0m\n\nThese buttons do things:\nq => exit.\na => toggle all processes.\ns => toggle smaps.\nt => toggle \"Top mode\"\nr => rebuild the UI incase its broken\n[space] => pause the UI.")
        .get_matches();

    let freq = value_t!(options, "frequency", u64).unwrap_or_else(|e| e.exit());

    ensure!((1000..=3000).contains(&freq), "\x1b[32mFrequency\x1b[0m must in range 1000-3000");

    // Initialize System and set the configuration options
    let mut system = system::System {
        config: std::sync::Arc::new(system::Config {
            smaps: atomic::AtomicBool::new(options.is_present("smaps")),
            topmode: atomic::AtomicBool::new(options.is_present("topmode")),
            all: atomic::AtomicBool::new(options.is_present("all")),
            frequency: atomic::AtomicU64::new(freq),
            strftime_format: value_t!(options, "strftime", String).unwrap_or_else(|e| e.exit()),
        }),
        ..Default::default()
    };

    // Event channel
    let (tx, rx) = std::sync::mpsc::channel();

    // Start monitoring threads
    system.start(tx);

    // Check if there was any errors starting up
    if !system.error.lock().unwrap().is_empty() {
        system.stop();

        for err in system.error.lock().expect("system.error lock couldn't be aquired!").iter() {
            eprintln!("{:?}", err);
        }

        bail!("An error occured while starting!");
    }

    let mut ui = ui::Ui::new(&system)?;

    let mut error: Option<anyhow::Error> = None;

    // Main loop
    for event in rx.iter() {
        match event {
            // Update UI element
            1..=13 => {
                if let Err(err) = ui.update(event).context("Error occured while updating UI") {
                    error = Some(err);
                    break;
                }
            },

            // This is a error event incase one of the threads break.
            99 => {
                ui.exit()?;
                system.stop();

                for err in system.error.lock().expect("system.error lock couldn't be aquired!").iter() {
                    eprintln!("{:?}", err);
                }

                bail!("Error event 99 occured!");
            },

            // Pause
            101 => ui.toggle_pause(),

            // resize
            105 => {
                if let Ok(val) =  system.events.lock() {
                    ui.terminal_size.x = val.tsizex;
                    ui.terminal_size.y = val.tsizey;
                } else {
                    panic!("Event lock is poisoned!");
                }

                if let Err(err) = ui.rebuild() {
                    error = Some(err);
                    break;
                }
            },

            // Rebuild UI if user pressed r
            106 => {
                if let Err(err) = ui.rebuild() {
                    error = Some(err);
                    break;
                }
            },

            // Exit - Someone pressed Q or ctrl+c
            255 => break,

            // If its something else we better exit just in case!
            _ => break,
        }
    }

    ui.exit()?;

    if let Some(err) = error {
        eprintln!("{:#?}", err);
    }

    // Stop monitoring threads
    system.stop();

    Ok(())
}
