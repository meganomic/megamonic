#![feature(iter_intersperse)]

use anyhow::{ bail, Context, Result };
use clap::{ Command, Arg, value_parser, ArgAction };
use std::sync::atomic;

mod system;
mod ui;
mod terminal;

use system::System;
use ui::Ui;

// Customized version of https://github.com/sfackler/rust-log-panics
fn custom_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>");

        // clear screen, disable Alternate screen, show cursor
        terminal::disable_custom_mode();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        println!("thread '{}' panicked at '{}', {}", name, msg, info.location().unwrap());

        static FIRST_PANIC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

        match std::env::var_os("RUST_BACKTRACE").is_some() {
            false => {
                if FIRST_PANIC.swap(false, std::sync::atomic::Ordering::SeqCst) {
                    println!("note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n");
                }
            }
            _ => {
                println!("{}", std::backtrace::Backtrace::capture());
            },
        }

        std::process::exit(1);

    }));
}

fn main() -> Result<()> {
    let options = Command::new("Megamonic")
        .about("A badly designed multithreaded system monitor")
        .version(concat!("v1.0.", env!("MEGAMONIC_VER")))
        .arg(
            Arg::new("all")
                .short('a')
                .long("enable-all-processes")
                .help("Shows all processes, including kernel threads and other stuff (slow)")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("smaps")
                .short('s')
                .long("enable-smaps")
                .help("Enable use of PSS value instead of RSS for memory reporting. Requires root for some processes (very slow)")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("topmode")
                .short('t')
                .long("enable-top-mode")
                .help("Report CPU % the same way top does")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("strftime")
                .long("strftime")
                .help("Strftime format string")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .default_value("%c")
        )
        .arg(
            Arg::new("frequency")
                .short('f')
                .long("frequency")
                .help("Sample frequency in milliseconds. Min: 1000, Max: 3000")
                .value_parser(value_parser!(u64).range(1000..=3000))
                .default_value("1000")
        )
        .after_help("\x1b[91mEnabling both smaps and all processes is ultra slow.\nEspecially if running as root.\x1b[0m\n\nThese buttons do things:\nq => exit.\na => toggle all processes.\ns => toggle smaps.\nt => toggle \"Top mode\"\nr => rebuild the UI incase its broken\nf => filter process list. [enter] or [esc] exits filter mode.\n[space] => pause the UI.")
        .get_matches();

    let freq: u64 = *options.get_one("frequency").unwrap();

    let config = system::Config {
        smaps: atomic::AtomicBool::new(options.get_flag("smaps")),
        topmode: atomic::AtomicBool::new(options.get_flag("topmode")),
        all: atomic::AtomicBool::new(options.get_flag("all")),
        frequency: atomic::AtomicU64::new(freq),
        strftime_format: options.get_one::<String>("strftime").unwrap().clone()
    };

    // Event channel
    let (tx, rx) = std::sync::mpsc::channel();

    // Initialize custom panic hook
    custom_panic_hook();

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
