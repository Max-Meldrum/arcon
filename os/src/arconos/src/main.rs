// Copyright (c) 2017, Oracle and/or its affiliates.
// SPDX-License-Identifier: Apache-2.0

// Modifications Copyright 2020 KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
#[macro_use(defer)]
extern crate scopeguard;
#[macro_use]
extern crate failure;

mod device;
mod mount;
mod namespace;
mod nix_ext;
mod signals;
mod sync;
mod util;

use clap::{App, AppSettings, Arg, SubCommand};
use failure::ResultExt;
use manager::{api_server::ApiServer, Manager};
use nix::errno::Errno;
use nix::fcntl::{open, OFlag};
use nix::poll::{poll, EventFlags, PollFd};
use nix::sched::{unshare, CloneFlags};
use nix::sys::signal::{SigSet, Signal};
use nix::sys::stat::{fstat, Mode};
use nix::sys::wait::WaitPidFlag;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{chdir, execvp, getpid, sethostname, setresgid, setresuid};
use nix::unistd::{close, dup2, fork, pipe2, read, write, ForkResult};
use nix::unistd::{Gid, Pid, Uid};
use nix_ext::{clearenv, putenv};
use spec::{PluginConfig, Specification};
use std::ffi::CString;
use std::fs::canonicalize;
use std::fs::metadata;
use std::os::unix::io::RawFd;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use sync::Cond;

pub type Result<T> = ::std::result::Result<T, failure::Error>;

const DEFAULT_SPEC: &str = "arconos.toml";
const DEFAULT_LOG_DIR: &str = "/tmp";
const DEFAULT_HOSTNAME: &str = "arconos";
const DEFAULT_ROOTFS_PATH: &str = "rootfs";
const DEFAULT_API_SOCK: &str = "/tmp/arconos.sock";

//const INIT_PID: &'static str = "init.pid";

fn main() {
    pretty_env_logger::init();

    let log_dir_arg = Arg::with_name("l")
        .required(false)
        .default_value(DEFAULT_LOG_DIR)
        .takes_value(true)
        .long("log-dir")
        .short("l")
        .help("Directory where logs are stored");

    let spec_arg = Arg::with_name("s")
        .required(true)
        .default_value(".")
        .takes_value(true)
        .long("spec")
        .short("s")
        .help("Path to Arconos specification");

    let rootfs_arg = Arg::with_name("r")
        .required(false)
        .default_value(DEFAULT_ROOTFS_PATH)
        .takes_value(true)
        .long("rootfs-dir")
        .short("r")
        .help("rootfs directory");

    let instance_id = Arg::with_name("id")
        .required(true)
        .takes_value(true)
        .long("instance id")
        .short("id")
        .help("ID of arconos instance");

    let matches = App::new("Arconos")
        .setting(AppSettings::ColoredHelp)
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("d")
                .help("daemonize Arconos")
                .long("daemon mode")
                .short("d"),
        )
        .arg(
            Arg::with_name("s")
                .default_value("/opt/arconos")
                .help("State location")
                .long("state dir")
                .short("s")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("run")
                .setting(AppSettings::ColoredHelp)
                .arg(&log_dir_arg)
                .arg(&rootfs_arg)
                .arg(&spec_arg)
                .about("Run arconos"),
        )
        .subcommand(
            SubCommand::with_name("ps")
                .setting(AppSettings::ColoredHelp)
                .about("Show current arconos instances"),
        )
        .subcommand(
            SubCommand::with_name("status")
                .setting(AppSettings::ColoredHelp)
                .arg(&instance_id)
                .about("Show status of a arconos instance"),
        )
        .get_matches_from(fetch_args());

    let daemonize: bool = matches.is_present("d");
    let state_dir = matches.value_of("s").expect("could not set state dir");

    match matches.subcommand() {
        ("run", Some(arg_matches)) => {
            let spec_path = arg_matches
                .value_of("s")
                .expect("Should not happen as there is a default");

            let log_dir: &str = arg_matches.value_of("l").unwrap_or(DEFAULT_LOG_DIR);
            let rootfs: &str = arg_matches.value_of("r").unwrap_or(DEFAULT_ROOTFS_PATH);
            if let Err(err) = prepare_run(spec_path, state_dir, rootfs, log_dir, daemonize) {
                error!("{}", err.to_string());
            }
        }
        ("ps", Some(_)) => {
            unimplemented!();
        }
        ("status", Some(_)) => {
            unimplemented!();
        }
        _ => panic!("Bad arg"),
    }
}

