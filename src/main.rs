use crossterm::{
    cursor, terminal, execute
};

use std::io::{stdout, Write};
use anyhow::Result;

use std::sync::atomic;
mod system;

#[macro_use]
mod ui;

static mut _CUMULATIVE_BENCHMARK: u128 = 0;
static mut _CUMULATIVE_COUNT: u128 = 0;

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

    let mut stdout = stdout();
    //let mut stdout = stdout_l.lock();

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

    let mut ui = ui::Ui::new(&system);

    // Disable all hotkeys and stuff.
    terminal::enable_raw_mode()?;

    // Setup the terminal screen
    execute!(
        stdout,
        terminal::EnterAlternateScreen,
        terminal::Clear(terminal::ClearType::All),
        terminal::DisableLineWrap,
        cursor::Hide,
    )?;

    ui.rebuild()?;

    // Main loop
    for event in rx.iter() {
        match event {
            // Update UI element
            1..=13 => ui.update(event)?,

            // This is a error event incase one of the threads break.
            99 => {
                system.stop();
                execute!(stdout, terminal::Clear(terminal::ClearType::All), terminal::LeaveAlternateScreen, terminal::EnableLineWrap, cursor::Show)?;
                terminal::disable_raw_mode()?;
                for err in system.error.lock().unwrap().iter() {
                    eprintln!("{:?}", err);
                }

                return Ok(());
            },

            // Pause
            101 => ui.toggle_pause(),

            // resize
            105 => {
                if let Ok(val) = system.events.lock() {
                    ui.terminal_size.x = val.tsizex;
                    ui.terminal_size.y = val.tsizey;
                }

                ui.rebuild()?;
            },

            // Rebuild UI if user pressed r
            106 => ui.rebuild()?,

            // Exit - Someone pressed Q or ctrl+c
            255 => break,

            // If its something else we better exit just in case!
            _ => break,
        }

        stdout.flush()?;
    }

    // Stop monitoring threads
    system.stop();

    // Reset terminal
    execute!(stdout, terminal::Clear(terminal::ClearType::All), terminal::LeaveAlternateScreen, terminal::EnableLineWrap, cursor::Show)?;

    terminal::disable_raw_mode()?;

    Ok(())
}
