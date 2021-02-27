#[derive(Default)]
pub struct Bandwidth {
    pub recv: i64,
    pub sent: i64,
    pub total_recv: i64,
    pub total_sent: i64,
}

#[derive(Default)]
pub struct Network {
    pub stats: std::collections::BTreeMap<String, Bandwidth>,
}

impl Network {
    pub fn update(&mut self) {
        if let Ok(networkinfo) = std::fs::read_to_string("/proc/net/dev") {
            for (idx, line) in networkinfo.lines().enumerate() {
                if idx >= 2 {
                    let mut bandwidth = Bandwidth::default();
                    let mut name = String::default();

                    for (i, s) in line.split_whitespace().enumerate() {
                        match i {
                            0 => name = s.to_string(),
                            1 => bandwidth.total_recv = s.parse::<i64>().unwrap_or(-1),
                            9 => { bandwidth.total_sent = s.parse::<i64>().unwrap_or(-1); break; },
                            _ => (),
                        }
                    }

                    // If it hasn't sent or recieved anything it's probably off so don't add it.
                    if bandwidth.total_recv != 0 || bandwidth.total_sent != 0 {
                        match self.stats.get_mut(name.as_str()) {
                            Some(bw) => {
                                // It already exists. Update the values.
                                bw.recv = bandwidth.total_recv - bw.total_recv;
                                bw.sent = bandwidth.total_sent - bw.total_sent;
                                bw.total_recv = bandwidth.total_recv;
                                bw.total_sent = bandwidth.total_sent;
                            },
                            None => {
                                // If it didn't already exist add it
                                self.stats.insert(name, bandwidth);
                            }
                        }
                    }
                }
            }
        } else {
            // Can't read the proc file. Notify user.
            self.stats.clear();
            self.stats.insert(String::from("Error"), Bandwidth{recv: -1, sent: -1, total_recv: -1, total_sent: -1});
        }
    }
}
