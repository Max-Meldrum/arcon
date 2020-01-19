// Copyright 2020 KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use failure::Fail;
use serde::{Deserialize, Serialize};

#[derive(Debug, Fail)]
#[fail(display = "Loading spec err: `{}`", msg)]
pub struct SpecError {
    msg: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Specification {
    pub id: String,
    pub cgroups: CgroupsConfig,
    pub resources: ResourceConfig,
    pub shell: ShellConfig,
    pub plugins: Option<Vec<PluginConfig>>,
}

impl Specification {
    pub fn load(path: &str) -> Result<Specification, SpecError> {
        let data = std::fs::read_to_string(path).map_err(|e| SpecError { msg: e.to_string() })?;
        toml::from_str(&data).map_err(|e| SpecError { msg: e.to_string() })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CgroupsConfig {
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    pub cmd: String,
    pub path: String,
    pub term: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub path: String,
    pub args: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub vcpu: u32,
    pub mem: u64,
    // TODO: blkio, net etc..
    // Optional RDMA, GPU etc..
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    static TOML_SPEC: &str = r#"
        id = "test"

        [cgroups]
        path = "/sys/fs/cgroup"

        [shell]
        cmd = "sh"
        path = "/bin"
        term = "xterm"

        [resources]
        vcpu = 4
        mem = 1024
    "#;

    #[cfg(target_os = "linux")]
    #[test]
    fn spec_file_test() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{}", TOML_SPEC).unwrap();
        let arconos_spec = Specification::load(file.path().to_str().unwrap());
        assert_eq!(arconos_spec.is_ok(), true);
        assert_eq!(arconos_spec.unwrap().id, String::from("test"));
    }
}
