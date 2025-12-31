//! Utility code shared across check.rs and generate.rs

#[cfg(not(test))]
use crate::config::Service;
#[cfg(test)]
use connate::config::Service;

use std::collections::HashMap;
#[cfg(feature = "host-checks")]
use std::fs::read_to_string;

pub fn get_uid_map() -> HashMap<String, u32> {
    #[cfg(feature = "host-checks")]
    {
        let mut uid_map = HashMap::new();
        let passwd = read_to_string("/etc/passwd").expect("Failed to read /etc/passwd");
        for line in passwd.lines() {
            if line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 3 {
                continue;
            }
            let name = parts[0];
            let uid = parts[2].parse().expect("Failed to parse uid");
            uid_map.insert(name.to_string(), uid);
        }
        uid_map
    }
    #[cfg(not(feature = "host-checks"))]
    {
        HashMap::new()
    }
}

pub fn get_gid_map() -> HashMap<String, u32> {
    #[cfg(feature = "host-checks")]
    {
        let mut gid_map = HashMap::new();
        let group = read_to_string("/etc/group").expect("Failed to read /etc/group");
        for line in group.lines() {
            if line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 3 {
                continue;
            }
            let name = parts[0];
            let gid = parts[2].parse().expect("Failed to parse gid");
            gid_map.insert(name.to_string(), gid);
        }
        gid_map
    }
    #[cfg(not(feature = "host-checks"))]
    {
        HashMap::new()
    }
}

pub fn get_svc_map(svcs: &[Service]) -> HashMap<&'static str, &Service> {
    let mut svc_map = HashMap::new();
    for svc in svcs {
        let _ = svc_map.insert(svc.name, svc);
    }
    svc_map
}

#[cfg_attr(test, allow(unused))]
pub fn get_svc_index_map(svcs: &[Service]) -> HashMap<&'static str, usize> {
    let mut svc_map = HashMap::new();
    for (i, svc) in svcs.iter().enumerate() {
        let _ = svc_map.insert(svc.name, i);
    }
    svc_map
}
