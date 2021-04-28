use std::sync::{Arc, mpsc, atomic};
use super::Config;
//use crossterm::event::{read, Event, KeyCode, KeyModifiers};

mod epoll;

pub fn start_thread(config: Arc<Config>, tx: mpsc::Sender::<u8>) -> std::thread::JoinHandle<()> {
    // Set up the signals for the Event thread
    let signalfd = epoll::SignalFD::new();

    std::thread::Builder::new().name("Events".to_string()).spawn(move || {
        // Buffer is 10 to make sure stuff fits
        let mut buf = Vec::<u8>::with_capacity(10);

        // Initialize epoll
        let mut epoll = epoll::Epoll::new();

        // Add stdin
        epoll.add(0);

        // Add singalfd
        epoll.add(signalfd.fd);

        loop {
            // Wait for a event
            let event = epoll.wait();

            let fd = unsafe { event.data.fd };

            // Check which fd contains the event
            if fd == 0 {
                buf.clear();

                // Read what's in stdin
                let ret: i32;
                unsafe {
                    asm!("syscall",
                        in("rax") 0, // SYS_READ
                        in("rdi") 0,
                        in("rsi") buf.as_mut_ptr(),
                        in("rdx") 10,
                        out("rcx") _,
                        out("r11") _,
                        lateout("rax") ret,
                    );
                }

                assert!(!ret.is_negative());

                // Set buffer length to however many bytes was read
                unsafe {
                    buf.set_len(ret as usize);
                }

                // Do stuff depending on what button was pressed
                // I only care about the first byte
                match buf[0] {
                    // Quit
                    b'q' => {
                        match tx.send(255) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                        break;
                    },

                    // Pause UI
                    b' ' => {
                        match tx.send(101) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    },

                    // Toggle Topmode
                    b't' => {
                        if config.topmode.load(atomic::Ordering::Acquire) {
                            config.topmode.store(false, atomic::Ordering::Release);
                        } else {
                            config.topmode.store(true, atomic::Ordering::Release);
                        }

                        match tx.send(10) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    },

                    // Toggle smaps
                    b's' => {
                        if config.smaps.load(atomic::Ordering::Acquire) {
                            config.smaps.store(false, atomic::Ordering::Release);
                        } else {
                            config.smaps.store(true, atomic::Ordering::Release);
                        }

                        match tx.send(11) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    },

                    // Toggle All Processes
                    b'a' => {
                        if config.all.load(atomic::Ordering::Acquire) {
                            config.all.store(false, atomic::Ordering::Release);
                        } else {
                            config.all.store(true, atomic::Ordering::Release);
                        }

                        match tx.send(12) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    },

                    // Rebuild UI cache
                    b'r' => {
                        match tx.send(106) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                    },

                    _ => (),
                }

            } else if fd == signalfd.fd {
                // This is only triggered if the terminal is resized

                // Buffer to hold the signal data
                let mut data = epoll::SignalfdSiginfo::default();

                // Read signal info from signalfd
                // I don't need it but I read it anyway to clear the buffer
                // Not sure if it's needed
                let ret: i32;
                unsafe {
                    asm!("syscall",
                        in("rax") 0, // SYS_READ
                        in("rdi") signalfd.fd,
                        in("rsi") &mut data as *mut epoll::SignalfdSiginfo,
                        in("rdx") 128, //std::mem::size_of_val(&data),
                        out("rcx") _,
                        out("r11") _,
                        lateout("rax") ret,
                    );
                }

                assert!(!ret.is_negative());

                // Notify main thread about resize
                match tx.send(105) {
                    Ok(_) => (),
                    Err(_) => break,
                };

            } else {
                // Something has gone horrible wrong!

                // Error event
                match tx.send(99) {
                    Ok(_) => (),
                    Err(_) => break,
                };
                break;
            }




            /*let ret: i32;
            unsafe {
                asm!("syscall",
                    in("rax") 0, // SYS_READ
                    in("rdi") 0,
                    in("rsi") buf.as_ptr(),
                    in("rdx") 10,
                    out("rcx") _,
                    out("r11") _,
                    lateout("rax") ret,
                );
            }

            assert!(!ret.is_negative());

            unsafe {
                buf.set_len(ret as usize);
            }

            match buf[0] {
                b'q' => {
                        match tx.send(255) {
                            Ok(_) => (),
                            Err(_) => break,
                        };
                        break;
                }

                /*Event::Resize(width, height) => {
                    if let Ok(mut val) = internal.lock() {
                        val.tsizex = width;
                        val.tsizey = height;
                    }

                    match tx.send(105) {
                        Ok(_) => (),
                        Err(_) => break,
                    };
                },*/
                _ => (),
            }*/

            /*if let Ok(ev) = read() {
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
                let _ = tx.send(99);
                break;
            }*/
        }

        epoll.close();
        signalfd.close();

        // Close signalfd
        /*let ret: i32;
        unsafe {
            asm!("syscall",
                in("rax") 3, // SYS_CLOSE
                in("rdi") signal.fd,
                out("rcx") _,
                out("r11") _,
                lateout("rax") ret,
            );
        }

        assert!(!ret.is_negative());*/
    }).expect("Couldn't spawn Events thread")
}
