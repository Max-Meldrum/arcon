// Copyright (c) 2017, Oracle and/or its affiliates.
// SPDX-License-Identifier: Apache-2.0

// Modifications Copyright 2020 KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use crate::device::{LinuxDevice, LinuxDeviceType};
use crate::nix_ext::fchdir;
use crate::Result;
use failure::ResultExt;
use nix::errno::Errno;
use nix::fcntl::{open, OFlag};
use nix::mount::MsFlags;
use nix::mount::*;
use nix::sys::stat::{mknod, umask};
use nix::sys::stat::{Mode, SFlag};
use nix::unistd::{chdir, chown, close, getcwd, pivot_root};
use nix::unistd::{Gid, Uid};
use nix::NixPath;
use std::collections::HashMap;
use std::fs::{canonicalize, create_dir_all, remove_file};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Mount {
    pub destination: String,
    pub mount_type: String,
    pub source: String,
    pub options: Option<Vec<String>>,
}

lazy_static! {
    static ref MOUNTS: Vec<Mount> = {
        let mut mounts = Vec::new();
        mounts.push(Mount {
            destination: "/proc".to_string(),
            mount_type: "proc".to_string(),
            source: "proc".to_string(),
            options: None,
        });
        mounts.push(Mount {
            destination: "/dev".to_string(),
            mount_type: "tmpfs".to_string(),
            source: "tmpfs".to_string(),
            options: Some(vec![
                "noexec".into(),
                "strictatime".into(),
                "mode=755".into(),
            ]),
        });
        mounts.push(Mount {
            destination: "/dev/pts".to_string(),
            mount_type: "devpts".to_string(),
            source: "devpts".to_string(),
            options: Some(vec![
                "nosuid".into(),
                "noexec".into(),
                "newinstance".into(),
                "ptmxmode=0666".into(),
                "mode=620".into(),
                "gid=5".into(),
            ]),
        });
        mounts.push(Mount {
            destination: "/dev/shm".to_string(),
            mount_type: "tmpfs".to_string(),
            source: "shm".to_string(),
            options: Some(vec![
                "nosuid".into(),
                "noexec".into(),
                "mode=1777".into(),
                "nodev".into(),
                "size=65536k".into(),
            ]),
        });
        mounts.push(Mount {
            destination: "/dev/mqueue".to_string(),
            mount_type: "mqueue".to_string(),
            source: "mqueue".to_string(),
            options: Some(vec!["nosuid".into(), "noexec".into(), "nodev".into()]),
        });
        mounts.push(Mount {
            destination: "/sys".to_string(),
            mount_type: "sysfs".to_string(),
            source: "sysfs".to_string(),
            options: Some(vec![
                "nosuid".into(),
                "noexec".into(),
                "nodev".into(),
                "ro".into(),
            ]),
        });
        mounts.push(Mount {
            destination: "/sys/fs/cgroup".to_string(),
            mount_type: "cgroup2".to_string(),
            source: "cgroup2".to_string(),
            options: Some(vec![
                "nosuid".into(),
                "noexec".into(),
                "nodev".into(),
                "relatime".into(),
                "ro".into(),
            ]),
        });
        mounts
    };
}

lazy_static! {
    pub static ref PATHS: HashMap<String, String> = {
        let mut result = HashMap::new();
        let f = match File::open("/proc/self/cgroup") {
            Ok(f) => f,
            Err(e) => {
                warn! {"could not load cgroup info: {}", e};
                return result;
            }
        };

        for line in BufReader::new(f).lines() {
            let l = match line {
                Ok(l) => l,
                Err(e) => {
                    warn!("failed to read cgroup info: {}", e);
                    return result;
                }
            };
            let fields: Vec<&str> = l.split(':').collect();
            if fields.len() != 3 {
                warn!("cgroup data is corrupted");
                continue;
            }
            result.insert(fields[1].to_string(), fields[2].to_string());
        }

        result
    };
}

