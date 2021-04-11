use std::sync::{Arc, Mutex, mpsc, atomic};
use super::Config;
use crossterm::event::{read, poll, Event, KeyCode, KeyModifiers};

#[derive(Default)]
pub struct Events {
    pub tsizex: u16,
    pub tsizey: u16,
}

pub fn start_thread(internal: Arc<Mutex<Events>>, config: Arc<Config>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let (lock, _) = &*exit;
        loop {
            if let Ok(polling) = poll(std::time::Duration::from_millis(1000)) {
                if polling {
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

                                    match tx.send(10) {
                                        Ok(_) => (),
                                        Err(_) => break,
                                    };
                                } else if key.code == KeyCode::Char('s') {
                                    if config.smaps.load(atomic::Ordering::Acquire) {
                                        config.smaps.store(false, atomic::Ordering::Release);
                                    } else {
                                        config.smaps.store(true, atomic::Ordering::Release);
                                    }

                                    match tx.send(11) {
                                        Ok(_) => (),
                                        Err(_) => break,
                                    };
                                } else if key.code == KeyCode::Char('a') {
                                    if config.all.load(atomic::Ordering::Acquire) {
                                        config.all.store(false, atomic::Ordering::Release);
                                    } else {
                                        config.all.store(true, atomic::Ordering::Release);
                                    }

                                    match tx.send(12) {
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
                                if let Ok(mut val) = internal.lock() {
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
                    } else {
                        let _ = tx.send(255);
                        break;
                    }
                } else if let Ok(exitvar) = lock.lock() {
                    if *exitvar {
                        break;
                    }
                } else {
                    let _ = tx.send(99);
                    break;
                }

            } else {
                let _ = tx.send(99);
                break;
            }
        }
    })
}
