use std::sync::{Arc, Mutex, mpsc};

//use nvml_wrapper::NVML;

//mod nvidia;

#[derive(Default)]
pub struct Gpu {
    pub temp: u32,
    pub gpu_load: u32,
    pub mem_load: u32,
    pub mem_used: f32,

}

pub fn start_thread(internal: Arc<Mutex<Gpu>>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new().name("Gpu".to_string()).spawn(move || {
        // Setup device
        if let Ok(nvml) = nvml_wrapper::NVML::init() {
            if let Ok(device) = nvml.device_by_index(0) {
                let (lock, cvar) = &*exit;
                'outer: loop {
                    match internal.lock() {
                        Ok(mut val) => {
                            if let Ok(temp) = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu) {
                                val.temp = temp;
                            }
                            if let Ok(util) = device.utilization_rates() {
                                val.gpu_load = util.gpu;
                                val.mem_load = util.memory;
                            }
                            if let Ok(mem) = device.memory_info() {
                                val.mem_used = (mem.used as f32 / mem.total as f32) * 100.0;
                            }
                        },
                        Err(_) => break,
                    }

                    match tx.send(9) {
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
            }
        }
    }).expect("Couldn't spawn Processes thread")
}
