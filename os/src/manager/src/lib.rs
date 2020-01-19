// Copyright 2019 KTH Royal Institute of Technology. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate log;

/// Public Interface
pub mod api_server;

/// Private Interface
mod error;
mod job;
mod resource_pool;
mod scheduler;

use error::ManagerError;
use job::Job;
use nix::unistd::Pid;
use resource_pool::ResourcePool;
use spec::Specification;
use std::collections::HashMap;

pub struct Manager {
    /// Dragonslayer Specification
    spec: Specification,
    /// Available Resources: Memory, CPU, Devices etc...
    resource_pool: ResourcePool,
    /// A Vector of active jobs
    jobs: Vec<Job>,
    jobs_map: HashMap<Pid, Job>,
}

impl Manager {
    pub fn new(spec: Specification) -> Manager {
        let resource_pool = ResourcePool::new(spec.resources.mem, spec.resources.vcpu);
        Manager {
            spec,
            resource_pool,
            jobs: Vec::new(),
            jobs_map: HashMap::default(),
        }
    }

    pub fn get_spec_json(&self) -> Result<String, ManagerError> {
        serde_json::to_string(&self.spec)
            .map_err(|e| ManagerError::JsonParseError { msg: e.to_string() })
    }

    pub fn get_jobs_json(&self) -> Result<String, ManagerError> {
        serde_json::to_string(&self.jobs)
            .map_err(|e| ManagerError::JsonParseError { msg: e.to_string() })
    }

    pub fn get_resource_pool_json(&self) -> Result<String, ManagerError> {
        serde_json::to_string(&self.resource_pool)
            .map_err(|e| ManagerError::JsonParseError { msg: e.to_string() })
    }

    fn _verify_manager(_: &Specification) -> Result<(), ManagerError> {
        // validate cgroups path
        // validate permissions
        // validate actual cgroups setup
        Ok(())
    }
}
