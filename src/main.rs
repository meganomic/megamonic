#![feature(asm)]
#![feature(iter_intersperse)]
#![feature(backtrace)]

use anyhow::{ bail, ensure, Context, Result };
use clap::{ App, AppSettings, Arg, value_t };
use std::sync::atomic;

mod system;
mod ui;
mod terminal;

use system::System;
use ui::Ui;

fn main() -> Result<()> {
    let options = App::new("Megamonic")
        .setting(AppSettings::ColoredHelp)
        .about("A badly designed multithreaded system monitor")
        .version(concat!("v0.1.", env!("MEGAMONIC_VER")))
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
        .after_help("\x1b[91mEnabling both smaps and all processes is ultra slow.\nEspecially if running as root.\x1b[0m\n\nThese buttons do things:\nq => exit.\na => toggle all processes.\ns => toggle smaps.\nt => toggle \"Top mode\"\nr => rebuild the UI incase its broken\nf => filter process list. [enter] or [esc] exits filter mode.\n[space] => pause the UI.")
        .get_matches();

    let freq = value_t!(options, "frequency", u64).unwrap_or_else(|e| e.exit());

    ensure!((1000..=3000).contains(&freq), "\x1b[32mFrequency\x1b[0m must in range 1000-3000");

    let config = system::Config {
        smaps: atomic::AtomicBool::new(options.is_present("smaps")),
        topmode: atomic::AtomicBool::new(options.is_present("topmode")),
        all: atomic::AtomicBool::new(options.is_present("all")),
        frequency: atomic::AtomicU64::new(freq),
        strftime_format: value_t!(options, "strftime", String).unwrap_or_else(|e| e.exit()),
    };

    // Event channel
    let (tx, rx) = std::sync::mpsc::channel();

    let system = System::new(config, tx)?;

    // Check if there was any errors starting up
    if !system.error.lock().unwrap().is_empty() {
        for err in system.error.lock().expect("system.error lock couldn't be aquired!").iter() {
            eprintln!("{:?}", err);
        }

        bail!("An error occured while starting!");
    }

    let mut ui = Ui::new(&system, terminal::gettermsize())?;

    // Main loop
    for event in rx.iter() {
        match event {
            // Update UI element
            1..=13 => {
                if let Err(err) = ui.update(event).context("Error occured while updating UI") {
                    ui.set_error(err);

                    break;
                }
            },

            // This is a error event incase one of the threads break.
            99 => {
                if let Some(err) = system.error.lock().expect("system.error lock couldn't be aquired!").pop() {
                    ui.set_error(err);
                }

                bail!("Error event 99 occured!");
            },

            // Pause
            101 => ui.toggle_pause(),

            // resize
            105 => {
                let (x, y) = terminal::gettermsize();
                ui.terminal_size.x = x;
                ui.terminal_size.y = y;

                if let Err(err) = ui.rebuild() {
                    ui.set_error(err);

                    break;
                }
            },

            // Rebuild UI if user pressed r
            106 => {
                if let Err(err) = ui.rebuild() {
                    ui.set_error(err);

                    break;
                }
            },

            // Exit - Someone pressed Q or a SIGINT was caught
            255 => break,

            // If its something else we better exit just in case!
            _ => break,
        }
    }

    Ok(())
}
