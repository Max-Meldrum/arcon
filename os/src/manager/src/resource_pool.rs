// Copyright 2019 KTH Royal Institute of Technology. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

/// Pool of Available Resources
#[derive(Debug, Serialize)]
pub struct ResourcePool {
    /// Available memory in bytes
    memory: u64,
    /// Available Virtual CPUs
    vcpu: u32,
}

impl ResourcePool {
    pub fn new(memory: u64, vcpu: u32) -> ResourcePool {
        ResourcePool { memory, vcpu }
    }
}
