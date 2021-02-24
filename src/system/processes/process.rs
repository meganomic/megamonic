use crate::system::{cpu, Config};
use std::sync::{Arc, RwLock};
use std::io::prelude::*;

#[derive(Default)]
pub struct Process {
    pub cpu_avg: f32,

    pub cmdline: String,
    pub executable: String,

    pub stat_file: String,
    pub statm_file: String,

    // /proc/stat
    pub pid: u32,        // 1
    pub utime: u64,      // 14
    pub stime: u64,      // 15
    pub cutime: u64,     // 16
    pub cstime: u64,     // 17

    // /proc/smaps_rollup
    pub rss: i64,
    pub pss: i64,

    // /proc/task
    //pub tasks : std::collections::HashSet<u32>,

    pub not_executable: bool,

    pub alive: bool,
    pub error: bool,
}

impl Process {
    /*pub fn update_tasks(&mut self) {
        // Can we even read /proc?
        if let Ok(entries) = std::fs::read_dir(format!("/proc/{}/task", self.pid)) {
            self.tasks.clear();
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(dir_name) = entry.file_name().into_string() {
                        // Only directory names made up of numbers will pass
                        if let Ok(pid) = dir_name.parse::<u32>() {
                            self.tasks.insert(pid);
                        }
                    }
                }
            }
        }
    }*/

    pub fn update(&mut self, buffer: &mut String, cpuinfo: &Arc<RwLock<cpu::Cpuinfo>>, config: &Arc<Config>) {
        if let Ok(mut file) = std::fs::File::open(&self.stat_file) {
            file.read_to_string(buffer).unwrap_or_default();

            if self.executable.is_empty() {
                self.not_executable = true;
                self.executable = buffer[buffer.find("(").unwrap_or_default()..buffer.find(")").unwrap_or_default()+1].to_string();
            }

            let old_total = self.utime + self.stime + self.cutime + self.cstime;

            for (i, s) in buffer[buffer.find(")").unwrap_or_default()..buffer.len()].split_whitespace().enumerate() {
                match i {
                    //0 => self.state = s.to_string(),
                    12 => self.utime = s.parse::<u64>().unwrap_or_else(|_| {  self.error = true; 0 }),
                    13 => self.stime = s.parse::<u64>().unwrap_or_else(|_| {  self.error = true; 0 }),
                    14 => self.cutime = s.parse::<u64>().unwrap_or_else(|_| {  self.error = true; 0 }),
                    15 => self.cstime = s.parse::<u64>().unwrap_or_else(|_| {  self.error = true; 0 }),
                    22 => self.rss = s.parse::<i64>().map_or(-1,|v| v * 4096),
                    23 => break,
                    //18 => self.num_threads = s.parse::<u32>().unwrap_or_else(|_| {  self.error = true; 0 }),
                    _ => (),
                }
            }

            if !self.error {
                let total = self.utime + self.stime + self.cutime + self.cstime;

                // If old_total is 0 it means we don't have anything to compare to. So work is 0.
                let work = if old_total == 0 {
                    0
                } else {
                    total - old_total
                };

                if let Ok(val) = cpuinfo.read() {
                    if config.topmode.load(std::sync::atomic::Ordering::Relaxed) {
                        self.cpu_avg = (work as f32 / val.totald as f32) * 100.0 *  val.cpu_count as f32;
                    } else {
                        self.cpu_avg = (work as f32 / val.totald as f32) * 100.0;
                    }
                }
            } else {
                self.cpu_avg = -1.0;
            }

            if config.smaps.load(std::sync::atomic::Ordering::Relaxed) {
                self.update_smaps();
            } /*else {
                buffer.clear();
                self.update_rss(buffer);
            }*/

            //self.update_tasks();
        } else {
            self.alive = false;
        }
    }

    pub fn update_smaps(&mut self) {
        if let Ok(smaps) = std::fs::read_to_string(format!("/proc/{}/smaps_rollup", self.pid)) {
            // RSS
            /*if let Some(line) = smaps.lines().nth(1) {
                if let Some(val) = line.split_whitespace().nth(1) {
                    self.rss = val.parse::<i64>().map_or(-1,|v| v * 1024);
                }
            }*/

            // PSS
            if let Some(line) = smaps.lines().nth(2) {
                if let Some(val) = line.split_whitespace().nth(1) {
                    self.pss = val.parse::<i64>().map_or(-1,|v| v * 1024);
                }
            }
        } else {
            //self.rss = -1;
            self.pss = -1;
        }
    }

    /*pub fn update_rss(&mut self, buffer: &mut String) {
        if let Ok(mut file) = std::fs::File::open(&self.statm_file) {
            file.read_to_string(buffer).unwrap();
        //if let Ok(rss) = std::fs::read_to_string(format!("/proc/{}/status", self.pid)) {
            /*if let Some(line) = buffer.lines().nth(0) {
                self.name = line.split(':').nth(1).unwrap_or("").trim_start().to_string();
            }*/
            if let Some(val) = buffer.split_whitespace().nth(1) {
                //if let Some(val) = line.split_whitespace().nth(1) {
                    self.rss = val.parse::<i64>().map_or(-1,|v| v * 4096);
                //}
            }
        } else {
            self.rss = -1;
        }
    }*/
}
