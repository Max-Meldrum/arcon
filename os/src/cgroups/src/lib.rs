// Copyright 2020 KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

pub mod blkio;
pub mod cpu;
pub mod device;
pub mod error;
pub mod memory;
pub mod network;

use error::CgroupsError;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::io::{Read, Write};

const CGROUP_CONTROLLERS: &str = "cgroup.controllers";
const CGROUP_SUBTREE_CONTROL: &str = "cgroup.subtree_control";
const CGROUP_PROCS: &str = "cgroup.procs";
const CGROUP_STAT: &str = "cgroup.stat";

/// Collect cgroup controllers from cgroup.controllers
/// Examples: pids, cpu, rdma, memory
pub fn get_controllers(dir: &str) -> Result<Vec<String>, CgroupsError> {
    let file_path = format!("{}/{}", dir, CGROUP_CONTROLLERS);
    let read = read_string_from(&file_path)?;
    Ok(read.split(" ").map(|s| s.to_string()).collect())
}

/// Collect PIDs from cgroup.procs
pub fn get_procs(dir: &str) -> Result<Vec<u64>, CgroupsError> {
    let file_path = format!("{}/{}", dir, CGROUP_PROCS);
    let read = read_string_from(&file_path)?;
    read.trim()
        .split_whitespace()
        .map(|w| {
            w.parse()
                .map_err(|e: std::num::ParseIntError| CgroupsError::ParseError {
                    msg: e.to_string(),
                })
        })
        .collect()
}

pub fn verify_cgroup_path(path: &str) -> Result<(), CgroupsError> {
    if !path.starts_with("/") {
        return Err(CgroupsError::BadPath {
            path: path.to_string(),
        });
    }
    // TODO: more checks
    Ok(())
}

pub fn read_string_from(path: &str) -> Result<String, CgroupsError> {
    let mut file =
        File::open(path).map_err(|e| CgroupsError::OpenFileError { msg: e.to_string() })?;
    let mut string = String::new();
    file.read_to_string(&mut string)
        .map_err(|e| CgroupsError::ReadError { msg: e.to_string() })?;
    Ok(string.trim().to_string())
}

pub fn read_u64_from(path: &str) -> Result<u64, failure::Error> {
    let mut file =
        File::open(path).map_err(|e| CgroupsError::OpenFileError { msg: e.to_string() })?;
    let mut string = String::new();
    file.read_to_string(&mut string)
        .map_err(|e| CgroupsError::ReadError { msg: e.to_string() })?;
    let parsed: u64 = string
        .trim()
        .parse()
        .map_err(|e: std::num::ParseIntError| CgroupsError::ParseError { msg: e.to_string() })?;
    Ok(parsed)
}

pub fn write_file(dir: &str, file: &str, data: &str) -> Result<(), CgroupsError> {
    let path = format! {"{}/{}", dir, file};
    let mut f =
        File::create(&path).map_err(|e| CgroupsError::OpenFileError { msg: e.to_string() })?;
    f.write_all(data.as_bytes())
        .map_err(|e| CgroupsError::WriteError { msg: e.to_string() })?;
    Ok(())
}
