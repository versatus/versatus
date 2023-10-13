/// This retrieves stats/metrics from the platform. Primarily stats from the control
/// cgroup (cgroup) of the current process and tree.
use anyhow::{Error, Result};
use std::process;
use std::str;

const LINUX_CGROUP_PATH: &str = "/sys/fs/cgroup";

/// Container struct for all of the cgroup-related stats we gather. Could be extended
/// to include other controllers, such as io later.
#[derive(Clone, Debug)]
pub struct CgroupStats {
    pub cpu: CgroupCpuStats,
    pub mem: CgroupMemStats,
}

/// Stats specific to the CPU utilisation of this cgroup. We'll likely add more to this
/// set when they're present, but this will do for now.
#[derive(Clone, Debug)]
pub struct CgroupCpuStats {
    pub cpu_total_usec: u64,
    pub cpu_system_usec: u64,
    pub cpu_user_usec: u64,
}

#[derive(Clone, Debug)]
pub struct CgroupMemStats {
    pub mem_anon_bytes: u64,
    pub mem_file_bytes: u64,
    pub mem_sock_bytes: u64,
}

impl CgroupStats {
    /// file_to_string retrieves the contents of a file as a string. Primarily useful for small
    /// files that are always known to be a string, such as many of the virtual files found in
    /// /proc and /sys.
    fn file_to_string(path: &str) -> Result<String, Error> {
        let data = std::fs::read(path)?;
        let data_str = match str::from_utf8(&data) {
            Ok(val) => val,
            Err(e) => return Err(e.into()),
        };
        Ok(data_str.to_string())
    }

    /// cgroup returns the control group path of the control group tha the current process
    /// is under.
    fn cgroup() -> Result<String, Error> {
        let pid = process::id();
        let self_path = format!("/proc/{}/cgroup", pid);
        let line = Self::file_to_string(&self_path)?;
        // the cgroup file contains three fields:
        //      hierarchy_id:controller_list:cgroup_path
        // We're interested in the final path, which could also contain a ':'
        let fields: Vec<&str> = line.splitn(3, ':').collect();
        let mut ret = fields[2].to_string();

        // Trim trailing newline
        if ret.ends_with('\n') {
            ret.pop();
        }

        Ok(ret)
    }

    /// Constructor for the stats object.
    pub fn new() -> Result<Self, Error> {
        // Collect CPU stats
        let mut cpu_total_usec: u64 = 0;
        let mut cpu_system_usec: u64 = 0;
        let mut cpu_user_usec: u64 = 0;

        let cpu_stat_file = format!("{}/{}/cpu.stat", LINUX_CGROUP_PATH, Self::cgroup()?);

        // gather up the whole file as a set of lines. It's a 4-line file of key-value pairs.
        let lines: Vec<String> = std::fs::read_to_string(cpu_stat_file)?
            .lines()
            .map(String::from)
            .collect();

        for line in lines {
            let fields = line.split_once(' ');
            if let Some(fields) = fields {
                if fields.0 == "usage_usec" {
                    cpu_total_usec = fields.1.parse::<u64>()?;
                } else if fields.0 == "user_usec" {
                    cpu_user_usec = fields.1.parse::<u64>()?;
                } else if fields.0 == "system_usec" {
                    cpu_system_usec = fields.1.parse::<u64>()?;
                }
            }
        }

        // Collect memory stats
        let mut mem_anon_bytes: u64 = 0;
        let mut mem_file_bytes: u64 = 0;
        let mut mem_sock_bytes: u64 = 0;

        let mem_stat_file = format!("{}/{}/memory.stat", LINUX_CGROUP_PATH, Self::cgroup()?);

        // gather up while file and parse
        let lines: Vec<String> = std::fs::read_to_string(mem_stat_file)?
            .lines()
            .map(String::from)
            .collect();

        for line in lines {
            let fields = line.split_once(' ');
            if let Some(fields) = fields {
                if fields.0 == "anon" {
                    mem_anon_bytes = fields.1.parse::<u64>()?;
                } else if fields.0 == "file" {
                    mem_file_bytes = fields.1.parse::<u64>()?;
                } else if fields.0 == "sock" {
                    mem_sock_bytes = fields.1.parse::<u64>()?;
                }
            }
        }

        // Assemble all stats and return

        let cpu = CgroupCpuStats {
            cpu_total_usec,
            cpu_system_usec,
            cpu_user_usec,
        };

        let mem = CgroupMemStats {
            mem_anon_bytes,
            mem_file_bytes,
            mem_sock_bytes,
        };

        Ok(CgroupStats { cpu, mem })
    }
}
