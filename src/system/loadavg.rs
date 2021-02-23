#[derive(Default)]
pub struct Loadavg {
    pub min1: String,
    pub min5: String,
    pub min15: String,
    pub exit: bool
}

impl Loadavg {
    pub fn update(&mut self) {
        if let Ok(procloadavg) = std::fs::read_to_string("/proc/loadavg") {
            self.min1.clear();
            self.min5.clear();
            self.min15.clear();


            for (i, s) in procloadavg.split_whitespace().enumerate() {
                match i {
                    0 => self.min1.push_str(s),
                    1 => self.min5.push_str(s),
                    2 => { self.min15.push_str(s); break; },
                    _ => (),
                }
            }
        } else {
            self.min1.push_str("Error");
            self.min5.push_str("Error");
            self.min15.push_str("Error");
        }
    }
}
