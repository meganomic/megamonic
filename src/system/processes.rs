mod process;
use crate::system::{cpu, Config};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct Processes {
    pub processes: std::collections::HashMap<u32, process::Process>,
    pub exit: bool,
    pub rebuild: bool,
    ignored: std::collections::HashSet<u32>,
}

impl Processes {
    pub fn update_pids(&mut self, config: &Arc<Config>) {
        // Trigger rebuild if 'show all processes' option is changed
        let all_processes = config.all.load(std::sync::atomic::Ordering::Acquire);
        if all_processes != self.rebuild {
            self.rebuild = all_processes;
            self.processes.clear();
            self.ignored.clear();
        }

        // Can we even read /proc?
        if let Ok(entries) = std::fs::read_dir("/proc/") {
            for entry in entries {
                if let Ok(entry) = &entry {
                    if let Some(dir_name) = &entry.file_name().to_str() {
                        // Only directory names made up of numbers will pass
                        if let Ok(pid) = &dir_name.parse::<u32>() {
                            if !self.ignored.contains(pid) {
                                // Don't add it if we already have it
                                if !self.processes.contains_key(pid) {
                                    if let Ok(cmdline) = std::fs::read_to_string(format!("/proc/{}/cmdline", pid)) {
                                        // Limit the results to actual programs unless 'all-processes' is enabled
                                        // pid == 1 is weird so make an extra check
                                        if !cmdline.is_empty() && *pid != 1 {
                                            let mut executable = String::new();
                                            let mut commandline = String::new();

                                            // Cancer code that is very hacky and don't work for all cases
                                            // For instance, if a directory name has spaces in it, it breaks.
                                            for (idx, val) in cmdline.split(&['\0', ' '][..]).enumerate() {
                                                if idx == 0 {
                                                    if let Some(last) = val.rsplit("/").nth(0) {
                                                        executable.push_str(last);
                                                    } else {
                                                        executable.push_str(val);
                                                    }
                                                } else {
                                                    commandline.push_str(" ");
                                                    commandline.push_str(val);
                                                }
                                            }

                                            self.processes.insert(
                                                *pid,
                                                process::Process {
                                                    pid: *pid,
                                                    executable: executable,
                                                    cmdline: commandline,
                                                    stat_file: format!("/proc/{}/stat", pid),
                                                    statm_file: format!("/proc/{}/statm", pid),
                                                    alive: true,
                                                    ..Default::default()
                                                },
                                            );
                                        } else {
                                            // If 'all-processes' is enabled add everything
                                            if all_processes {
                                                self.processes.insert(
                                                    *pid,
                                                    process::Process {
                                                        pid: *pid,
                                                        executable: String::new(),
                                                        cmdline: String::new(),
                                                        stat_file: format!("/proc/{}/stat", pid),
                                                        statm_file: format!("/proc/{}/statm", pid),
                                                        alive: true,
                                                        ..Default::default()
                                                    },
                                                );
                                            } else {
                                                // Otherwise add it to the ignore list
                                                self.ignored.insert(*pid);
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

        self.processes.retain(|_,v| v.alive);
    }

    pub fn update(&mut self, cpuinfo: &Arc<RwLock<cpu::Cpuinfo>>, config: &Arc<Config>) {
        //let now = std::time::Instant::now();
        self.update_pids(config);
        //eprintln!("{}", now.elapsed().as_micros());

        // Make a buffer here so it doesn't have to allocated over and over again.
        let mut buffer = String::with_capacity(10000);

        for val in self.processes.values_mut() {
            //let now = std::time::Instant::now();
            val.update(&mut buffer, cpuinfo, config);
            //eprintln!("{}", now.elapsed().as_nanos());
            buffer.clear();
        }
    }

    pub fn cpu_sort(&self) -> (usize, Vec::<(u32,&process::Process)>) {
        let mut sorted = Vec::new();
        let mut pidlen = 0;

        for val in self.processes.values() {
            /*
            1 = 10
            2 = 9
            5 = 8
            8 = 7
            12 = 6
            15 = 5
            18 = 4
            22 = 3
            25 = 2
            28 = 1
            */
            // What is the longest PID when converted to a string?
            match val.pid.leading_zeros() {
                0..=1 => if pidlen < 10 { pidlen = 10 },
                2..=4 => if pidlen < 9 { pidlen = 9 },
                5..=7 => if pidlen < 8 { pidlen = 8 },
                8..=11 => if pidlen < 7 { pidlen = 7 },
                12..=14 => if pidlen < 6 { pidlen = 6 },
                15..=17 => if pidlen < 5 { pidlen = 5 },
                18..=21 => if pidlen < 4 { pidlen = 4 },
                22..=24 => if pidlen < 3 { pidlen = 3 },
                25..=27 => if pidlen < 2 { pidlen = 2 },
                28..=32 => if pidlen < 1 { pidlen = 1 },
                _ => pidlen = 10, // This should never happen?
            }

            // Multiply it so it can be sorted
            sorted.push(((val.cpu_avg * 1000.0) as u32, val));
        }

        // Sort by CPU% if the process did work, otherwise sort by Total CPU time
        sorted.sort_by(|(i,a), (z,b)| {
            if z.cmp(i) == std::cmp::Ordering::Equal {
                let at = a.utime + a.stime + a.cutime + a.cstime;
                let bt = b.utime + b.stime + b.cutime + b.cstime;
                bt.cmp(&at)
            } else {
                z.cmp(i)
            }
        });

        (pidlen, sorted)
    }

    // This function is cancer
    /*pub fn cpu_sort_combined(&self, cpu_totald: i64, cpu_count: u8) -> Vec::<(u32,&process::Process)> {
        let mut sorted = Vec::new();

        for process in self.processes.values() {
            let mut work = 0;
            for task in &process.tasks {
                if let Some(shoe) = self.processes.get(task) {
                    let total = shoe.utime + shoe.stime + shoe.cutime + shoe.cstime;
                    let old_total = shoe.old_utime + shoe.old_stime + shoe.old_cutime + shoe.old_cstime;
                    work += total - old_total;
                }
            }

            let cpu_avg = (work as f32 / cpu_totald as f32) * 100.0 *  cpu_count as f32;
            sorted.push(((cpu_avg * 1000.0) as u32, process));
        }


        /*for val in self.processes.values() {
            // Multiply so it sorts properly
            sorted.push(((val.cpu_avg * 1000.0) as u32, val));
        }*/
        let mut bajs = Vec::new();
        sorted.sort_by(|(i,_), (z,_)| i.cmp(z));
        sorted.reverse();
        for (_,val) in &sorted {
            for x in &val.tasks {
                for (_,z) in &sorted {
                    if val.pid != *x {
                        if *x == z.pid {
                            bajs.push(x.clone());

                        }
                    }
                }
            }
        }

        for val in bajs {
            sorted.dedup_by_key(|(_, i)| i.pid == val);
        }
        sorted
    }*/
}
