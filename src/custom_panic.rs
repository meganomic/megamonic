// Customized version of https://github.com/sfackler/rust-log-panics
use crossterm::{ terminal, execute, cursor };
use std::{ panic, thread };
use backtrace::Backtrace;

pub fn init() {
    panic::set_hook(Box::new(|info| {
        let backtrace = Backtrace::default();

        let thread = thread::current();
        let thread = thread.name().unwrap_or("<unnamed>");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };

        let _ = execute!(std::io::stdout(),
                    terminal::Clear(terminal::ClearType::All),
                    terminal::LeaveAlternateScreen,
                    terminal::EnableLineWrap,
                    cursor::Show
                );

        let _ = terminal::disable_raw_mode();

        match info.location() {
            Some(location) => {
                println!(
                    "panic thread '{}' panicked at '{}': {}:{}{:?}",
                    thread,
                    msg,
                    location.file(),
                    location.line(),
                    backtrace
                );
            }
            None => println!(
                "panic thread '{}' panicked at '{}'{:?}",
                thread,
                msg,
                backtrace
            ),
        }
    }));
}
