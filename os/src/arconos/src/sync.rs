// Copyright (c) 2017, Oracle and/or its affiliates.
// SPDX-License-Identifier: Apache-2.0

// Modifications Copyright 2020 KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use crate::Result;
use nix::fcntl::OFlag;
use nix::unistd::{close, pipe2, read};
use std::os::unix::io::RawFd;

pub struct Cond {
    rfd: RawFd,
    wfd: RawFd,
}

impl Cond {
    pub fn new() -> Result<Cond> {
        let (rfd, wfd) = pipe2(OFlag::O_CLOEXEC)?;
        Ok(Cond { rfd: rfd, wfd: wfd })
    }

    pub fn wait(&self) -> Result<()> {
        close(self.wfd)?;
        let data: &mut [u8] = &mut [0];
        while read(self.rfd, data)? != 0 {}
        close(self.rfd)?;
        Ok(())
    }
    pub fn notify(&self) -> Result<()> {
        close(self.rfd)?;
        close(self.wfd)?;
        Ok(())
    }
}
