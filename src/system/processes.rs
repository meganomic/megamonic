mod process;
use super::{cpu, Config};
use anyhow::{bail, anyhow, Context, Result};
use std::sync::{Arc, Mutex, mpsc};
use std::io::prelude::*;
use std::fmt::Write as fmtWrite;

#[derive(Default)]
pub struct Processes {
    pub processes: std::collections::HashMap<u32, process::Process>,
    pub rebuild: bool,
    buffer: String,
    buffer2: String,
    ignored: std::collections::HashSet<u32>,
}

impl Processes {
    pub fn update_pids(&mut self, config: &Arc<Config>) -> Result<()> {
        // Trigger rebuild if 'show all processes' option is changed
        let all_processes = config.all.load(std::sync::atomic::Ordering::Relaxed);
        if all_processes != self.rebuild {
            self.rebuild = all_processes;
            self.processes.clear();
            self.ignored.clear();
        }

        let entries = std::fs::read_dir("/proc/").context("Can't read /proc/")?;

        for entry in entries {
            // Only directory names made up of numbers will pass
            if let Ok(pid) = entry
                .context("IO error while reading /proc/")?
                .file_name()
                .to_str()
                .ok_or_else(||anyhow!("Entry in /proc/ contains illegal unicode"))?
                .parse::<u32>()
            {
                if !self.ignored.contains(&pid) {
                    // Don't add it if we already have it
                    if !self.processes.contains_key(&pid) {
                        // If cmdline can't be opened it probably means that the process has terminated, skip it.
                        self.buffer2.clear();
                        write!(&mut self.buffer2, "/proc/{}/cmdline", pid)?;
                        if let Ok(mut f) = std::fs::File::open(&self.buffer2) {
                            self.buffer.clear();
                            f.read_to_string(&mut self.buffer).with_context(|| format!("/proc/{}/cmdline", pid))?;
                        } else {
                            continue
                        };

                        // Limit the results to actual programs unless 'all-processes' is enabled
                        // pid == 1 is weird so make an extra check
                        if !self.buffer.is_empty() && pid != 1 {
                            // Cancer code that is very hacky and don't work for all cases
                            // For instance, if a directory name has spaces or slashes in it, it breaks.
                            let mut split = self.buffer.split(&['\0', ' '][..]);
                            let executable = split.next()
                                .ok_or_else(||anyhow!("Parsing error in /proc/[pid]/cmdline"))?
                                .rsplit("/")
                                .next()
                                .ok_or_else(||anyhow!("Parsing error in /proc/[pid]/cmdline"))?
                                .to_string();

                            let cmdline = split
                                .fold(
                                    String::new(),
                                    |mut o, i|
                                    {
                                        o.push(' ');
                                        o.push_str(i);
                                        o
                                    }
                                );

                            self.processes.insert(
                                pid,
                                process::Process::new(
                                    pid,
                                    executable,
                                    cmdline,
                                    false
                                )
                            );
                        } else {
                            // If 'all-processes' is enabled add everything
                            if all_processes {
                                // If stat can't be opened it means the process has terminated, skip it.
                                self.buffer2.clear();
                                write!(&mut self.buffer2, "/proc/{}/stat", pid)?;
                                let executable = if let Ok(mut f) = std::fs::File::open(&self.buffer2) {
                                    self.buffer.clear();
                                    f.read_to_string(&mut self.buffer).with_context(|| format!("/proc/{}/stat", pid))?;
                                    self.buffer[
                                        self.buffer.find("(")
                                        .ok_or_else(||
                                            anyhow!("Can't find '('")
                                            .context("Can't parse /proc/[pid]/stat"))?
                                        ..self.buffer.find(")")
                                        .ok_or_else(||
                                            anyhow!("Can't find ')'")
                                            .context("Can't parse /proc/[pid]/stat"))?+1
                                    ].to_string()
                                } else {
                                    continue
                                };

                                self.processes.insert(
                                    pid,
                                    process::Process::new(
                                        pid,
                                        executable,
                                        String::new(),
                                        true
                                    )
                                );
                            } else {
                                // Otherwise add it to the ignore list
                                self.ignored.insert(pid);
                            }
                        }
                    }
                }
            }
        }

        self.processes.retain(|_,v| v.alive);

        Ok(())
    }

    pub fn update(&mut self, cpuinfo: &Arc<Mutex<cpu::Cpuinfo>>, config: &Arc<Config>) -> Result<()> {
        //let now = std::time::Instant::now();
        self.update_pids(config)?;
        //eprintln!("{}", now.elapsed().as_nanos());

        let (cpu_count, totald) = if let Ok(cpu) = cpuinfo.lock() {
            (cpu.cpu_count as f32, cpu.totald)
        } else {
            bail!("Cpuinfo lock is poisoned!");
        };

        let topmode = config.topmode.load(std::sync::atomic::Ordering::Relaxed);
        let smaps = config.smaps.load(std::sync::atomic::Ordering::Relaxed);

        for val in self.processes.values_mut() {
            //let now = std::time::Instant::now();
            val.update(&mut self.buffer, smaps)?;
            //eprintln!("{}", now.elapsed().as_nanos());

            if topmode {
                if val.work > totald {
                    val.cpu_avg = 100.0 * cpu_count;
                } else {
                    val.cpu_avg = (val.work as f32 / totald as f32) * 100.0 *  cpu_count;
                }
            } else {
                if val.work > totald {
                    val.cpu_avg = 100.0;
                } else {
                    val.cpu_avg = (val.work as f32 / totald as f32) * 100.0;
                }
            }
        }

        //eprintln!("{}", now.elapsed().as_nanos());
        Ok(())
    }

    pub fn cpu_sort(&self) -> (usize, Vec::<&process::Process>) {
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
                28..=31 => if pidlen < 1 { pidlen = 1 },
                _ => pidlen = 10, // This should never happen
            }

            // Multiply it so it can be sorted
            sorted.push(val);
        }

        // Sort by amount of Work, if equal sort by Total Work
        sorted.sort_by(|a, b| {
            let comparison = b.work.cmp(&a.work);
            if comparison == std::cmp::Ordering::Equal {
                b.total.cmp(&a.total)
            } else {
                comparison
            }
        });

        (pidlen, sorted)
    }
}

pub fn start_thread(internal: Arc<Mutex<Processes>>, cpuinfo: Arc<Mutex<cpu::Cpuinfo>>, config: Arc<Config>, tx: mpsc::Sender::<u8>, exit: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>, error: Arc<Mutex<Vec::<anyhow::Error>>>, sleepy: std::time::Duration) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let (lock, cvar) = &*exit;
        'outer: loop {
            match internal.lock() {
                Ok(mut val) => {
                    if let Err(err) = val.update(&cpuinfo, &config) {
                        let mut errvec = error.lock().expect("Error lock couldn't be aquired!");
                        errvec.push(err);

                        match tx.send(99) {
                            Ok(_) => (),
                            Err(_) => break,
                        }

                        break;
                    }
                },
                Err(_) => break,
            }

            match tx.send(8) {
                Ok(_) => (),
                Err(_) => break,
            }

            if let Ok(mut exitvar) = lock.lock() {
                loop {
                    if let Ok(result) = cvar.wait_timeout(exitvar, sleepy) {
                        exitvar = result.0;

                        if *exitvar == true {
                            break 'outer;
                        }

                        if result.1.timed_out() == true {
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
    })
}
