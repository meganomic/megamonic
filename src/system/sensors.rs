use std::sync::{Arc, RwLock, mpsc};

pub struct Sensors {
    pub chips: std::collections::BTreeMap<String, u8>,
    pub sensors: sensors::Sensors,
}

impl Default for Sensors {
    fn default() -> Self {
        Sensors {
            chips: std::collections::BTreeMap::new(),
            sensors: sensors::Sensors::new(),
        }
    }
}

impl Sensors {
    pub fn update(&mut self) {
        // Replace with manual parsing
        for chip in self.sensors {
            if let Ok(c_name) = chip.get_name() {
                for feature in chip {
                    if let Ok(f_name) = feature.get_label() {
                        for subfeature in feature {
                            let sf_name = subfeature.name(); // Format: tempX_Y, we want tempX_input

                            if let Ok(val) = subfeature.get_value() {
                                // We only want tempX_input
                                if sf_name.ends_with("input") {
                                    // If the name is temp1 that means it doesn't have a f_name
                                    // So use the chip name instead
                                    if f_name == "temp1" {
                                        self.chips.insert(c_name.clone(), val.round() as u8);
                                    }
                                    else {
                                        self.chips.insert(f_name.clone(), val.round() as u8);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn start_thread(internal: Arc<RwLock<Sensors>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || 'outer: loop {
        match internal.write() {
            Ok(mut val) => {
                val.update();
            },
            Err(_) => break,
        };
        match tx.send(6) {
            Ok(_) => (),
            Err(_) => break,
        };

        let (lock, cvar) = &*exit;
        if let Ok(mut exitvar) = lock.lock() {
            loop {
                if let Ok(result) = cvar.wait_timeout(exitvar, sleepy) {
                    exitvar = result.0;

                    if *exitvar == true {
                        break 'outer;
                    }

                    if result.1.timed_out() == true {
                        break;
                    }
                } else {
                    break 'outer;
                }
            }
        } else {
            break;
        }
    })
}