lazy_static! {
    pub static ref MOUNTS_MAP: HashMap<String, String> = {
        let mut result = HashMap::new();
        let f = match File::open("/proc/self/mountinfo") {
            Ok(f) => f,
            Err(e) => {
                warn! {"could not load mount info: {}", e};
                return result;
            }
        };
        for line in BufReader::new(f).lines() {
            let l = match line {
                Ok(l) => l,
                Err(e) => {
                    warn!("failed to read mount info: {}", e);
                    return result;
                }
            };
            if let Some(sep) = l.find(" - ") {
                if l.len() < sep + 10 {
                    continue;
                }
                let key = &l[sep + 3..sep + 10];
                if key != "cgroup " && key != "cgroup2" {
                    continue;
                }
                let pre: Vec<&str> = l[..sep].split(' ').collect();
                if pre.len() != 7 {
                    warn!("mountinfo data is corrupted");
                    continue;
                }
                let post: Vec<&str> = l[sep + 3..].split(' ').collect();
                if post.len() != 3 {
                    warn!("mountinfo data is corrupted");
                    continue;
                }
                let mut offset = post[2].len();
                while let Some(o) = post[2][..offset].rfind(',') {
                    let name = &post[2][o + 1..];
                    if PATHS.contains_key(name) {
                        result.insert(name.to_string(), pre[4].to_string());
                        break;
                    }
                    offset = o;
                }
            } else {
                warn!("mountinfo data is corrupted");
            }
        }
        result
    };
}

#[cfg_attr(rustfmt, rustfmt_skip)]
lazy_static! {
    static ref OPTIONS: HashMap<&'static str, (bool, MsFlags)> = {
        let mut m = HashMap::new();
        m.insert("defaults",      (false, MsFlags::empty()));
        m.insert("ro",            (false, MsFlags::MS_RDONLY));
        m.insert("rw",            (true,  MsFlags::MS_RDONLY));
        m.insert("suid",          (true,  MsFlags::MS_NOSUID));
        m.insert("nosuid",        (false, MsFlags::MS_NOSUID));
        m.insert("dev",           (true,  MsFlags::MS_NODEV));
        m.insert("nodev",         (false, MsFlags::MS_NODEV));
        m.insert("exec",          (true,  MsFlags::MS_NOEXEC));
        m.insert("noexec",        (false, MsFlags::MS_NOEXEC));
        m.insert("sync",          (false, MsFlags::MS_SYNCHRONOUS));
        m.insert("async",         (true,  MsFlags::MS_SYNCHRONOUS));
        m.insert("dirsync",       (false, MsFlags::MS_DIRSYNC));
        m.insert("remount",       (false, MsFlags::MS_REMOUNT));
        m.insert("mand",          (false, MsFlags::MS_MANDLOCK));
        m.insert("nomand",        (true,  MsFlags::MS_MANDLOCK));
        m.insert("atime",         (true,  MsFlags::MS_NOATIME));
        m.insert("noatime",       (false, MsFlags::MS_NOATIME));
        m.insert("diratime",      (true,  MsFlags::MS_NODIRATIME));
        m.insert("nodiratime",    (false, MsFlags::MS_NODIRATIME));
        m.insert("bind",          (false, MsFlags::MS_BIND));
        m.insert("rbind",         (false, MsFlags::MS_BIND | MsFlags::MS_REC));
        m.insert("unbindable",    (false, MsFlags::MS_UNBINDABLE));
        m.insert("runbindable",   (false, MsFlags::MS_UNBINDABLE | MsFlags::MS_REC));
        m.insert("private",       (false, MsFlags::MS_PRIVATE));
        m.insert("rprivate",      (false, MsFlags::MS_PRIVATE | MsFlags::MS_REC));
        m.insert("shared",        (false, MsFlags::MS_SHARED));
        m.insert("rshared",       (false, MsFlags::MS_SHARED | MsFlags::MS_REC));
        m.insert("slave",         (false, MsFlags::MS_SLAVE));
        m.insert("rslave",        (false, MsFlags::MS_SLAVE | MsFlags::MS_REC));
        m.insert("relatime",      (false, MsFlags::MS_RELATIME));
        m.insert("norelatime",    (true,  MsFlags::MS_RELATIME));
        m.insert("strictatime",   (false, MsFlags::MS_STRICTATIME));
        m.insert("nostrictatime", (true,  MsFlags::MS_STRICTATIME));
        m
    };
}

