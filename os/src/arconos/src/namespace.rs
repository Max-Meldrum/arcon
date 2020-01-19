// Copyright (c) 2017, Oracle and/or its affiliates.
// SPDX-License-Identifier: Apache-2.0

// Modifications Copyright 2020 KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use nix::fcntl::{open, OFlag};
use nix::sched::{setns, CloneFlags};
use nix::sys::stat::Mode;
use nix::unistd::close;
use nix::unistd::{Gid, Uid};

use failure::Fail;

#[derive(Debug, Fail)]
#[fail(display = "Namespace error: {}", msg)]
pub struct NamespaceError {
    pub msg: String,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum LinuxNamespaceType {
    Mount = 0x00020000,   /* New mount namespace group */
    Cgroup = 0x02000000,  /* New cgroup namespace */
    Uts = 0x04000000,     /* New utsname namespace */
    Ipc = 0x08000000,     /* New ipc namespace */
    User = 0x10000000,    /* New user namespace */
    Pid = 0x20000000,     /* New pid namespace */
    Network = 0x40000000, /* New network namespace */
}

#[derive(Debug, Clone)]
pub struct LinuxNamespace {
    pub ns_type: LinuxNamespaceType,
    pub path: String,
}

lazy_static! {
    static ref NAMESPACES: Vec<LinuxNamespace> = {
        let mut namespaces = Vec::new();
        namespaces.push(LinuxNamespace {
            ns_type: LinuxNamespaceType::Uts,
            path: "".into(),
        });
        namespaces.push(LinuxNamespace {
            ns_type: LinuxNamespaceType::Pid,
            path: "".into(),
        });
        // TODO: Bring back once I figure out veth interfaces
        /*
        namespaces.push(LinuxNamespace {
            ns_type: LinuxNamespaceType::Network,
            path: "".into(),
        });
        */
        namespaces.push(LinuxNamespace {
            ns_type: LinuxNamespaceType::Mount,
            path: "".into(),
        });
        namespaces
    };
}

pub fn collect() -> Result<(CloneFlags, Vec<(CloneFlags, i32)>), failure::Error> {
    let mut cf = CloneFlags::empty();
    let mut to_enter = Vec::new();
    for ns in &*NAMESPACES {
        let space = CloneFlags::from_bits_truncate(ns.ns_type as i32);
        if ns.path.is_empty() {
            cf |= space;
        } else {
            let fd = open(&*ns.path, OFlag::empty(), Mode::empty())
                .map_err(|e| NamespaceError { msg: e.to_string() })?;
            to_enter.push((space, fd));
        }
    }

    Ok((cf, to_enter))
}

pub fn enter(namespaces: Vec<(CloneFlags, i32)>) -> Result<i32, failure::Error> {
    let mut mount_fd = -1;
    // enter path namespaces
    for &(space, fd) in &namespaces {
        if space == CloneFlags::CLONE_NEWNS {
            // enter mount ns last
            mount_fd = fd;
            continue;
        }
        setns(fd, space).map_err(|_| NamespaceError {
            msg: format!("failed to enter namespace {:?}", space),
        })?;
        close(fd)?;
        if space == CloneFlags::CLONE_NEWUSER {
            crate::setid(Uid::from_raw(0), Gid::from_raw(0)).expect("fix me");
        }
    }

    Ok(mount_fd)
}
