use super::Config;
use anyhow::{anyhow, Context, Result};
use std::sync::Arc;
use std::io::prelude::*;

#[derive(Default)]
pub struct Process {
    pub cpu_avg: f32,

    pub cmdline: String,
    pub executable: String,

    stat_file: String,
    smaps_file: String,

    // /proc/stat
    pub pid: u32,        // 1
    utime: u64,      // 14
    stime: u64,      // 15
    cutime: u64,     // 16
    cstime: u64,     // 17

    // /proc/smaps_rollup
    pub rss: i64,
    pub pss: i64,

    pub work: u64,
    pub total: u64,
    // /proc/task
    //pub tasks : std::collections::HashSet<u32>,

    pub not_executable: bool,

    pub alive: bool,
}

impl Process {
    pub fn new(pid: u32, executable: String, cmdline: String, not_executable: bool) -> Self {

        Self {
            pid,
            executable,
            cmdline,
            stat_file: format!("/proc/{}/stat", pid),
            smaps_file: format!("/proc/{}/smaps_rollup", pid),
            alive: true,
            not_executable,
            ..Default::default()
        }
    }

    pub fn update(&mut self, buffer: &mut String, config: &Arc<Config>) -> Result<()> {
        if let Ok(mut file) = std::fs::File::open(&self.stat_file) {
            buffer.clear();
            if file.read_to_string(buffer).is_ok() {
                let old_total = self.total;

                let mut split = buffer[
                        buffer.find(")")
                        .ok_or_else(||
                            anyhow!("Can't find ')'")
                            .context("Can't parse /proc/[pid]/stat"))?
                        ..buffer.len()
                    ].split_ascii_whitespace();

                self.utime = split.nth(11)
                    .ok_or_else(||anyhow!("Can't parse 'utime' from /proc/[pid]/stat"))?
                    .parse::<u64>()
                    .context("Can't parse 'utime' from /proc/[pid]/stat")?;

                self.stime = split.next()
                    .ok_or_else(||anyhow!("Can't parse 'stime' from /proc/[pid]/stat"))?
                    .parse::<u64>()
                    .context("Can't parse 'stime' from /proc/[pid]/stat")?;

                self.cutime = split.next()
                    .ok_or_else(||anyhow!("Can't parse 'cutime' from /proc/[pid]/stat"))?
                    .parse::<u64>()
                    .context("Can't parse 'cutime' from /proc/[pid]/stat")?;

                self.cstime = split.next()
                    .ok_or_else(||anyhow!("Can't parse 'cstime' from /proc/[pid]/stat"))?
                    .parse::<u64>()
                    .context("Can't parse 'cstime' from /proc/[pid]/stat")?;

                self.rss = split.nth(7)
                    .ok_or_else(||anyhow!("Can't parse 'rss' from /proc/[pid]/stat"))?
                    .parse::<i64>()
                    .context("Can't parse 'rss' from /proc/[pid]/stat")?
                    * 4096;

                self.total = self.utime + self.stime + self.cutime + self.cstime;

                // If old_total is 0 it means we don't have anything to compare to. So work is 0.
                self.work = if old_total == 0 {
                    0
                } else {
                    self.total - old_total
                };

                if config.smaps.load(std::sync::atomic::Ordering::Relaxed) {
                    self.update_smaps(buffer)?;
                }

            } else {
                self.alive = false;
            }
        } else {
            self.alive = false;
        }

        Ok(())
    }

    pub fn update_smaps(&mut self, buffer: &mut String) -> Result<()> {
        buffer.clear();
        if let Ok(mut file) = std::fs::File::open(&self.smaps_file) {
            if file.read_to_string(buffer).is_ok() {
                self.pss = buffer.lines()
                    .nth(2)
                    .ok_or_else(||anyhow!("Can't parse 'pss' from /proc/[pid]/smaps_rollup"))?
                    .split_ascii_whitespace()
                    .nth(1)
                    .ok_or_else(||anyhow!("Can't parse 'pss' from /proc/[pid]/smaps_rollup"))?
                    .parse::<i64>()
                    .context("Can't parse 'pss' from /proc/[pid]/smaps_rollup")?
                    * 1024;
            } else {
                self.pss = -1;
            }
        } else {
            self.pss = -1;
        }

        Ok(())
    }
}
