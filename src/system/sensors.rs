//#[derive(Default)]
pub struct Sensors {
    pub chips: std::collections::BTreeMap<String, u8>,
    pub exit: bool,
    pub sensors: sensors::Sensors,
}

impl Default for Sensors {
    fn default() -> Self {
        Sensors {
            chips: std::collections::BTreeMap::new(),
            exit: false,
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
