// Copyright (c) 2017, Oracle and/or its affiliates.
// SPDX-License-Identifier: Apache-2.0

// Modifications Copyright 2020 KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum LinuxRlimitType {
    RLIMIT_CPU,        // CPU time in sec
    RLIMIT_FSIZE,      // Maximum filesize
    RLIMIT_DATA,       // max data size
    RLIMIT_STACK,      // max stack size
    RLIMIT_CORE,       // max core file size
    RLIMIT_RSS,        // max resident set size
    RLIMIT_NPROC,      // max number of processes
    RLIMIT_NOFILE,     // max number of open files
    RLIMIT_MEMLOCK,    // max locked-in-memory address space
    RLIMIT_AS,         // address space limit
    RLIMIT_LOCKS,      // maximum file locks held
    RLIMIT_SIGPENDING, // max number of pending signals
    RLIMIT_MSGQUEUE,   // maximum bytes in POSIX mqueues
    RLIMIT_NICE,       // max nice prio allowed to raise to
    RLIMIT_RTPRIO,     // maximum realtime priority
    RLIMIT_RTTIME,     // timeout for RT tasks in us
}

#[derive(Debug, Clone)]
pub struct LinuxRlimit {
    pub typ: LinuxRlimitType,
    pub hard: u64,
    pub soft: u64,
}