fn fetch_args() -> Vec<String> {
    std::env::args().collect()
}

fn prepare_run(
    spec_path: &str,
    state_dir: &str,
    rootfs: &str,
    _log_dir: &str,
    daemonize: bool,
) -> Result<()> {
    let spec_file: String = {
        let md = metadata(&spec_path)?;
        if md.is_file() {
            spec_path.to_string()
        } else {
            (spec_path.to_owned() + "/" + DEFAULT_SPEC)
        }
    };

    // Load Arconos spec
    let spec = spec::Specification::load(&spec_file)?;

    // Fetch instance name
    let instance_id = spec.id.clone();

    // Verify state dir and check if instance already exists
    let instance_dir = format!("{}/{}", state_dir, instance_id);
    if std::path::Path::new(&instance_dir).exists() {
        bail!(format!("instance with id {} already exists", instance_id));
    } else {
        // continue and create instance directory
        std::fs::create_dir_all(instance_dir)?;
    }

    // TODO: Fill instance dir with pid file in order to monitor if the process is
    //       alive or dead etc...

    // rootfs directory
    let rootfs_path = canonicalize(rootfs).unwrap().to_string_lossy().into_owned();

    let pid = safe_run(spec, &rootfs_path, daemonize)?;
    debug!("I am saving pid {}", pid);

    Ok(())
}

fn safe_run(spec: Specification, rootfs: &str, daemonize: bool) -> Result<Pid> {
    let pid = getpid();
    match run(spec, &rootfs, Pid::from_raw(-1), daemonize) {
        Err(e) => {
            // if we are the top level thread, kill all children
            if pid == getpid() {
                signals::signal_children(Signal::SIGTERM).unwrap();
            }
            Err(e)
        }
        Ok(child_pid) => Ok(child_pid),
    }
}

fn run(spec: Specification, rootfs: &str, init_pid: Pid, daemonize: bool) -> Result<Pid> {
    let tsocketfd = -1;

    if let Err(e) = prctl::set_dumpable(false) {
        bail!(format!("set dumpable returned {}", e));
    };

    // Set up console sockets etc..

    // Set up namespaces
    let (cf, to_enter) = namespace::collect()?;

    if !daemonize {
        if let Err(e) = prctl::set_child_subreaper(true) {
            bail!(format!("set subreaper returned {}", e));
        };
    }
    let (child_pid, wfd) = fork_arconos(init_pid, daemonize)?;

    if child_pid != Pid::from_raw(-1) {
        return Ok(child_pid);
    }

    let _mount_fd = namespace::enter(to_enter)?;

    // Unshare the other namespaces
    unshare(cf & !CloneFlags::CLONE_NEWUSER)?;

    fork_enter_pid(true, daemonize)?;

    // Set dragonslayer hostname
    sethostname(DEFAULT_HOSTNAME)?;

    mount::init_rootfs(&*rootfs, true)
        .map_err(|e| format_err!("failed to initialise rootfs => {}", e.to_string()))?;

    /*
    // Notify parent that it can continue
    debug!("writing zero to pipe to trigger prestart");
    let data: &[u8] = &[0];
    write(wfd, data).context("failed to write zero")?;
    */

    mount::pivot_rootfs(&*rootfs)
        .map_err(|e| format_err!("failed to pivot rootfs => {}", e.to_string()))?;

    reopen_dev_null()?;

    // only set sysctls in newns
    /*
    for (key, value) in &linux.sysctl {
        set_sysctl(key, value)?;
    }
    */

    mount::finish_rootfs()
        .map_err(|e| format_err!("failed to finallise rootfs => {}", e.to_string()))?;

    // Change current working directory to the root of the filesystem
    chdir("/")?;

    // set uid/gid groups
    let uid = Uid::from_raw(0);
    let gid = Gid::from_raw(0);
    setid(uid, gid)?;

    debug!("writing zero to pipe to trigger poststart");
    let data: &[u8] = &[0];
    write(wfd, data).context("failed to write zero")?;

    fork_final_child(&"", wfd, tsocketfd, daemonize)?;

    // we no longer need wfd, so close it
    close(wfd).context("could not close wfd")?;

    // At this point we have our init process with pid 1

    // Fork and create the manager process for pid 2
    //run_manager(&spec)?;

    // Launch plugin processes if specified
    if let Some(plugins) = &spec.plugins {
        debug!("plugins {:?}", plugins);
        for plugin in plugins {
            run_plugin(&plugin)?;
        }
    }

    if daemonize {
        run_manager(&spec)?;
    } else {
        run_manager(&spec)?;
        // Enter shell
        do_exec(
            &spec.shell.cmd,
            &*vec![spec.shell.cmd.clone()],
            &vec![spec.shell.term, spec.shell.path],
        )?;
    }
    Ok(Pid::from_raw(-1))
}

