// Copyright 2019 KTH Royal Institute of Technology. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

pub mod pressure;
pub mod stat;

//const CPU_STAT: &str = "cpu.stat";
const CPU_WEIGHT: &str = "cpu.weight";
//const CPU_WEIGHT_NICE: &str = "cpu.weight.nice";
//const CPU_MAX: &str = "cpu.max";

use crate::error::CgroupsError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Cpu {
    weight: u32,
}

impl Cpu {
    pub fn get_weight(&self) -> u32 {
        self.weight
    }
    pub fn set_weight(&mut self, cgroup_path: &str, weight: u32) -> Result<(), CgroupsError> {
        // NOTE: weight range check
        // https://www.kernel.org/doc/Documentation/cgroup-v2.txt
        assert!(weight >= 1 && weight <= 10000);

        crate::write_file(cgroup_path, CPU_WEIGHT, &weight.to_string())?;

        // Set new weight
        self.weight = weight;

        Ok(())
    }
}
