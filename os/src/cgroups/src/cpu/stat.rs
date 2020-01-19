// Copyright 2019 KTH Royal Institute of Technology. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

// NOTE: Time durations are in microseconds
// May be collected from the cpu.stat file
// https://www.kernel.org/doc/Documentation/cgroup-v2.txt
#[derive(Debug, Serialize)]
pub struct CpuStat {
    usage: u64,
    user: u64,
    system: u64,
    nr_periods: u64,
    nr_throttled: u64,
    throttled: u64,
}

impl CpuStat {
    pub fn new() -> CpuStat {
        CpuStat {
            usage: 0,
            user: 0,
            system: 0,
            nr_periods: 0,
            nr_throttled: 0,
            throttled: 0,
        }
    }
}