fn do_exec(path: &str, args: &[String], env: &[String]) -> Result<()> {
    let p = CString::new(path.to_string()).unwrap();
    let a: Vec<CString> = args
        .iter()
        .map(|s| CString::new(s.to_string()).unwrap_or_default())
        .collect();
    let env: Vec<CString> = env
        .iter()
        .map(|s| CString::new(s.to_string()).unwrap_or_default())
        .collect();
    // execvp doesn't use env for the search path, so we set env manually
    clearenv()?;
    for e in &env {
        debug!("adding {:?} to env", e);
        putenv(e)?;
    }
    execvp(&p, &a).context("failed to exec")?;
    // should never reach here
    Ok(())
}

fn fork_arconos(init_pid: Pid, daemonize: bool) -> Result<(Pid, RawFd)> {
    let ccond = Cond::new().context("cond failed")?;
    let pcond = Cond::new().context("cond failed")?;
    let (rfd, wfd) = pipe2(OFlag::O_CLOEXEC).context("pipe failed")?;
    match fork()? {
        ForkResult::Child => {
            close(rfd).context("could not close rfd")?;
            set_name("dragonslayer")?;

            // set rlimits (before entering user ns)
            /*
            use crate::rlimit::*;
            let rlimits = vec![LinuxRlimit {
                typ: LinuxRlimitType::RLIMIT_NOFILE,
                hard: 1024,
                soft: 1024,
            }];
            */
            /*
            for rlimit in rlimits.iter() {
                setrlimit(rlimit.typ as i32, rlimit.soft, rlimit.hard)?;
            }
            */

            ccond.notify().context("failed to notify parent")?;
            pcond.wait().context("failed to wait for parent")?;
            // Child continues
        }
        ForkResult::Parent { child } => {
            close(wfd).context("could not close wfd")?;
            ccond.wait().context("failed to wait for child")?;
            pcond.notify().context("failed to notify child")?;

            let (_, _) = wait_for_child(child)?;

            let mut pid = Pid::from_raw(-1);

            // Prestart hooks
            wait_for_pipe_zero(rfd, -1)?;

            let procs = cgroups::get_procs("/sys/fs/cgroup/cpuset")?;
            for p in procs {
                let pd = Pid::from_raw(p as i32);
                if pd != init_pid {
                    debug!("actual pid of child is {}", p);
                    pid = pd;
                    break;
                }
            }

            // Poststart hooks
            //wait_for_pipe_zero(rfd, -1)?;

            if daemonize {
                debug!("first parent exiting for daemonization");
                return Ok((pid, wfd));
            }

            signals::pass_signals(pid)?;
            let sig = wait_for_pipe_sig(rfd, -1)?;
            let (exit_code, _) = wait_for_child(pid)?;
            debug!("Successfully waited for the second child");
            exit(exit_code as i8, sig)?;
        }
    }

    Ok((Pid::from_raw(-1), wfd))
}

fn wait_for_pipe_sig(rfd: RawFd, timeout: i32) -> Result<Option<Signal>> {
    let result = wait_for_pipe_vec(rfd, timeout, 1)?;
    if result.len() < 1 {
        return Ok(None);
    }
    let s = Signal::from_c_int(result[0] as i32).context("SignalError => invalid signal")?;
    Ok(Some(s))
}

fn wait_for_pipe_zero(rfd: RawFd, timeout: i32) -> Result<()> {
    let result = wait_for_pipe_vec(rfd, timeout, 1)?;
    if result.len() < 1 {
        let msg = "file descriptor closed unexpectedly".to_string();
        return Err(format_err!("PipeError => {}", msg));
    }
    if result[0] != 0 {
        let msg = format! {"got {} from pipe instead of 0", result[0]};
        return Err(format_err!("PipeError => {}", msg));
    }
    Ok(())
}

