// Copyright 2019 KTH Royal Institute of Technology. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use cgroups::{cpu::Cpu, memory::Memory};
use serde::Serialize;

#[derive(Debug, Serialize)]
enum JobVariant {
    Stream,
    Batch,
}

#[derive(Debug, Serialize)]
pub struct Job {
    id: String,
    variant: JobVariant,
    cpu: Cpu,
    memory: Memory,
}
