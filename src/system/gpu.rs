//use nvml_wrapper::NVML;

//mod nvidia;

#[derive(Default)]
pub struct Gpu {
    pub temp: u32,
    pub gpu_load: u32,
    pub mem_load: u32,
    pub mem_used: f32,
    pub exit: bool,
}
