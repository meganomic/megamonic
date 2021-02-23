use nvml_wrapper::NVML;

#[derive(Default)]
pub struct Nvidia {
    pub temp: u8,
    pub usage: f64,
    pub exit: bool,

    nvml: Option<NVML>,
    device: Vec::<nvml_wrapper::device::Device>,
}


impl Nvidia {
    pub fn init(&mut self) {
        self.nvml = NVML::init().ok();
        self.device = Option::Some(self.nvml.as_ref().unwrap().device_by_index(0).unwrap());

    }

    pub fn update(&mut self) {

    }
}
