use super::Config;
use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
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

    pub work: u64,
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

    pub fn update(&mut self, buffer: &mut String, config: &Arc<Config>) -> Result<()> {
        if let Ok(mut file) = std::fs::File::open(&self.stat_file) {
            if file.read_to_string(buffer).is_ok() {
                let old_total = self.utime + self.stime + self.cutime + self.cstime;

                let mut split = buffer[
                        buffer.find(")")
                        .ok_or(
                            anyhow!("Can't find ')'")
                            .context("Can't parse /proc/[pid]/stat"))?
                        ..buffer.len()
                    ].split_whitespace();

                self.utime = split.nth(11)
                    .ok_or(anyhow!("Can't parse 'utime' from /proc/[pid]/stat"))?
                    .parse::<u64>()
                    .context("Can't parse 'utime' from /proc/[pid]/stat")?;

                self.stime = split.next()
                    .ok_or(anyhow!("Can't parse 'stime' from /proc/[pid]/stat"))?
                    .parse::<u64>()
                    .context("Can't parse 'stime' from /proc/[pid]/stat")?;

                self.cutime = split.next()
                    .ok_or(anyhow!("Can't parse 'cutime' from /proc/[pid]/stat"))?
                    .parse::<u64>()
                    .context("Can't parse 'cutime' from /proc/[pid]/stat")?;

                self.cstime = split.next()
                    .ok_or(anyhow!("Can't parse 'cstime' from /proc/[pid]/stat"))?
                    .parse::<u64>()
                    .context("Can't parse 'cstime' from /proc/[pid]/stat")?;

                self.rss = split.nth(7)
                    .ok_or(anyhow!("Can't parse 'rss' from /proc/[pid]/stat"))?
                    .parse::<i64>()
                    .context("Can't parse 'rss' from /proc/[pid]/stat")?
                    * 4096;

                if !self.error {
                    let total = self.utime + self.stime + self.cutime + self.cstime;

                    // If old_total is 0 it means we don't have anything to compare to. So work is 0.
                    self.work = if old_total == 0 {
                        0
                    } else {
                        total - old_total
                    };

                } else {
                    self.cpu_avg = -1.0;
                }

                if config.smaps.load(std::sync::atomic::Ordering::Relaxed) {
                    self.update_smaps()?;
                }

                //self.update_tasks();
            } else {
                self.alive = false;
            }
        } else {
            self.alive = false;
        }

        Ok(())
    }

    pub fn update_smaps(&mut self) -> Result<()> {
        if let Ok(smaps) = std::fs::read_to_string(format!("/proc/{}/smaps_rollup", self.pid)) {
            self.pss = smaps.lines()
                .nth(2)
                .ok_or(anyhow!("Can't parse 'pss' from /proc/[pid]/smaps_rollup"))?
                .split_whitespace()
                .nth(1)
                .ok_or(anyhow!("Can't parse 'pss' from /proc/[pid]/smaps_rollup"))?
                .parse::<i64>()
                .context("Can't parse 'pss' from /proc/[pid]/smaps_rollup")?
                * 1024;

        } else {
            //self.rss = -1;
            self.pss = -1;
        }

        Ok(())
    }
}