fn wait_for_pipe_vec(rfd: RawFd, timeout: i32, num: usize) -> Result<(Vec<u8>)> {
    let mut result = Vec::new();
    while result.len() < num {
        let pfds = &mut [PollFd::new(rfd, EventFlags::POLLIN | EventFlags::POLLHUP)];
        match poll(pfds, timeout) {
            Err(e) => {
                if e != ::nix::Error::Sys(Errno::EINTR) {
                    return Err(format_err!("PipeError => unable to poll rfd"));
                }
                continue;
            }
            Ok(n) => {
                if n == 0 {
                    return Err(format_err!("PipeError => pipe timeout"));
                }
            }
        }
        let events = pfds[0].revents();
        if events.is_none() {
            // continue on no events
            continue;
        }
        if events.unwrap() == EventFlags::POLLNVAL {
            let msg = "file descriptor closed unexpectedly".to_string();
            return Err(format_err!("PipeError => {}", msg));
        }
        if !events
            .unwrap()
            .intersects(EventFlags::POLLIN | EventFlags::POLLHUP)
        {
            // continue on other events (should not happen)
            debug!("got a continue on other events {:?}", events);
            continue;
        }
        let data: &mut [u8] = &mut [0];
        let n = read(rfd, data).context("could not read from rfd")?;
        if n == 0 {
            // the wfd was closed so close our end
            close(rfd).context("could not close rfd")?;
            break;
        }
        result.extend(data.iter().cloned());
    }
    Ok(result)
}

fn fork_final_child(_cgroups_path: &str, wfd: RawFd, tfd: RawFd, daemonize: bool) -> Result<()> {
    // fork again so child becomes pid 2
    match fork()? {
        ForkResult::Child => {
            // child continues on
            Ok(())
        }
        ForkResult::Parent { .. } => {
            // TODO: add security seccomp/capabilities

            if tfd != -1 {
                close(tfd).context("could not close trigger fd")?;
            }

            do_init(wfd, daemonize)?;
            Ok(())
        }
    }
}

fn do_init(wfd: RawFd, daemonize: bool) -> Result<()> {
    if daemonize {
        close(wfd).context("could not close wfd")?;
    }
    let s = SigSet::all();
    s.thread_block()?;
    loop {
        let signal = s.wait()?;
        if signal == Signal::SIGCHLD {
            debug!("got a sigchld");
            let mut sig = None;
            let code;
            match reap_children()? {
                WaitStatus::Exited(_, c) => code = c as i32,
                WaitStatus::Signaled(_, s, _) => {
                    sig = Some(s);
                    code = 128 + s as libc::c_int;
                }
                _ => continue,
            };
            if !daemonize {
                if let Some(s) = sig {
                    // raising from pid 1 doesn't work as you would
                    // expect, so write signal to pipe.
                    let data: &[u8] = &[s as u8];
                    write(wfd, data).context("failed to write signal")?;
                }
                close(wfd).context("could not close wfd")?;
            }
            debug!("all children terminated, exiting with {}", code);
            std::process::exit(code)
        }
        debug!("passing {:?} on to children", signal);
        if let Err(e) = signals::signal_process(Pid::from_raw(-1), signal) {
            warn!("failed to signal children, {}", e);
        }
    }
}

fn wait_for_child(child: Pid) -> Result<(i32, Option<Signal>)> {
    loop {
        // wait on all children, but only return if we match child.
        let result = match waitpid(Pid::from_raw(-1), None) {
            Err(::nix::Error::Sys(errno)) => {
                // ignore EINTR as it gets sent when we get a SIGCHLD
                if errno == Errno::EINTR {
                    continue;
                }
                let msg = format!("could not waitpid on {}", child);
                return Err(::nix::Error::Sys(errno)).context(msg)?;
            }
            Err(e) => {
                return Err(e)?;
            }
            Ok(s) => s,
        };
        match result {
            WaitStatus::Exited(pid, code) => {
                if child != Pid::from_raw(-1) && pid != child {
                    continue;
                }
                reap_children()?;
                return Ok((code as i32, None));
            }
            WaitStatus::Signaled(pid, signal, _) => {
                if child != Pid::from_raw(-1) && pid != child {
                    continue;
                }
                reap_children()?;
                return Ok((0, Some(signal)));
            }
            _ => {}
        };
    }
}

