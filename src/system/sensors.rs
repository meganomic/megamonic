use std::sync::{Arc, Mutex, mpsc};

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
        // Don't handle any errors, just skip that sensor.
        for chip in self.sensors {
            if let Ok(mut c_name) = chip.get_name() {
                for feature in chip {
                    if let Ok(mut f_name) = feature.get_label() {
                        if let Some(subfeature) = feature.get_subfeature(sensors::SubfeatureType::SENSORS_SUBFEATURE_TEMP_INPUT) {
                            if let Ok(val) = subfeature.get_value() {
                                // If the name is temp1 that means it doesn't have a f_name
                                // So use the chip name instead
                                if f_name == "temp1" {
                                    c_name.truncate(14);
                                    self.chips.insert(c_name.clone(), val.round() as u8);
                                }
                                else {
                                    f_name.truncate(14);
                                    self.chips.insert(f_name, val.round() as u8);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn start_thread(internal: Arc<Mutex<Sensors>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new().name("Sensors".to_string()).spawn(move || {
        let (lock, cvar) = &*exit;
        'outer: loop {
            match internal.lock() {
                Ok(mut val) => val.update(),
                Err(_) => break,
            }

            match tx.send(6) {
                Ok(_) => (),
                Err(_) => break,
            }

            if let Ok(mut exitvar) = lock.lock() {
                loop {
                    if let Ok(result) = cvar.wait_timeout(exitvar, sleepy) {
                        exitvar = result.0;

                        if *exitvar {
                            break 'outer;
                        }

                        if result.1.timed_out() {
                            break;
                        }
                    } else {
                        break 'outer;
                    }
                }
            } else {
                break;
            }
        }
    }).expect("Couldn't spawn Sensors thread")
}
