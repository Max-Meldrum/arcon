// Copyright 2019 KTH Royal Institute of Technology. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

// Tracking of pressure stall information (PSI).
// read from the cpu.pressure file
#[derive(Debug, Serialize)]
pub struct CpuPressure {
    /// Elapsed walltime avg over 10 seconds
    avg_ten: f32,
    /// Elapsed walltime avg over 1 min
    avg_sixty: f32,
    /// Elapsed walltime avg over 5 min
    avg_three_hundred: f32,
    /// Total elapsed walltime in microseconds
    total: u64,
}