fn exit(exit_code: i8, sig: Option<Signal>) -> Result<()> {
    match sig {
        Some(signal) => {
            debug!("child exited with signal {:?}", signal);

            signals::raise_for_parent(signal)?;
            // wait for normal signal handler to deal with us
            loop {
                signals::wait_for_signal()?;
            }
        }
        None => {
            debug!("child exited with code {:?}", exit_code);
            std::process::exit(exit_code as i32);
        }
    }
}

fn reap_children() -> Result<(WaitStatus)> {
    let mut result = WaitStatus::Exited(Pid::from_raw(0), 0);
    loop {
        match waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
            Err(e) => {
                if e != ::nix::Error::Sys(Errno::ECHILD) {
                    return Err(e).map_err(|err| format_err!("{}", err.to_string()));
                }
                // ECHILD means no processes are left
                break;
            }
            Ok(s) => {
                result = s;
                if result == WaitStatus::StillAlive {
                    break;
                }
            }
        }
    }
    Ok(result)
}

fn reopen_dev_null() -> Result<()> {
    let null_fd = open("/dev/null", OFlag::O_WRONLY, Mode::empty())?;
    let null_stat = fstat(null_fd)?;
    defer!(close(null_fd).unwrap());
    for fd in 0..3 {
        if let Ok(stat) = fstat(fd) {
            if stat.st_rdev == null_stat.st_rdev {
                if fd == 0 {
                    // close and reopen to get RDONLY
                    close(fd)?;
                    open("/dev/null", OFlag::O_RDONLY, Mode::empty())?;
                } else {
                    // we already have wronly fd, so duplicate it
                    dup2(null_fd, fd)?;
                }
            }
        }
    }
    Ok(())
}

fn set_name(name: &str) -> Result<()> {
    if let Err(e) = prctl::set_name(name) {
        bail!(format!("set name returned {}", e));
    };
    Ok(())
}

fn fork_enter_pid(init: bool, daemonize: bool) -> Result<()> {
    // do the first fork right away because we must fork before we can
    // mount proc. The child will be in the pid namespace.
    match fork()? {
        ForkResult::Child => {
            if init {
                set_name("init")?;
            } else if daemonize {
                // NOTE: if we are daemonizing non-init, we need an additional
                //       fork to allow process to be reparented to init
                match fork()? {
                    ForkResult::Child => {
                        // child continues
                    }
                    ForkResult::Parent { .. } => {
                        debug!("third parent exiting for daemonization");
                        exit(0, None)?;
                    }
                }
            }
            // child continues
        }
        ForkResult::Parent { .. } => {
            debug!("second parent exiting");
            exit(0, None)?;
        }
    };
    Ok(())
}

pub fn setid(uid: Uid, gid: Gid) -> Result<()> {
    if let Err(e) = prctl::set_keep_capabilities(true) {
        bail!(format!("set keep capabilities returned {}", e));
    };
    {
        setresgid(gid, gid, gid)?;
    }
    {
        setresuid(uid, uid, uid)?;
    }
    // if we change from zero, we lose effective caps
    /*
    if uid != Uid::from_raw(0) {
        capabilities::reset_effective()?;
    }
    if let Err(e) = prctl::set_keep_capabilities(false) {
        bail!(format!("set keep capabilities returned {}", e));
    };
    */
    Ok(())
}

fn run_manager(spec: &Specification) -> Result<()> {
    match fork()? {
        ForkResult::Child => {
            // child continues..
        }
        ForkResult::Parent { child: _ } => {
            set_name("manager")?;
            info!("loaded spec {:?}", spec);
            // Set up manager and the API server
            let manager = Arc::new(Mutex::new(Manager::new(spec.clone())));
            let mut api_server = ApiServer::new(manager.clone());

            // Attempt to remove existing dragonslayer.sock
            std::fs::remove_file(DEFAULT_API_SOCK).unwrap_or_default();

            // Launch API server
            // NOTE: this is blocking
            api_server.run(PathBuf::from(DEFAULT_API_SOCK.to_string()))?;
        }
    }
    Ok(())
}

fn run_plugin(plugin: &PluginConfig) -> Result<()> {
    match fork()? {
        ForkResult::Child => {
            // child continues..
        }
        ForkResult::Parent { child: _ } => {
            set_name(&plugin.name)?;
            if !std::path::Path::new(&plugin.path).exists() {
                bail!(format!(
                    "plugin binary does not exist at path {}",
                    plugin.path
                ));
            }
            // launch plugin through execvp
            do_exec(&plugin.path, &*vec![plugin.args.clone()], &vec![])?;
        }
    }
    Ok(())
}