pub fn init_rootfs(rootfs: &str, bind_devices: bool) -> Result<()> {
    // set namespace propagation
    let mut flags = MsFlags::MS_REC;
    flags |= MsFlags::MS_SLAVE;

    mount(None::<&str>, "/", None::<&str>, flags, None::<&str>)?;

    // mount root dir
    mount(
        Some(rootfs),
        rootfs,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    for m in &*MOUNTS {
        // TODO: check for nasty destinations involving symlinks and illegal
        //       locations.
        // NOTE: this strictly is less permissive than runc, which allows ..
        //       as long as the resulting path remains in the rootfs. There
        //       is no good reason to allow this so we just forbid it
        if !m.destination.starts_with('/') || m.destination.contains("..") {
            let msg = format!("invalid mount destination: {}", m.destination);
            bail!(msg);
        }
        let (flags, data) = parse_mount(m);
        if m.mount_type == "cgroup2" {
            mount_cgroups_v2(m, rootfs, &"")?;
        } else if m.destination == "/dev" {
            // dev can't be read only yet because we have to mount devices
            mount_from(
                m,
                rootfs,
                flags & !MsFlags::MS_RDONLY,
                &data,
                &"", // fix
            )?;
        } else {
            mount_from(m, rootfs, flags, &data, &"")?;
        }
    }

    // chdir into the rootfs so we can make devices with simpler paths
    let olddir = getcwd()?;
    chdir(rootfs)?;

    default_symlinks()?;
    crate::device::create_devices(bind_devices)?;
    ensure_ptmx()?;

    chdir(&olddir)?;

    Ok(())
}

pub fn pivot_rootfs<P: ?Sized + NixPath>(path: &P) -> Result<()> {
    let oldroot = open("/", OFlag::O_DIRECTORY | OFlag::O_RDONLY, Mode::empty())?;
    defer!(close(oldroot).unwrap());
    let newroot = open(path, OFlag::O_DIRECTORY | OFlag::O_RDONLY, Mode::empty())?;
    defer!(close(newroot).unwrap());
    pivot_root(path, path)?;
    umount2("/", MntFlags::MNT_DETACH)?;
    fchdir(newroot)?;
    Ok(())
}

pub fn finish_rootfs() -> Result<()> {
    let masked_paths: Vec<String> = vec![
        "/proc/kcore".into(),
        "/proc/latency_stats".into(),
        "/proc/timer_list".into(),
        "/proc/timer_stats".into(),
        "/proc/sched_debug".into(),
        "/sys/firemware".into(),
        "/proc/scsi".into(),
    ];
    let readonly_paths: Vec<String> = vec![
        "/proc/asound".into(),
        "/proc/bus".into(),
        "/proc/fs".into(),
        "/proc/irq".into(),
        "/proc/sys".into(),
        "/proc/sysrq-trigger".into(),
    ];
    for path in &masked_paths {
        mask_path(path)?;
    }

    for path in &readonly_paths {
        readonly_path(path)?;
    }
    /*
    // remount dev ro if necessary
    for m in &spec.mounts {
        if m.destination == "/dev" {
            let (flags, _) = parse_mount(m);
            if flags.contains(MsFlags::MS_RDONLY) {
                mount(
                    Some("/dev"),
                    "/dev",
                    None::<&str>,
                    flags | MsFlags::MS_REMOUNT,
                    None::<&str>,
                )?;
            }
        }
    }

    if spec.root.readonly {
        let flags = MsFlags::MS_BIND
            | MsFlags::MS_RDONLY
            | MsFlags::MS_NODEV
            | MsFlags::MS_REMOUNT;
        mount(Some("/"), "/", None::<&str>, flags, None::<&str>)?;
    }

    */
    // Uncomment to make Read only..
    //let flags = MsFlags::MS_BIND | MsFlags::MS_RDONLY | MsFlags::MS_NODEV | MsFlags::MS_REMOUNT;
    //mount(Some("/"), "/", None::<&str>, flags, None::<&str>)?;
    umask(Mode::from_bits_truncate(0o022));
    Ok(())
}
fn mount_cgroups_v2(m: &Mount, rootfs: &str, label: &str) -> Result<()> {
    let cflags = MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID | MsFlags::MS_NODEV;
    mount_from(m, rootfs, cflags, "", label)?;
    // TODO: continue...
    Ok(())
}

fn parse_mount(m: &Mount) -> (MsFlags, String) {
    let mut flags = MsFlags::empty();
    let mut data = Vec::new();
    match m.options.as_ref() {
        Some(opts) => {
            for s in &*opts {
                match OPTIONS.get(s.as_str()) {
                    Some(x) => {
                        let (clear, f) = *x;
                        if clear {
                            flags &= !f;
                        } else {
                            flags |= f;
                        }
                    }
                    None => {
                        data.push(s.as_str());
                    }
                }
            }
        }
        None => {}
    }
    (flags, data.join(","))
}

fn mount_from(m: &Mount, rootfs: &str, flags: MsFlags, data: &str, label: &str) -> Result<()> {
    let d;
    if !label.is_empty() && m.mount_type != "proc" && m.mount_type != "sysfs" {
        if data.is_empty() {
            d = format! {"context=\"{}\"", label};
        } else {
            d = format! {"{},context=\"{}\"", data, label};
        }
    } else {
        d = data.to_string();
    }

    let dest = format! {"{}{}", rootfs, &m.destination};

    debug!(
        "mounting {} to {} as {} with data '{}'",
        &m.source, &m.destination, &m.mount_type, &d
    );

    let src = if m.mount_type == "bind" {
        let src = canonicalize(&m.source)?;
        let dir = if src.is_file() {
            Path::new(&dest).parent().unwrap()
        } else {
            Path::new(&dest)
        };
        if let Err(e) = create_dir_all(&dir) {
            debug!("ignoring create dir fail of {:?}: {}", &dir, e)
        }
        // make sure file exists so we can bind over it
        if src.is_file() {
            if let Err(e) = OpenOptions::new().create(true).write(true).open(&dest) {
                debug!("ignoring touch fail of {:?}: {}", &dest, e)
            }
        }
        src
    } else {
        if let Err(e) = create_dir_all(&dest) {
            debug!("ignoring create dir fail of {:?}: {}", &dest, e)
        }
        PathBuf::from(&m.source)
    };

    if let Err(::nix::Error::Sys(errno)) =
        mount(Some(&*src), &*dest, Some(&*m.mount_type), flags, Some(&*d))
    {
        if errno != Errno::EINVAL {
            let msg = format!("mount of {} failed", &m.destination);
            return Err(::nix::Error::Sys(errno)).context(msg)?;
        }
        // try again without mount label
        mount(Some(&*src), &*dest, Some(&*m.mount_type), flags, Some(data))?;
        // warn if label cannot be set
        /*
        if let Err(e) = setfilecon(&dest, label) {
            warn! {"could not set mount label of {} to {}: {}",
            &m.destination, &label, e};
        }
        */
    }
    // remount bind mounts if they have other flags (like MsFlags::MS_RDONLY)
    if flags.contains(MsFlags::MS_BIND)
        && flags.intersects(
            !(MsFlags::MS_REC
                | MsFlags::MS_REMOUNT
                | MsFlags::MS_BIND
                | MsFlags::MS_PRIVATE
                | MsFlags::MS_SHARED
                | MsFlags::MS_SLAVE),
        )
    {
        let err_msg = format!("remount of {} failed", &dest);
        mount(
            Some(&*dest),
            &*dest,
            None::<&str>,
            flags | MsFlags::MS_REMOUNT,
            None::<&str>,
        )
        .context(err_msg)?;
    }
    Ok(())
}

static SYMLINKS: &'static [(&'static str, &'static str)] = &[
    ("/proc/self/fd", "dev/fd"),
    ("/proc/self/fd/0", "dev/stdin"),
    ("/proc/self/fd/1", "dev/stdout"),
    ("/proc/self/fd/2", "dev/stderr"),
];

