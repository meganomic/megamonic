use std::sync::{ Arc, mpsc, atomic };
use super::Config;

mod epoll;

pub static mut InputBuffer: String = String::new();

pub fn start_thread(config: Arc<Config>, tx: mpsc::Sender::<u8>) -> std::thread::JoinHandle<()> {
    // Set up the signals for the Event thread
    // This needs to be done in the MAIN thread BEFORE any child threads are spawned
    // so the rules are inherited to all child threads
    let signalfd = epoll::SignalFD::new();

    std::thread::Builder::new().name("Events".to_string()).spawn(move || {
        // Buffer is 10 to make sure stuff fits
        let mut buf = Vec::<u8>::with_capacity(10);

        let mut search = false;

        // Initialize epoll
        let mut epoll = epoll::Epoll::new();

        // Add stdin
        epoll.add(0);

        // Add singalfd
        epoll.add(signalfd.fd);

        loop {
            // Wait for a event
            let event = epoll.wait();

            // This is the fd that caused epoll to wakeup
            let fd = unsafe { event.data.fd };

            // Check which fd contains the event
            if fd == 0 {
                // Stdin event

                buf.clear();

                // Read what's in stdin
                let ret: i32;
                unsafe {
                    asm!("syscall",
                        in("rax") 0, // SYS_READ
                        in("rdi") 0, // STDIN
                        in("rsi") buf.as_mut_ptr(),
                        in("rdx") buf.capacity(),
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


                if search {
                    // Disable Search
                    if buf[0] == b'\r' || buf[0] == 27 {
                        search = false;

                        unsafe {
                            InputBuffer.clear();
                        }

                        // Disable search mode
                        match tx.send(102) {
                            Ok(_) => (),
                            Err(_) => break,
                        }
                    } else {
                        // Delete last char if you press backspace
                        if buf[0] == 127 {
                            unsafe {
                                let _ = InputBuffer.pop();
                            }
                        } else {
                            unsafe {
                                InputBuffer.push(buf[0] as char);
                            }
                        }

                        // Notify UI that string has been updated
                        match tx.send(103) {
                            Ok(_) => (),
                            Err(_) => break,
                        }
                    }
                } else {
                    // Do stuff depending on what button was pressed
                    // I only care about the first byte
                    match buf[0] {
                        // Enable Search
                        b'f' => {
                            search = true;

                            match tx.send(102) {
                                Ok(_) => (),
                                Err(_) => break,
                            }
                        }

                        // Quit
                        b'q' => {
                            match tx.send(255) {
                                Ok(_) => (),
                                Err(_) => break,
                            }

                            break;
                        },

                        // Pause UI
                        b' ' => {
                            match tx.send(101) {
                                Ok(_) => (),
                                Err(_) => break,
                            }
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
                            }
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
                            }
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
                            }
                        },

                        // Rebuild UI cache
                        b'r' => {
                            match tx.send(106) {
                                Ok(_) => (),
                                Err(_) => break,
                            }
                        },

                        _ => (),
                    }
                }
            } else if fd == signalfd.fd {
                // Get what signal was recieved
                let signo = signalfd.read();

                match signo {
                    // SIGWINCH aka terminal resize signal
                    28 => {
                        // Notify main thread about resize
                        match tx.send(105) {
                            Ok(_) => (),
                            Err(_) => break,
                        }
                    },

                    // SIGINT
                    2 => {
                        // Notify main thread about SIGINT
                        match tx.send(255) {
                            Ok(_) => (),
                            Err(_) => break,
                        }
                    },

                    // SIGUSR1 - Used to exit the thread in case of errors
                    10 => break,
                    _ => (),
                }
            } else {
                // Something has gone horrible wrong!

                // Error event
                match tx.send(99) {
                    Ok(_) => (),
                    Err(_) => break,
                }

                break;
            }
        }
    }).expect("Couldn't spawn Events thread")
}
