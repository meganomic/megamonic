use std::sync::{Arc, RwLock, mpsc, atomic};
use super::Config;
use crossterm::event::{read, Event, KeyCode, KeyModifiers};

#[derive(Default)]
pub struct Events {
    pub tsizex: u16,
    pub tsizey: u16,
}

pub fn start_thread(internal: Arc<RwLock<Events>>, config: Arc<Config>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || loop {
        if let Ok(ev) = read() {
            match ev {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('q') || (key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL) {
                        match tx.send(255) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                        break;
                    } else if key.code == KeyCode::Char(' ') {
                        match tx.send(101) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    } else if key.code == KeyCode::Char('t') {
                        if config.topmode.load(atomic::Ordering::Acquire) {
                            config.topmode.store(false, atomic::Ordering::Release);
                        } else {
                            config.topmode.store(true, atomic::Ordering::Release);
                        }

                        match tx.send(102) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    } else if key.code == KeyCode::Char('s') {
                        if config.smaps.load(atomic::Ordering::Acquire) {
                            config.smaps.store(false, atomic::Ordering::Release);
                        } else {
                            config.smaps.store(true, atomic::Ordering::Release);
                        }

                        match tx.send(103) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    } else if key.code == KeyCode::Char('a') {
                        if config.all.load(atomic::Ordering::Acquire) {
                            config.all.store(false, atomic::Ordering::Release);
                        } else {
                            config.all.store(true, atomic::Ordering::Release);
                        }

                        match tx.send(104) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    } else if key.code == KeyCode::Char('r') {
                        match tx.send(106) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    }
                },
                Event::Resize(width, height) => {
                    if let Ok(mut val) = internal.write() {
                        val.tsizex = width;
                        val.tsizey = height;
                    }

                    match tx.send(105) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                },
                _ => (),
            }
        }
    })
}