fn default_symlinks() -> Result<()> {
    if Path::new("/proc/kcore").exists() {
        symlink("/proc/kcore", "dev/kcore")?;
    }
    for &(src, dst) in SYMLINKS {
        symlink(src, dst)?;
    }
    Ok(())
}

fn ensure_ptmx() -> Result<()> {
    if let Err(e) = remove_file("dev/ptmx") {
        if e.kind() != ::std::io::ErrorKind::NotFound {
            let msg = "could not delete /dev/ptmx".to_string();
            bail!(msg);
        }
    }
    symlink("pts/ptmx", "dev/ptmx")?;
    Ok(())
}

fn makedev(major: u64, minor: u64) -> u64 {
    (minor & 0xff) | ((major & 0xfff) << 8) | ((minor & !0xff) << 12) | ((major & !0xfff) << 32)
}

fn to_sflag(t: LinuxDeviceType) -> Result<SFlag> {
    Ok(match t {
        LinuxDeviceType::b => SFlag::S_IFBLK,
        LinuxDeviceType::c | LinuxDeviceType::u => SFlag::S_IFCHR,
        LinuxDeviceType::p => SFlag::S_IFIFO,
        LinuxDeviceType::a => {
            let msg = "type a is not allowed for linux device".to_string();
            bail!(msg);
        }
    })
}

