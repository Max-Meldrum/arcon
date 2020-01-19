// Copyright 2019 KTH Royal Institute of Technology. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use failure::Fail;

#[derive(Debug, Fail)]
pub enum CgroupsError {
    #[fail(display = "Parse error: {}", msg)]
    ParseError { msg: String },
    #[fail(display = "Failed to write to cgroup with err: {}", msg)]
    WriteError { msg: String },
    #[fail(display = "Failed to read from cgroup with err: {}", msg)]
    ReadError { msg: String },
    #[fail(display = "Bad path: {}", path)]
    BadPath { path: String },
    #[fail(display = "Failed to open file with err: {}", msg)]
    OpenFileError { msg: String },
}
