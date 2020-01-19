// Copyright (c) 2017, Oracle and/or its affiliates.
// SPDX-License-Identifier: Apache-2.0

// Modifications Copyright 2020 KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use crate::mount::{bind_dev, mknod_dev};
use crate::Result;
use nix::sys::stat::{umask, Mode};

// a is for LinuxDeviceCgroup
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum LinuxDeviceType {
    b,
    c,
    u,
    p,
    a,
}

impl Default for LinuxDeviceType {
    fn default() -> LinuxDeviceType {
        LinuxDeviceType::a
    }
}

#[derive(Debug, Clone)]
pub struct LinuxDevice {
    pub path: String,
    pub typ: LinuxDeviceType,
    pub major: u64,
    pub minor: u64,
    pub file_mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

lazy_static! {
    static ref DEFAULT_DEVICES: Vec<LinuxDevice> = {
        let mut v = Vec::new();
        v.push(LinuxDevice {
            path: "/dev/null".to_string(),
            typ: LinuxDeviceType::c,
            major: 1,
            minor: 3,
            file_mode: Some(0o066),
            uid: None,
            gid: None,
        });
        v.push(LinuxDevice {
            path: "/dev/zero".to_string(),
            typ: LinuxDeviceType::c,
            major: 1,
            minor: 5,
            file_mode: Some(0o066),
            uid: None,
            gid: None,
        });
        v.push(LinuxDevice {
            path: "/dev/full".to_string(),
            typ: LinuxDeviceType::c,
            major: 1,
            minor: 7,
            file_mode: Some(0o066),
            uid: None,
            gid: None,
        });
        v.push(LinuxDevice {
            path: "/dev/tty".to_string(),
            typ: LinuxDeviceType::c,
            major: 5,
            minor: 0,
            file_mode: Some(0o066),
            uid: None,
            gid: None,
        });
        v.push(LinuxDevice {
            path: "/dev/urandom".to_string(),
            typ: LinuxDeviceType::c,
            major: 1,
            minor: 9,
            file_mode: Some(0o066),
            uid: None,
            gid: None,
        });
        v.push(LinuxDevice {
            path: "/dev/random".to_string(),
            typ: LinuxDeviceType::c,
            major: 1,
            minor: 8,
            file_mode: Some(0o066),
            uid: None,
            gid: None,
        });
        v
    };
}

pub(crate) fn create_devices(bind: bool) -> Result<()> {
    let op: fn(&LinuxDevice) -> Result<()> = if bind { bind_dev } else { mknod_dev };
    let old = umask(Mode::from_bits_truncate(0o000));
    for dev in DEFAULT_DEVICES.iter() {
        op(dev)?;
    }
    for dev in &*DEFAULT_DEVICES {
        if !dev.path.starts_with("/dev") || dev.path.contains("..") {
            let msg = format!("{} is not a valid device path", dev.path);
            bail!(msg);
        }
        op(dev)?;
    }
    umask(old);
    Ok(())
}