pub fn mknod_dev(dev: &LinuxDevice) -> Result<()> {
    let f = to_sflag(dev.typ)?;
    debug!("mknoding {}", &dev.path);
    mknod(
        &dev.path[1..],
        f,
        Mode::from_bits_truncate(dev.file_mode.unwrap_or(0)),
        makedev(dev.major, dev.minor),
    )?;
    chown(
        &dev.path[1..],
        dev.uid.map(|n| Uid::from_raw(n)),
        dev.gid.map(|n| Gid::from_raw(n)),
    )?;
    Ok(())
}

pub fn bind_dev(dev: &LinuxDevice) -> Result<()> {
    let fd = open(
        &dev.path[1..],
        OFlag::O_RDWR | OFlag::O_CREAT,
        Mode::from_bits_truncate(0o644),
    )?;
    close(fd)?;
    debug!("bind mounting {}", &dev.path);
    mount(
        Some(&*dev.path),
        &dev.path[1..],
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    )?;
    Ok(())
}

fn mask_path(path: &str) -> Result<()> {
    if !path.starts_with('/') || path.contains("..") {
        let msg = format!("invalid maskedPath: {}", path);
        bail!(msg);
    }

    if let Err(::nix::Error::Sys(errno)) = mount(
        Some("/dev/null"),
        path,
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    ) {
        // ignore ENOENT and ENOTDIR: path to mask doesn't exist
        if errno != Errno::ENOENT && errno != Errno::ENOTDIR {
            let msg = format!("could not mask {}", path);
            bail!(msg);
        } else {
            debug!("ignoring mask of {} because it doesn't exist", path);
        }
    }
    Ok(())
}

fn readonly_path(path: &str) -> Result<()> {
    if !path.starts_with('/') || path.contains("..") {
        let msg = format!("invalid readonlyPath: {}", path);
        bail!(msg);
    }
    if let Err(e) = mount(
        Some(&path[1..]),
        path,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    ) {
        match e {
            ::nix::Error::Sys(errno) => {
                // ignore ENOENT: path to make read only doesn't exist
                if errno != Errno::ENOENT {
                    let msg = format!("could not readonly {}", path);
                    bail!(msg);
                }
                debug!("ignoring remount of {} because it doesn't exist", path);
                return Ok(());
            }
            _ => {
                unreachable!("Supposedly unreachable error {:?}", e);
            }
        }
    }
    mount(
        Some(&path[1..]),
        &path[1..],
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC | MsFlags::MS_RDONLY | MsFlags::MS_REMOUNT,
        None::<&str>,
    )?;
    Ok(())
}
