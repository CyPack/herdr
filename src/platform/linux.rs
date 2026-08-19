use std::{
    collections::{HashSet, VecDeque},
    io::Write,
    os::fd::RawFd,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use super::{
    read_limited_reader, ClipboardCommand, ClipboardImage, FileIdentity, ForegroundJob,
    ForegroundProcess, LimitedRead, Signal,
};

const WSL_MARKER_ENV_VARS: &[&str] = &["WSL_DISTRO_NAME", "WSL_INTEROP"];

/// How long a clipboard helper may hold the calling thread before it is handed
/// off to a background reaper. Measured on Wayland: `wl-copy` finishes its
/// compositor handshake and forks in ~104ms, so this leaves roughly 3x headroom
/// while staying under the threshold where a copy feels like a UI stall.
const CLIPBOARD_HANDOFF_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(300);

/// Poll granularity while waiting for a clipboard helper to exit.
const CLIPBOARD_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);

pub(crate) fn file_identity(
    _path: &std::path::Path,
    metadata: &std::fs::Metadata,
) -> std::io::Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;

    Ok(FileIdentity::new(metadata.dev(), metadata.ino()))
}

pub(crate) fn publish_staged_path_no_replace(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;

    let source = std::ffi::CString::new(source.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "staging path contains an interior NUL",
        )
    })?;
    let destination = std::ffi::CString::new(destination.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination path contains an interior NUL",
        )
    })?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcGroupMember {
    pid: u32,
    comm: String,
}

pub fn raise_server_nofile_limit() {}

pub(crate) fn should_draw_host_cursor_by_default() -> bool {
    running_inside_wsl()
}

fn running_inside_wsl() -> bool {
    proc_file_indicates_wsl("/proc/sys/kernel/osrelease")
        || proc_file_indicates_wsl("/proc/version")
        || WSL_MARKER_ENV_VARS
            .iter()
            .any(|key| std::env::var_os(key).is_some())
        || std::path::Path::new("/run/WSL").exists()
}

fn proc_file_indicates_wsl(path: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|text| text_indicates_wsl(&text))
        .unwrap_or(false)
}

fn text_indicates_wsl(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.contains("microsoft") || text.contains("wsl")
}

fn raw_command_argv(command: &str, flag: &str) -> Vec<std::ffi::OsString> {
    vec!["/bin/sh".into(), flag.into(), command.into()]
}

pub(crate) fn detached_custom_command_process_platform(command: &str) -> std::process::Command {
    let argv = raw_command_argv(command, "-lc");
    let mut command = std::process::Command::new(&argv[0]);
    command.args(&argv[1..]);
    command
}

pub(crate) fn pane_custom_command_pty_builder_platform(
    command: &str,
) -> portable_pty::CommandBuilder {
    portable_pty::CommandBuilder::from_argv(raw_command_argv(command, "-c"))
}

pub(crate) fn scrollback_editor_argv(path: &std::path::Path) -> std::io::Result<Vec<String>> {
    let quoted_path = shell_quote(&path.display().to_string());
    let command = format!(
        r#"scrollback_file={quoted_path}; eval "${{EDITOR:-vi}} \"\$scrollback_file\""; status=$?; rm -f "$scrollback_file"; exit $status"#
    );
    Ok(vec!["/bin/sh".to_string(), "-c".to_string(), command])
}

pub(crate) fn interactive_shell_command(argv: &[String], shell_name: &str) -> Option<String> {
    super::interactive_unix_shell_command(argv, shell_name, shell_quote)
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '@' | '%' | '_' | '+' | '=' | ':' | ',' | '.' | '/' | '-'
                )
        })
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Collect the foreground terminal job for a given child PID.
pub(crate) fn available_pane_shell(child_pid: u32) -> Option<String> {
    super::available_pane_shell_from_job(child_pid, foreground_job(child_pid)?)
}

pub fn foreground_job(child_pid: u32) -> Option<ForegroundJob> {
    let tpgid = foreground_process_group_id(child_pid)?;
    let members = foreground_process_group_members(child_pid, tpgid)?;
    let processes = members
        .into_iter()
        .map(|member| {
            let argv = process_argv(member.pid);
            ForegroundProcess {
                pid: member.pid,
                name: member.comm,
                argv0: None,
                cmdline: argv.as_ref().map(|parts| parts.join(" ")),
                argv,
            }
        })
        .collect::<Vec<_>>();

    if processes.is_empty() {
        return None;
    }

    Some(ForegroundJob {
        process_group_id: tpgid,
        processes,
    })
}

fn foreground_process_group_members(
    child_pid: u32,
    process_group_id: u32,
) -> Option<Vec<ProcGroupMember>> {
    foreground_process_group_members_with(
        child_pid,
        process_group_id,
        process_task_ids,
        process_task_children,
        live_process_group_member,
    )
}

fn foreground_process_group_members_with(
    child_pid: u32,
    process_group_id: u32,
    task_ids: impl FnMut(u32) -> Vec<u32>,
    task_children: impl FnMut(u32, u32) -> Vec<u32>,
    mut live_member: impl FnMut(u32, u32) -> Option<ProcGroupMember>,
) -> Option<Vec<ProcGroupMember>> {
    let mut members = process_tree_pids([child_pid, process_group_id], task_ids, task_children)
        .into_iter()
        .filter_map(|pid| live_member(process_group_id, pid))
        .collect::<Vec<_>>();
    members.sort_unstable_by_key(|member| member.pid);
    (!members.is_empty()).then_some(members)
}

fn process_tree_pids(
    roots: impl IntoIterator<Item = u32>,
    mut task_ids: impl FnMut(u32) -> Vec<u32>,
    mut task_children: impl FnMut(u32, u32) -> Vec<u32>,
) -> Vec<u32> {
    let mut pending = VecDeque::new();
    let mut visited = HashSet::new();
    for pid in roots {
        if pid > 0 && visited.insert(pid) {
            pending.push_back(pid);
        }
    }

    let mut pids = Vec::new();
    while let Some(pid) = pending.pop_front() {
        pids.push(pid);
        for tid in task_ids(pid) {
            for child_pid in task_children(pid, tid) {
                if child_pid > 0 && visited.insert(child_pid) {
                    pending.push_back(child_pid);
                }
            }
        }
    }
    pids
}

fn process_task_ids(pid: u32) -> Vec<u32> {
    std::fs::read_dir(format!("/proc/{pid}/task"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| numeric_file_name(&entry))
        .collect()
}

fn process_task_children(pid: u32, tid: u32) -> Vec<u32> {
    let Some(children) = std::fs::read_to_string(format!("/proc/{pid}/task/{tid}/children")).ok()
    else {
        return Vec::new();
    };
    children
        .split_whitespace()
        .filter_map(|child| child.parse::<u32>().ok())
        .collect()
}

fn numeric_file_name(entry: &std::fs::DirEntry) -> Option<u32> {
    let file_name = entry.file_name();
    let value = file_name.to_str()?;
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn live_process_group_member(process_group_id: u32, pid: u32) -> Option<ProcGroupMember> {
    let (pgrp, comm) = process_pgrp_and_comm(pid)?;
    (pgrp > 0 && pgrp as u32 == process_group_id).then_some(ProcGroupMember { pid, comm })
}

pub fn foreground_group_leader_job(process_group_id: u32) -> Option<ForegroundJob> {
    let (pgrp, name) = process_pgrp_and_comm(process_group_id)?;
    if pgrp as u32 != process_group_id {
        return None;
    }

    let argv = process_argv(process_group_id);
    Some(ForegroundJob {
        process_group_id,
        processes: vec![ForegroundProcess {
            pid: process_group_id,
            name,
            argv0: None,
            cmdline: argv.as_ref().map(|parts| parts.join(" ")),
            argv,
        }],
    })
}

pub fn foreground_process_group_id(child_pid: u32) -> Option<u32> {
    // /proc/<pid>/stat format: "pid (comm) state ppid pgrp session tty_nr tpgid ..."
    // The (comm) field can contain spaces and parens, so we find the last ')' first.
    let stat = std::fs::read_to_string(format!("/proc/{child_pid}/stat")).ok()?;
    let rest = stat.get(stat.rfind(')')? + 2..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // After (comm): state(0) ppid(1) pgrp(2) session(3) tty_nr(4) tpgid(5)
    let tpgid: i32 = fields.get(5)?.parse().ok()?;
    (tpgid > 0).then_some(tpgid as u32)
}

pub fn foreground_process_group_id_for_tty_fd(fd: RawFd) -> Option<u32> {
    let pgid = unsafe { libc::tcgetpgrp(fd) };
    (pgid > 0).then_some(pgid as u32)
}

fn process_pgrp_and_comm(pid: u32) -> Option<(i32, String)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    process_pgrp_and_comm_from_stat(&stat)
}

fn process_pgrp_and_comm_from_stat(stat: &str) -> Option<(i32, String)> {
    let close = stat.rfind(')')?;
    let comm = stat.get(1 + stat.find('(')?..close)?.to_string();
    let rest = stat.get(close + 2..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    let pgrp: i32 = fields.get(2)?.parse().ok()?;
    Some((pgrp, comm))
}

fn process_argv(pid: u32) -> Option<Vec<String>> {
    let bytes = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let parts: Vec<String> = bytes
        .split(|&b| b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect();
    (!parts.is_empty()).then_some(parts)
}

/// Get the current working directory of a process.
/// Uses /proc/<pid>/cwd symlink.
pub fn process_cwd(pid: u32) -> Option<PathBuf> {
    if pid == 0 {
        return None;
    }
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

/// Read a Herdr agent identity hint from a process environment.
pub fn process_agent_hint(pid: u32) -> Option<crate::detect::Agent> {
    if pid == 0 {
        return None;
    }
    let environ = std::fs::read(format!("/proc/{pid}/environ")).ok()?;
    super::parse_agent_env_hint(&environ)
}

pub fn session_processes(child_pid: u32) -> Vec<u32> {
    let Some(session_id) = process_session_id(child_pid) else {
        return Vec::new();
    };

    let mut pids = Vec::new();
    for entry in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let file_name = entry.file_name();
        let Some(pid_str) = file_name.to_str() else {
            continue;
        };
        if !pid_str.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }

        let Ok(pid) = pid_str.parse::<u32>() else {
            continue;
        };
        if process_session_id(pid) == Some(session_id) {
            pids.push(pid);
        }
    }
    pids
}

pub fn signal_processes(pids: &[u32], signal: Signal) {
    let sig = match signal {
        Signal::Hangup => libc::SIGHUP,
        Signal::Terminate => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
    };

    for &pid in pids {
        // A pid above i32::MAX would wrap negative under `as i32` and turn
        // kill(2) into a process-group (or kill(-1): every process) signal.
        // try_from makes that class unrepresentable.
        let Ok(pid) = i32::try_from(pid) else {
            continue;
        };
        if pid == 0 {
            continue;
        }
        unsafe {
            libc::kill(pid, sig);
        }
    }
}

pub fn process_exists(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    let result = unsafe { libc::kill(pid, 0) };
    if result == 0 {
        true
    } else {
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Identity-safe shutdown targets (pidfd).
//
// A bare pid is NOT a process identity: pid numbers recycle, and a pid stored
// at spawn time can name an unrelated process by shutdown time — the classic
// ABA problem (an active pid-counter wraparound was measured on this machine
// during incident PM-2026-07-27-001). A pidfd is an identity: once opened it
// pins exactly one process incarnation, and signalling through it can never
// hit a recycled pid, no matter how much time passes between the /proc scan
// and the signal.
// ═══════════════════════════════════════════════════════════════════════════

/// A shutdown target pinned to a specific process incarnation.
#[derive(Debug)]
pub struct ShutdownTarget {
    pid: u32,
    /// `Some` = pidfd-pinned identity. `None` = legacy pid-only fallback
    /// (pre-5.3 kernels without pidfd support).
    fd: Option<std::os::fd::OwnedFd>,
}

impl ShutdownTarget {
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

fn pidfd_open(pid: u32) -> Result<std::os::fd::OwnedFd, std::io::Error> {
    use std::os::fd::FromRawFd;
    // SAFETY: pidfd_open(2) takes a pid and a flags word — no pointers. The
    // only invariant is checking the return value before wrapping it.
    let ret = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0u32) };
    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `ret` is a freshly returned file descriptor we now own.
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(ret as RawFd) })
}

fn pidfd_send_signal(fd: &std::os::fd::OwnedFd, sig: libc::c_int) -> bool {
    use std::os::fd::AsRawFd;
    // SAFETY: pidfd_send_signal(2) with a null siginfo pointer behaves like
    // kill(2) aimed at the exact process the fd pins; arguments are plain
    // integers plus an explicitly-null pointer.
    let ret = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            fd.as_raw_fd(),
            sig,
            std::ptr::null::<libc::c_void>(),
            0u32,
        )
    };
    ret == 0
}

/// What to do when the child's own pidfd cannot be opened. Factored out so the
/// error mapping is unit-testable without forcing kernel error conditions.
fn targets_after_child_open_failure(child_pid: u32, err: &std::io::Error) -> Vec<ShutdownTarget> {
    if err.raw_os_error() == Some(libc::ENOSYS) {
        // Kernel without pidfd support: keep the legacy behavior rather than
        // silently skipping shutdown signals.
        let mut pids = session_processes(child_pid);
        if pids.is_empty() {
            pids.push(child_pid);
        }
        return pids
            .into_iter()
            .map(|pid| ShutdownTarget { pid, fd: None })
            .collect();
    }
    // ESRCH and friends: the child is already gone. A dead child needs no
    // signals, and its recorded pid may already name a stranger — the old
    // `pids.push(child_pid)` fallback here was exactly the ABA hazard.
    Vec::new()
}

/// Collect identity-pinned shutdown targets for a pane child.
///
/// Order of operations is the whole point: each target's pidfd is opened
/// FIRST and its kernel session id verified AFTER, so the fd pins the very
/// incarnation the verification saw (no scan→signal TOCTOU window). If the
/// child's session id does not equal its pid (PTY children are session
/// leaders; a mismatch means either pid reuse or an unusual spawn path), the
/// session sweep is skipped and only the pinned child itself is targeted —
/// leaking grandchildren is the fail-safe direction, signalling strangers is
/// not.
pub fn session_shutdown_targets(child_pid: u32) -> Vec<ShutdownTarget> {
    if child_pid == 0 {
        return Vec::new();
    }
    let child_fd = match pidfd_open(child_pid) {
        Ok(fd) => fd,
        Err(err) => return targets_after_child_open_failure(child_pid, &err),
    };
    if process_session_id(child_pid) != Some(child_pid as i32) {
        return vec![ShutdownTarget {
            pid: child_pid,
            fd: Some(child_fd),
        }];
    }
    let mut targets = Vec::new();
    for pid in session_processes(child_pid) {
        if pid == child_pid {
            continue;
        }
        let Ok(fd) = pidfd_open(pid) else {
            continue; // exited between scan and open — nothing to pin
        };
        // Verify AFTER the open: the fd pins the incarnation this check sees.
        if process_session_id(pid) == Some(child_pid as i32) {
            targets.push(ShutdownTarget { pid, fd: Some(fd) });
        }
    }
    targets.push(ShutdownTarget {
        pid: child_pid,
        fd: Some(child_fd),
    });
    targets
}

pub fn signal_targets(targets: &[ShutdownTarget], signal: Signal) {
    let sig = match signal {
        Signal::Hangup => libc::SIGHUP,
        Signal::Terminate => libc::SIGTERM,
        Signal::Kill => libc::SIGKILL,
    };
    for target in targets {
        match &target.fd {
            Some(fd) => {
                let _ = pidfd_send_signal(fd, sig);
            }
            None => signal_processes(&[target.pid], signal),
        }
    }
}

pub fn target_alive(target: &ShutdownTarget) -> bool {
    match &target.fd {
        // Signal 0 through the pidfd: existence probe that can never race
        // onto a recycled pid (mirrors kill(pid, 0) semantics, incl. zombies).
        Some(fd) => pidfd_send_signal(fd, 0),
        None => process_exists(target.pid),
    }
}

pub fn write_clipboard(bytes: &[u8]) -> bool {
    for command in clipboard_commands() {
        if run_clipboard_command(&command, bytes) {
            return true;
        }
    }
    false
}

pub fn read_clipboard_text() -> Option<String> {
    for command in read_clipboard_text_commands() {
        if let Some(text) = read_clipboard_text_with_command(&command) {
            return Some(text);
        }
    }
    None
}

/// Ceilings for the bookmark source. It is an external, hand-editable file, so
/// a runaway or hostile one must not turn into unbounded startup work in the
/// locations model.
const DESKTOP_BOOKMARKS_MAX_BYTES: u64 = 64 * 1024;
const DESKTOP_BOOKMARKS_MAX_ENTRIES: usize = 128;

/// Parse the freedesktop bookmark list shared by GTK file managers: one URI per
/// line, optionally followed by a space and a display label.
///
/// Order is preserved because the user arranged it. Remote schemes are dropped
/// rather than kept as dead rows — this surface navigates local paths only.
fn parse_desktop_bookmarks(contents: &str) -> Vec<super::DesktopBookmark> {
    contents
        .lines()
        .filter_map(|line| {
            let (uri, label) = match line.trim().split_once(' ') {
                Some((uri, label)) => (uri, Some(label.trim())),
                None => (line.trim(), None),
            };
            let path = percent_decoded_path(uri.strip_prefix("file://")?)?;
            path.is_absolute().then(|| super::DesktopBookmark {
                path,
                label: label
                    .filter(|label| !label.is_empty())
                    .map(ToOwned::to_owned),
            })
        })
        .take(DESKTOP_BOOKMARKS_MAX_ENTRIES)
        .collect()
}

/// Decode the URI escapes a real desktop path carries. A malformed escape
/// invalidates the whole entry rather than being passed through, so a truncated
/// line cannot resolve to a different directory than the user bookmarked.
fn percent_decoded_path(encoded: &str) -> Option<PathBuf> {
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        let digits = bytes.get(index + 1..index + 3)?;
        if !digits.iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        decoded.push(u8::from_str_radix(std::str::from_utf8(digits).ok()?, 16).ok()?);
        index += 3;
    }
    String::from_utf8(decoded).ok().map(PathBuf::from)
}

/// Candidate bookmark files, most current first. Nautilus, Nemo, Thunar and
/// PCManFM all read and write the GTK 3 list, so the rail follows whichever of
/// them the user actually curates their sidebar in.
/// The localized user-directory list is external, user-editable text, so it is
/// bounded before it can grow startup work — the same ceiling the bookmark
/// list is read under.
const USER_DIRS_MAX_BYTES: u64 = 64 * 1024;

/// Reads the freedesktop user-directory list, refusing anything past the
/// ceiling rather than letting a runaway file into startup.
fn read_user_dirs(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    if metadata.len() > USER_DIRS_MAX_BYTES {
        tracing::warn!(
            ?path,
            bytes = metadata.len(),
            ceiling = USER_DIRS_MAX_BYTES,
            "user directory list exceeds the read ceiling; ignoring it"
        );
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Projects the recorded lines onto the directories this rail draws. Lines that
/// name nothing this surface publishes are simply not this function's business.
fn parse_user_directories(contents: &str, home: &Path) -> Vec<super::UserDirectory> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let kind = super::UserDirectoryKind::ALL
                .into_iter()
                .find(|kind| kind.config_key() == key.trim())?;
            let path = recorded_user_directory_path(value.trim(), home)?;
            Some(super::UserDirectory { kind, path })
        })
        .collect()
}

/// `user-dirs.dirs(5)`: the value is `"$HOME/Path"` or `"/Path"`. Anything else
/// names a place this reader cannot resolve, and a place it cannot resolve is
/// worse than the unlocalized default it would otherwise fall back to.
fn recorded_user_directory_path(value: &str, home: &Path) -> Option<PathBuf> {
    let value = value.trim_matches('"');
    if value.is_empty() {
        return None;
    }
    if let Some(rest) = value.strip_prefix("$HOME") {
        let rest = rest.trim_start_matches('/');
        return Some(if rest.is_empty() {
            home.to_path_buf()
        } else {
            home.join(rest)
        });
    }
    let path = PathBuf::from(value);
    path.is_absolute().then_some(path)
}

/// Completes a partial list with the unlocalized defaults, so a host that
/// records only some of its directories still yields a full built-in block.
fn merged_user_directories(
    parsed: Vec<super::UserDirectory>,
    home: &Path,
) -> Vec<super::UserDirectory> {
    super::well_known_user_directories(home)
        .into_iter()
        .map(|fallback| {
            parsed
                .iter()
                .find(|recorded| recorded.kind == fallback.kind)
                .cloned()
                .unwrap_or(fallback)
        })
        .collect()
}

pub(crate) fn user_directories(home: &Path) -> Vec<super::UserDirectory> {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| Some(home.join(".config")));
    let parsed = config_home
        .map(|dir| dir.join("user-dirs.dirs"))
        .and_then(|path| read_user_dirs(&path))
        .map(|contents| parse_user_directories(&contents, home))
        .unwrap_or_default();
    merged_user_directories(parsed, home)
}

fn desktop_bookmark_files() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute());
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| home.as_ref().map(|home| home.join(".config")));

    let mut candidates = Vec::with_capacity(2);
    candidates.extend(config_home.map(|dir| dir.join("gtk-3.0").join("bookmarks")));
    candidates.extend(home.map(|dir| dir.join(".gtk-bookmarks")));
    candidates
}

/// Bookmarks curated in the host file manager, in the order the user arranged
/// them. The first readable candidate is authoritative: an empty current list
/// means the user emptied it, not that a legacy file should be resurrected.
pub(crate) fn desktop_bookmarks() -> Vec<super::DesktopBookmark> {
    for path in desktop_bookmark_files() {
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        if metadata.len() > DESKTOP_BOOKMARKS_MAX_BYTES {
            tracing::warn!(
                ?path,
                bytes = metadata.len(),
                ceiling = DESKTOP_BOOKMARKS_MAX_BYTES,
                "desktop bookmarks file exceeds the read ceiling; ignoring it"
            );
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(contents) => return parse_desktop_bookmarks(&contents),
            Err(err) => tracing::debug!(?path, %err, "reading desktop bookmarks failed"),
        }
    }
    Vec::new()
}

/// Root of the GVfs mounts the user currently has.
///
/// Presence is established from the runtime directory listing rather than by
/// stat-ing the mount root. `XDG_RUNTIME_DIR` is a tmpfs, so its `read_dir`
/// answers from the kernel, while stat-ing the mount root itself enters the
/// FUSE filesystem and blocks indefinitely when `gvfsd-fuse` has died. This
/// runs during startup model preparation, where that stall would be a hang.
pub(crate) fn network_mounts_root() -> Option<PathBuf> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())?;
    std::fs::read_dir(&runtime_dir)
        .ok()?
        .flatten()
        .any(|entry| entry.file_name() == std::ffi::OsStr::new("gvfs"))
        .then(|| runtime_dir.join("gvfs"))
}

pub fn open_url(url: &str) -> std::io::Result<()> {
    Command::new("xdg-open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn read_clipboard_image() -> Option<ClipboardImage> {
    for (mime, extension) in [
        ("image/png", "png"),
        ("image/jpeg", "jpg"),
        ("image/jpg", "jpg"),
        ("image/gif", "gif"),
        ("image/webp", "webp"),
        ("image/bmp", "bmp"),
    ] {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            if let Some(image) =
                read_validated_clipboard_image("wl-paste", &["--type", mime], extension)
            {
                return Some(image);
            }
        }

        if std::env::var_os("DISPLAY").is_some() {
            if let Some(image) = read_validated_clipboard_image(
                "xclip",
                &["-selection", "clipboard", "-t", mime, "-o"],
                extension,
            ) {
                return Some(image);
            }
        }
    }

    None
}

fn read_validated_clipboard_image(
    program: &str,
    args: &[&str],
    extension: &'static str,
) -> Option<ClipboardImage> {
    let bytes = read_clipboard_image_with_command(program, args)?;
    if !bytes_match_image_signature(extension, &bytes) {
        return None;
    }
    Some(ClipboardImage { bytes, extension })
}

fn bytes_match_image_signature(extension: &str, bytes: &[u8]) -> bool {
    match extension {
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WEBP",
        "bmp" => {
            if bytes.len() < 26 || !bytes.starts_with(b"BM") {
                return false;
            }
            let offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
            (26..=bytes.len()).contains(&offset)
        }
        _ => false,
    }
}

/// Show a native desktop notification through libnotify's command-line helper.
pub fn show_desktop_notification(title: &str, body: Option<&str>) -> std::io::Result<bool> {
    show_desktop_notification_with_command(title, body, |program| Command::new(program))
}

fn show_desktop_notification_with_command(
    title: &str,
    body: Option<&str>,
    mut command: impl FnMut(&str) -> Command,
) -> std::io::Result<bool> {
    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        return Ok(false);
    }

    let mut cmd = command("notify-send");
    cmd.arg("--").arg(title);
    if let Some(body) = body.filter(|body| !body.is_empty()) {
        cmd.arg(body);
    }
    run_notification_command(cmd)
}

fn run_notification_command(mut command: Command) -> std::io::Result<bool> {
    let status = match command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => status,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err),
    };

    Ok(status.success())
}

fn read_clipboard_image_with_command(program: &str, args: &[&str]) -> Option<Vec<u8>> {
    let mut command = Command::new(program);
    command.args(args);
    read_clipboard_image_with_spawned_command(command)
}

fn read_clipboard_image_with_spawned_command(command: Command) -> Option<Vec<u8>> {
    read_clipboard_image_with_spawned_command_max(
        command,
        crate::protocol::MAX_CLIPBOARD_IMAGE_PAYLOAD,
    )
}

fn read_clipboard_image_with_spawned_command_max(
    mut command: Command,
    max_bytes: usize,
) -> Option<Vec<u8>> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;

    let read = match read_limited_reader(stdout, max_bytes) {
        Ok(read) => read,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };

    if read == LimitedRead::Oversized {
        let _ = child.kill();
        let _ = child.wait();
        return None;
    }

    let status = child.wait().ok()?;
    if !status.success() {
        return None;
    }

    match read {
        LimitedRead::Complete(bytes) => Some(bytes),
        LimitedRead::Empty | LimitedRead::Oversized => None,
    }
}

fn clipboard_commands() -> Vec<ClipboardCommand> {
    let mut commands = Vec::new();

    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        commands.push(ClipboardCommand {
            program: "wl-copy",
            args: &["--type", "text/plain;charset=utf-8"],
        });
    }

    if std::env::var_os("DISPLAY").is_some() {
        commands.push(ClipboardCommand {
            program: "xclip",
            args: &["-selection", "clipboard", "-in"],
        });
        commands.push(ClipboardCommand {
            program: "xsel",
            args: &["--clipboard", "--input"],
        });
    }

    commands
}

fn read_clipboard_text_commands() -> Vec<ClipboardCommand> {
    let mut commands = Vec::new();

    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        commands.push(ClipboardCommand {
            program: "wl-paste",
            args: &["--type", "text/plain;charset=utf-8"],
        });
        commands.push(ClipboardCommand {
            program: "wl-paste",
            args: &["--type", "text/plain"],
        });
    }

    if std::env::var_os("DISPLAY").is_some() {
        commands.push(ClipboardCommand {
            program: "xclip",
            args: &["-selection", "clipboard", "-out"],
        });
        commands.push(ClipboardCommand {
            program: "xsel",
            args: &["--clipboard", "--output"],
        });
    }

    commands
}

fn read_clipboard_text_with_command(command: &ClipboardCommand) -> Option<String> {
    const MAX_CLIPBOARD_TEXT_BYTES: usize = 1024 * 1024;

    let mut child = Command::new(command.program)
        .args(command.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let stdout = child.stdout.take()?;
    let read = match read_limited_reader(stdout, MAX_CLIPBOARD_TEXT_BYTES) {
        Ok(LimitedRead::Oversized) => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        Ok(read) => read,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };

    let status = child.wait().ok()?;
    if !status.success() {
        return None;
    }

    match read {
        LimitedRead::Complete(bytes) => String::from_utf8(bytes).ok(),
        LimitedRead::Empty => None,
        LimitedRead::Oversized => unreachable!("oversized clipboard text is handled before wait"),
    }
}

fn run_clipboard_command(command: &ClipboardCommand, bytes: &[u8]) -> bool {
    run_clipboard_command_with_timeout(command, bytes, CLIPBOARD_HANDOFF_TIMEOUT)
}

/// Writes `bytes` to a clipboard helper and waits only until `timeout`.
///
/// Clipboard helpers are not fully under our control: `wl-copy` must acquire
/// clipboard ownership from the Wayland compositor before it forks into the
/// background, so a slow or throttled compositor stretches that handshake. An
/// unbounded wait turns that transient delay into a permanent lock of the
/// calling thread — the TUI client then stops processing input entirely.
///
/// Past the timeout the child is handed to a background reaper: the copy is
/// reported as delivered (ownership is presumed taken, so we must not spawn a
/// competing X11 writer) while the helper finishes on its own.
fn run_clipboard_command_with_timeout(
    command: &ClipboardCommand,
    bytes: &[u8],
    timeout: std::time::Duration,
) -> bool {
    let mut child = match Command::new(command.program)
        .args(command.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };

    let Some(mut stdin) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    };

    if stdin.write_all(bytes).is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return false;
    }
    drop(stdin);

    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    hand_off_clipboard_child(child);
                    return true;
                }
                std::thread::sleep(CLIPBOARD_POLL_INTERVAL);
            }
            Err(_) => return false,
        }
    }
}

/// Owns a handed-off clipboard helper until it exits, so a bounded wait never
/// trades a UI lock for a leaked zombie process.
fn hand_off_clipboard_child(mut child: std::process::Child) {
    let pid = child.id();
    if std::thread::Builder::new()
        .name("clipboard-reap".to_owned())
        .spawn(move || {
            let _ = child.wait();
        })
        .is_err()
    {
        tracing::debug!(pid, "could not spawn clipboard reaper thread");
    }
}

fn process_session_id(pid: u32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = stat.get(stat.rfind(')')? + 2..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    fields.get(3)?.parse().ok()
}

/// Reads the kernel's cumulative CPU counters.
///
/// Two files, read whole and parsed elsewhere. The read is deliberately the
/// only thing that happens here: the arithmetic that turns these counters into
/// a percentage is pure and tested against fixtures, on every platform,
/// including the ones where this function does not exist.
///
/// A missing or unreadable `/proc` is `None`, not zero. Containers and hardened
/// kernels can hide either file, and a meter that reads 0% there would be
/// lying about an idle machine rather than admitting it cannot see.
// TP-RES-05: reading is separated from arithmetic, and failure is None.
pub(crate) fn read_cpu_times() -> Option<crate::resource::CpuTimes> {
    let text = std::fs::read_to_string("/proc/stat").ok()?;
    crate::resource::parse_proc_stat(&text)
}

/// Reads memory and swap totals. See `read_cpu_times` for why this is a read
/// and nothing else.
pub(crate) fn read_memory() -> (
    Option<crate::resource::Usage>,
    Option<crate::resource::Usage>,
) {
    let Ok(text) = std::fs::read_to_string("/proc/meminfo") else {
        return (None, None);
    };
    crate::resource::parse_proc_meminfo(&text)
}

/// Space on the filesystem herdr is running from.
///
/// The filesystem holding the current directory, not `/`: somebody watching
/// disk space is watching the one their work is on, and on a machine with
/// `/home` mounted separately those are different numbers.
pub(crate) fn read_disk() -> Option<crate::resource::Usage> {
    let path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c_path` is a NUL-terminated C string that outlives the call, and
    // `stats` is a live, correctly aligned `statvfs` this thread owns
    // exclusively. `statvfs` writes only into it and reports failure through its
    // return value, which is checked before anything is read back out.
    let ok = unsafe { libc::statvfs(c_path.as_ptr(), &mut stats) } == 0;
    if !ok {
        return None;
    }
    // `f_frsize` is the unit the block counts are in. `f_bsize` is the
    // preferred I/O size and is not always the same number.
    let block = stats.f_frsize as u64;
    let total = (stats.f_blocks as u64).checked_mul(block)?;
    // Blocks available to us, not blocks free: the difference is the root
    // reserve, which is neither ours to use nor in use by anybody. Counting it
    // as used is what `df` does, and agreeing with `df` matters more here than
    // any other definition of "used".
    let available = (stats.f_bavail as u64).checked_mul(block)?;
    Some(crate::resource::Usage {
        used: total.saturating_sub(available),
        total,
    })
}

/// Charge remaining, from the first battery the kernel reports.
///
/// The first rather than a sum: a laptop with two batteries reports each
/// separately, and averaging them would show a figure neither one is at. The
/// name is sorted so `BAT0` wins whatever order the directory comes back in — a
/// bar that showed a different battery between two boots would be reporting a
/// different machine.
pub(crate) fn read_battery() -> Option<f32> {
    let entries = std::fs::read_dir("/sys/class/power_supply").ok()?;
    let mut names = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .filter(|name| name.to_string_lossy().starts_with("BAT"))
        .collect::<Vec<_>>();
    names.sort();
    let name = names.first()?;
    let text = std::fs::read_to_string(
        std::path::Path::new("/sys/class/power_supply")
            .join(name)
            .join("capacity"),
    )
    .ok()?;
    text.trim()
        .parse::<f32>()
        .ok()
        .filter(|pct| (0.0..=100.0).contains(pct))
}

/// Bytes carried since boot, across every interface but loopback.
pub(crate) fn read_net_total() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/net/dev").ok()?;
    crate::resource::parse_proc_net_dev(&text)
}

/// The warmest thermal zone the kernel exposes.
pub(crate) fn read_temperature() -> Option<f32> {
    let entries = std::fs::read_dir("/sys/class/thermal").ok()?;
    let readings = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("thermal_zone")
        })
        .filter_map(|entry| std::fs::read_to_string(entry.path().join("temp")).ok())
        .filter_map(|text| text.trim().parse::<i64>().ok());
    crate::resource::warmest_millidegrees(readings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use std::{cell::RefCell, collections::HashMap};

    // TP-FDB-PARSE-01: the host bookmark list is user-editable text, so its
    // reader keeps file order, honours renamed entries, decodes the URI escapes
    // that real desktop paths carry, and drops what this surface cannot open.
    #[test]
    fn desktop_bookmarks_preserve_order_labels_and_decoded_paths() {
        let parsed = parse_desktop_bookmarks(concat!(
            "file:///home/user/MnMVeldOps MnMVeldOps\n",
            "file:///home/user/Desktop/Project%20Meta%20Genesis Project Meta Genesis\n",
            "file:///home/user/Asus-Downloads ASUS Downloads (Tailscale)\n",
            "file:///home/user/projects\n",
            "\n",
            "   \n",
            "smb://server/share Shared\n",
            "file://localhost/relative\n",
            "file:///home/user/broken%2 Broken escape\n",
        ));

        assert_eq!(
            parsed
                .iter()
                .map(|bookmark| (
                    bookmark.path.to_string_lossy().into_owned(),
                    bookmark.label.clone()
                ))
                .collect::<Vec<_>>(),
            [
                (
                    "/home/user/MnMVeldOps".to_string(),
                    Some("MnMVeldOps".to_string())
                ),
                (
                    "/home/user/Desktop/Project Meta Genesis".to_string(),
                    Some("Project Meta Genesis".to_string())
                ),
                (
                    "/home/user/Asus-Downloads".to_string(),
                    Some("ASUS Downloads (Tailscale)".to_string())
                ),
                ("/home/user/projects".to_string(), None),
            ],
            "remote schemes, non-absolute hosts and malformed escapes are not navigable here"
        );
    }

    // TP-FDB-PARSE-02: the file is an external input. A hostile or runaway one
    // cannot grow unbounded startup work in the locations model.
    #[test]
    fn desktop_bookmarks_are_bounded_by_entry_ceiling() {
        let contents = (0..DESKTOP_BOOKMARKS_MAX_ENTRIES + 64)
            .map(|index| format!("file:///virtual/{index}\n"))
            .collect::<String>();

        assert_eq!(
            parse_desktop_bookmarks(&contents).len(),
            DESKTOP_BOOKMARKS_MAX_ENTRIES
        );
    }

    fn user_dir(kind: super::super::UserDirectoryKind, path: &str) -> super::super::UserDirectory {
        super::super::UserDirectory {
            kind,
            path: PathBuf::from(path),
        }
    }

    // TP-FDB-XDG-01: the well-known directories are localized per path element
    // on the host, so their real names are read from the list the desktop
    // records rather than assumed to be the English ones. Assuming English
    // leaves the built-in block empty on every desktop that is not English.
    #[test]
    fn user_directories_follow_the_localized_names_the_host_recorded() {
        use super::super::UserDirectoryKind as Kind;

        let parsed = parse_user_directories(
            concat!(
                "# This file is written by xdg-user-dirs-update\n",
                "XDG_DESKTOP_DIR=\"$HOME/Masaüstü\"\n",
                "XDG_DOWNLOAD_DIR=\"$HOME/İndirilenler\"\n",
                "XDG_DOCUMENTS_DIR=\"/mnt/veri/Belgeler\"\n",
            ),
            Path::new("/home/user"),
        );

        assert_eq!(
            parsed,
            vec![
                user_dir(Kind::Desktop, "/home/user/Masaüstü"),
                user_dir(Kind::Downloads, "/home/user/İndirilenler"),
                user_dir(Kind::Documents, "/mnt/veri/Belgeler"),
            ],
            "localized names and absolute overrides are both what the host means"
        );
    }

    // TP-FDB-XDG-02: the list is external input. Comments, keys this surface
    // does not publish, and malformed lines are skipped without taking the
    // readable entries down with them, and a runaway file is refused outright.
    #[test]
    fn user_directories_survive_comments_unknown_keys_and_malformed_lines() {
        use super::super::UserDirectoryKind as Kind;

        let parsed = parse_user_directories(
            concat!(
                "# a comment\n",
                "XDG_TEMPLATES_DIR=\"$HOME/Şablonlar\"\n",
                "XDG_DOWNLOAD_DIR\n",
                "XDG_PICTURES_DIR=\"\"\n",
                "XDG_MUSIC_DIR=\"$HOME/Müzik\"\n",
                "   \n",
            ),
            Path::new("/home/user"),
        );

        assert_eq!(
            parsed,
            vec![user_dir(Kind::Music, "/home/user/Müzik")],
            "one unusable line must not cost the rail the lines around it"
        );
    }

    #[test]
    fn a_runaway_user_directory_list_is_refused_before_it_is_parsed() {
        let dir = std::env::temp_dir().join(format!(
            "herdr-user-dirs-{}-{}",
            std::process::id(),
            "ceiling"
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("user-dirs.dirs");
        // Sized from the line itself rather than an assumed width, so the
        // fixture cannot drift under the ceiling it is meant to exceed.
        let line = "XDG_DOWNLOAD_DIR=\"$HOME/Downloads\"\n";
        let bloat = line.repeat((USER_DIRS_MAX_BYTES as usize / line.len()) + 64);
        std::fs::write(&path, &bloat).expect("write");

        let refused = read_user_dirs(&path);
        std::fs::remove_dir_all(&dir).ok();

        assert!(
            refused.is_none(),
            "a file past the ceiling must not reach the parser"
        );
    }

    // A host that records nothing localized still gets a full built-in block:
    // the unlocalized freedesktop names are the fallback, which is exactly the
    // behaviour that existed before the localized list was read at all.
    #[test]
    fn user_directories_fall_back_to_the_unlocalized_names() {
        use super::super::UserDirectoryKind as Kind;

        let merged = merged_user_directories(Vec::new(), Path::new("/home/user"));

        assert_eq!(
            merged,
            vec![
                user_dir(Kind::Desktop, "/home/user/Desktop"),
                user_dir(Kind::Downloads, "/home/user/Downloads"),
                user_dir(Kind::Documents, "/home/user/Documents"),
                user_dir(Kind::Pictures, "/home/user/Pictures"),
                user_dir(Kind::Videos, "/home/user/Videos"),
                user_dir(Kind::Music, "/home/user/Music"),
            ]
        );
    }

    #[test]
    fn a_partial_localized_list_is_completed_rather_than_truncated() {
        use super::super::UserDirectoryKind as Kind;

        let merged = merged_user_directories(
            vec![user_dir(Kind::Downloads, "/home/user/İndirilenler")],
            Path::new("/home/user"),
        );

        assert_eq!(
            merged,
            vec![
                user_dir(Kind::Desktop, "/home/user/Desktop"),
                user_dir(Kind::Downloads, "/home/user/İndirilenler"),
                user_dir(Kind::Documents, "/home/user/Documents"),
                user_dir(Kind::Pictures, "/home/user/Pictures"),
                user_dir(Kind::Videos, "/home/user/Videos"),
                user_dir(Kind::Music, "/home/user/Music"),
            ],
            "the recorded entry wins its slot; the rest keep their published order"
        );
    }

    // ── F1: identity-safe shutdown targets (PRD session-collapse-hardening) ──

    /// Anchors the assumption `session_shutdown_targets` builds on: a PTY
    /// child is its own kernel session leader (sid == pid). If a spawn-path
    /// change ever breaks this, the sweep silently degrades to single-child
    /// mode — this test makes that degradation loud instead.
    #[test]
    fn pty_child_is_its_own_session_leader() {
        let pty = portable_pty::native_pty_system()
            .openpty(portable_pty::PtySize::default())
            .expect("openpty");
        let mut cmd = portable_pty::CommandBuilder::new("sleep");
        cmd.arg("300");
        let mut child = pty.slave.spawn_command(cmd).expect("spawn");
        let pid = child.process_id().expect("pid");

        assert_eq!(
            process_session_id(pid),
            Some(pid as i32),
            "PTY child must be a session leader"
        );

        let targets = session_shutdown_targets(pid);
        assert!(
            targets.iter().any(|t| t.pid() == pid),
            "sweep must include the child itself"
        );

        child.kill().ok();
        child.wait().ok();
    }

    /// F1-T1 + mismatch yolu: a non-setsid child (plain std::process spawn)
    /// has sid != pid, so the sweep must pin ONLY the child — and the signal
    /// must be delivered through the pidfd.
    #[test]
    fn shutdown_targets_signal_via_pidfd_and_terminate() {
        use std::os::unix::process::ExitStatusExt;
        let mut child = Command::new("sleep")
            .arg("300")
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn sleep");
        let pid = child.id();

        let targets = session_shutdown_targets(pid);
        assert_eq!(
            targets.len(),
            1,
            "non-leader child must yield exactly the pinned child, no session sweep"
        );
        assert_eq!(targets[0].pid(), pid);

        signal_targets(&targets, Signal::Terminate);
        let status = child.wait().expect("wait");
        assert_eq!(status.signal(), Some(libc::SIGTERM));
    }

    /// F1-T2: a handle whose process died (and was reaped) reports dead and
    /// signalling it is a no-op — never a hit on a recycled pid.
    #[test]
    fn dead_target_alive_is_false_and_signal_is_noop() {
        let mut child = Command::new("sleep")
            .arg("300")
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn sleep");
        let pid = child.id();

        let targets = session_shutdown_targets(pid);
        assert_eq!(targets.len(), 1);
        assert!(target_alive(&targets[0]), "child is alive before kill");

        child.kill().expect("kill");
        child.wait().expect("wait");

        assert!(!target_alive(&targets[0]), "reaped child must read as dead");
        // Must not panic and must not signal anything else.
        signal_targets(&targets, Signal::Terminate);
    }

    /// F1-T4 (ABA çekirdeği): a child that exited and was reaped BEFORE
    /// collection yields ZERO targets. The pre-fix behavior pushed the stale
    /// pid into the kill set — exactly the recycled-pid hazard from
    /// PM-2026-07-27-001. (Theoretical flake: the pid would have to be
    /// recycled within microseconds across a ~4M pid space to break this.)
    #[test]
    fn stale_child_pid_yields_no_targets() {
        let mut child = Command::new("true")
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn true");
        let pid = child.id();
        child.wait().expect("wait");

        let targets = session_shutdown_targets(pid);
        assert!(
            targets.is_empty(),
            "stale (reaped) child pid must produce no shutdown targets"
        );
    }

    /// F1-T5: kernel-capability error mapping — ENOSYS keeps the legacy
    /// pid-based path (old kernels must not lose shutdown signals), ESRCH
    /// yields nothing (dead child needs none).
    #[test]
    fn child_open_failure_mapping_enosys_vs_esrch() {
        let enosys = std::io::Error::from_raw_os_error(libc::ENOSYS);
        let esrch = std::io::Error::from_raw_os_error(libc::ESRCH);

        let legacy = targets_after_child_open_failure(std::process::id(), &enosys);
        assert!(
            legacy.iter().any(|t| t.pid() == std::process::id()),
            "ENOSYS must fall back to the legacy pid set"
        );
        assert!(
            legacy
                .iter()
                .all(|t| target_alive(t) || t.pid() != std::process::id()),
            "legacy targets must still be pid-probeable"
        );

        assert!(
            targets_after_child_open_failure(999_999_999, &esrch).is_empty(),
            "ESRCH must yield an empty target set"
        );
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn wsl_marker_detection_matches_kernel_release_text() {
        assert!(text_indicates_wsl("5.15.167.4-microsoft-standard-WSL2"));
        assert!(text_indicates_wsl("4.4.0-19041-Microsoft"));
        assert!(!text_indicates_wsl("6.8.0-64-generic"));
        assert!(!text_indicates_wsl(""));
    }

    #[test]
    fn foreground_members_follow_the_pane_tree_and_filter_by_process_group() {
        let tasks = HashMap::from([
            (100, vec![100, 101]),
            (200, vec![200]),
            (201, vec![201]),
            (210, vec![210]),
            (220, vec![220]),
            (221, vec![221]),
            (300, vec![300]),
        ]);
        let children = HashMap::from([
            ((100, 100), vec![200, 201, 300]),
            ((100, 101), vec![210]),
            ((200, 200), vec![220]),
            ((220, 220), vec![221]),
        ]);
        let processes = HashMap::from([
            (100, (100, "shell")),
            (200, (200, "leader")),
            (201, (200, "pipeline")),
            (210, (200, "thread-child")),
            (220, (220, "intermediate")),
            (221, (200, "nested-agent")),
            (300, (300, "background")),
            (9999, (200, "unrelated-host-process")),
        ]);
        let task_reads = RefCell::new(Vec::new());
        let child_reads = RefCell::new(Vec::new());
        let member_reads = RefCell::new(Vec::new());

        let members = foreground_process_group_members_with(
            100,
            200,
            |pid| {
                task_reads.borrow_mut().push(pid);
                tasks.get(&pid).cloned().unwrap_or_default()
            },
            |pid, tid| {
                child_reads.borrow_mut().push((pid, tid));
                children.get(&(pid, tid)).cloned().unwrap_or_default()
            },
            |process_group_id, pid| {
                member_reads.borrow_mut().push(pid);
                let (pgrp, comm) = processes.get(&pid)?;
                (*pgrp == process_group_id).then(|| ProcGroupMember {
                    pid,
                    comm: (*comm).to_string(),
                })
            },
        )
        .unwrap();

        assert_eq!(
            members
                .into_iter()
                .map(|member| (member.pid, member.comm))
                .collect::<Vec<_>>(),
            vec![
                (200, "leader".to_string()),
                (201, "pipeline".to_string()),
                (210, "thread-child".to_string()),
                (221, "nested-agent".to_string()),
            ]
        );
        assert!(child_reads.borrow().contains(&(100, 101)));
        assert!(task_reads.borrow().contains(&220));
        assert!(!task_reads.borrow().contains(&9999));
        assert!(!member_reads.borrow().contains(&9999));
    }

    #[test]
    fn foreground_members_degrade_to_the_direct_group_leader() {
        let members = foreground_process_group_members_with(
            100,
            200,
            |_| Vec::new(),
            |_, _| Vec::new(),
            |process_group_id, pid| {
                (pid == process_group_id).then(|| ProcGroupMember {
                    pid,
                    comm: "leader".to_string(),
                })
            },
        )
        .unwrap();

        assert_eq!(
            members,
            vec![ProcGroupMember {
                pid: 200,
                comm: "leader".to_string()
            }]
        );
    }

    #[test]
    fn foreground_members_observe_new_children_without_a_snapshot_cache() {
        let children = RefCell::new(HashMap::from([((100, 100), vec![200])]));
        let discover = || {
            foreground_process_group_members_with(
                100,
                200,
                |pid| vec![pid],
                |pid, tid| {
                    children
                        .borrow()
                        .get(&(pid, tid))
                        .cloned()
                        .unwrap_or_default()
                },
                |process_group_id, pid| {
                    [200, 201]
                        .contains(&pid)
                        .then(|| ProcGroupMember {
                            pid,
                            comm: format!("member-{pid}"),
                        })
                        .filter(|_| process_group_id == 200)
                },
            )
            .unwrap()
            .into_iter()
            .map(|member| member.pid)
            .collect::<Vec<_>>()
        };

        assert_eq!(discover(), vec![200]);
        children.borrow_mut().insert((100, 100), vec![200, 201]);
        assert_eq!(discover(), vec![200, 201]);
    }

    #[test]
    fn proc_stat_parsing_keeps_group_leader_inputs_live() {
        assert_eq!(
            process_pgrp_and_comm_from_stat("123 (name with ) paren) S 1 456 789 0 456"),
            Some((456, "name with ) paren".to_string()))
        );
    }

    #[test]
    fn clipboard_commands_prefer_wayland_when_available() {
        let _guard = env_lock().lock().unwrap();
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
            std::env::remove_var("DISPLAY");
        }
        let commands = clipboard_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].program, "wl-copy");
    }

    #[test]
    fn clipboard_commands_include_x11_fallbacks() {
        let _guard = env_lock().lock().unwrap();
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
            std::env::set_var("DISPLAY", ":0");
        }
        let commands = clipboard_commands();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].program, "xclip");
        assert_eq!(commands[1].program, "xsel");
    }

    #[test]
    fn read_clipboard_text_commands_include_session_backends() {
        let _guard = env_lock().lock().unwrap();
        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
            std::env::set_var("DISPLAY", ":0");
        }

        let commands = read_clipboard_text_commands();
        assert_eq!(commands[0].program, "wl-paste");
        assert_eq!(commands[1].program, "wl-paste");
        assert_eq!(commands[2].program, "xclip");
        assert_eq!(commands[3].program, "xsel");
    }

    // ── clipboard handoff must not block the caller ──────────────────────────
    //
    // `wl-copy` acquires clipboard ownership from the Wayland compositor before
    // it forks into the background. When the compositor is slow (CPU pressure,
    // throttled cgroup) that handshake stalls, and an unbounded `wait()` turns a
    // transient delay into a permanent UI lock: the client's main thread parks in
    // `do_wait` and stops processing input. Measured 2026-07-29: a stalled
    // `wl-copy` child (ppid = herdr client) held the client for 2m50s until it
    // was killed; normal handoff completes in ~104ms.

    /// The regression guard for that lock: a helper that never exits must not
    /// hold the caller. Without the bounded wait this test hangs for 60s.
    #[test]
    fn clipboard_command_does_not_block_on_hanging_helper() {
        let command = ClipboardCommand {
            program: "sleep",
            args: &["60"],
        };

        let started = std::time::Instant::now();
        let handed_off = run_clipboard_command_with_timeout(
            &command,
            b"payload",
            std::time::Duration::from_millis(100),
        );
        let elapsed = started.elapsed();

        // Reported as delivered: ownership is presumed taken, so the caller must
        // NOT fall through to the X11 helpers and spawn a competing writer.
        assert!(handed_off);
        assert!(
            elapsed < std::time::Duration::from_millis(2_000),
            "bounded wait exceeded: {elapsed:?}"
        );
    }

    /// A helper that exits quickly keeps the previous behaviour: reaped inline,
    /// success reported from its exit status.
    #[test]
    fn clipboard_command_succeeds_for_fast_helper() {
        let command = ClipboardCommand {
            program: "cat",
            args: &[],
        };

        assert!(run_clipboard_command_with_timeout(
            &command,
            b"payload",
            std::time::Duration::from_millis(2_000)
        ));
    }

    /// Spawn failure must stay falsy so the xclip/xsel fallback chain still runs.
    #[test]
    fn clipboard_command_fails_for_missing_program() {
        let command = ClipboardCommand {
            program: "herdr-clipboard-helper-that-does-not-exist",
            args: &[],
        };

        assert!(!run_clipboard_command_with_timeout(
            &command,
            b"payload",
            std::time::Duration::from_millis(200)
        ));
    }

    /// A helper that exits non-zero must report failure, not a false success.
    #[test]
    fn clipboard_command_fails_for_nonzero_exit() {
        let command = ClipboardCommand {
            program: "sh",
            args: &["-c", "exit 3"],
        };

        assert!(!run_clipboard_command_with_timeout(
            &command,
            b"payload",
            std::time::Duration::from_millis(2_000)
        ));
    }

    /// Handing the child off must not leak zombies: once the helper exits, the
    /// background reaper collects it. Guards against trading a UI lock for a
    /// process leak.
    #[test]
    fn handed_off_clipboard_child_is_reaped_not_zombied() {
        let command = ClipboardCommand {
            program: "sh",
            args: &["-c", "sleep 0.3"],
        };

        assert!(run_clipboard_command_with_timeout(
            &command,
            b"payload",
            std::time::Duration::from_millis(50)
        ));

        // Give the helper time to exit and the reaper time to collect it.
        std::thread::sleep(std::time::Duration::from_millis(1_200));

        let zombies = std::fs::read_dir("/proc")
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| {
                let name = entry.file_name();
                let Some(pid) = name.to_str() else {
                    return false;
                };
                if pid.parse::<u32>().is_err() {
                    return false;
                }
                let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
                    return false;
                };
                let Some(rest) = stat.rfind(')').and_then(|idx| stat.get(idx + 2..)) else {
                    return false;
                };
                // state == Z and the parent is this test process
                rest.starts_with("Z ")
                    && rest
                        .split_whitespace()
                        .nth(1)
                        .and_then(|ppid| ppid.parse::<u32>().ok())
                        == Some(std::process::id())
            })
            .count();

        assert_eq!(zombies, 0, "handed-off clipboard child left a zombie");
    }

    #[test]
    fn read_clipboard_text_with_command_reads_utf8() {
        let command = ClipboardCommand {
            program: "printf",
            args: &["feature/linear-302"],
        };

        assert_eq!(
            read_clipboard_text_with_command(&command).as_deref(),
            Some("feature/linear-302")
        );
    }

    #[test]
    fn read_clipboard_text_with_command_rejects_oversized_output() {
        let command = ClipboardCommand {
            program: "sh",
            args: &["-c", "yes x | head -c 1048578"],
        };

        assert_eq!(read_clipboard_text_with_command(&command), None);
    }

    #[test]
    fn read_clipboard_image_with_spawned_command_reads_under_limit() {
        let mut command = Command::new("sh");
        command.arg("-c").arg("printf image");

        assert_eq!(
            read_clipboard_image_with_spawned_command_max(command, 16),
            Some(b"image".to_vec())
        );
    }

    #[test]
    fn read_clipboard_image_with_spawned_command_rejects_over_limit() {
        let mut command = Command::new("sh");
        command.arg("-c").arg("printf oversized");

        assert_eq!(
            read_clipboard_image_with_spawned_command_max(command, 4),
            None
        );
    }

    #[test]
    fn read_clipboard_image_rejects_xclip_text_served_for_image_target() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("herdr-fake-xclip-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let fake_xclip = temp_dir.join("xclip");
        std::fs::write(&fake_xclip, "#!/bin/sh\nprintf '# Tasks'\n")
            .expect("fake xclip should be written");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = std::fs::metadata(&fake_xclip)
                .expect("fake xclip metadata")
                .permissions();
            permissions.set_mode(0o700);
            std::fs::set_permissions(&fake_xclip, permissions)
                .expect("fake xclip should be executable");
        }

        let old_path = std::env::var_os("PATH");
        let test_path = match old_path.as_ref() {
            Some(path) => {
                let mut paths = vec![temp_dir.clone()];
                paths.extend(std::env::split_paths(path));
                std::env::join_paths(paths).expect("test path should be valid")
            }
            None => temp_dir.clone().into_os_string(),
        };

        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
            std::env::set_var("DISPLAY", ":0");
            std::env::set_var("PATH", test_path);
        }

        let result = read_clipboard_image();

        unsafe {
            match old_path {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
        }
        let _ = std::fs::remove_file(fake_xclip);
        let _ = std::fs::remove_dir(temp_dir);

        assert_eq!(result, None);
    }

    #[test]
    fn read_clipboard_image_rejects_wayland_xclip_fallback_text_for_image_target() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir =
            std::env::temp_dir().join(format!("herdr-fake-wayland-xclip-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        let fake_wl_paste = temp_dir.join("wl-paste");
        let fake_xclip = temp_dir.join("xclip");
        std::fs::write(&fake_wl_paste, "#!/bin/sh\nexit 1\n")
            .expect("fake wl-paste should be written");
        std::fs::write(&fake_xclip, "#!/bin/sh\nprintf '# Tasks'\n")
            .expect("fake xclip should be written");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            for command in [&fake_wl_paste, &fake_xclip] {
                let mut permissions = std::fs::metadata(command)
                    .expect("fake clipboard command metadata")
                    .permissions();
                permissions.set_mode(0o700);
                std::fs::set_permissions(command, permissions)
                    .expect("fake clipboard command should be executable");
            }
        }

        let old_path = std::env::var_os("PATH");
        let test_path = match old_path.as_ref() {
            Some(path) => {
                let mut paths = vec![temp_dir.clone()];
                paths.extend(std::env::split_paths(path));
                std::env::join_paths(paths).expect("test path should be valid")
            }
            None => temp_dir.clone().into_os_string(),
        };

        unsafe {
            std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
            std::env::set_var("DISPLAY", ":0");
            std::env::set_var("PATH", test_path);
        }

        let result = read_clipboard_image();

        unsafe {
            match old_path {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
        }
        let _ = std::fs::remove_file(fake_wl_paste);
        let _ = std::fs::remove_file(fake_xclip);
        let _ = std::fs::remove_dir(temp_dir);

        assert_eq!(result, None);
    }

    #[test]
    fn read_validated_clipboard_image_accepts_real_png_payload() {
        assert_eq!(
            read_validated_clipboard_image(
                "sh",
                &["-c", "printf '\\211PNG\\r\\n\\032\\nrest-of-image'"],
                "png"
            ),
            Some(ClipboardImage {
                bytes: b"\x89PNG\r\n\x1a\nrest-of-image".to_vec(),
                extension: "png",
            })
        );
    }

    #[test]
    fn image_signatures_match_only_their_format() {
        assert!(bytes_match_image_signature("png", b"\x89PNG\r\n\x1a\n..."));
        assert!(bytes_match_image_signature(
            "jpg",
            &[0xFF, 0xD8, 0xFF, 0xE0]
        ));
        assert!(bytes_match_image_signature("gif", b"GIF87a..."));
        assert!(bytes_match_image_signature("gif", b"GIF89a..."));
        assert!(bytes_match_image_signature(
            "webp",
            b"RIFF\x10\x00\x00\x00WEBPVP8 "
        ));

        let mut bmp = vec![0u8; 26];
        bmp[..2].copy_from_slice(b"BM");
        bmp[10] = 26;
        assert!(bytes_match_image_signature("bmp", &bmp));

        assert!(!bytes_match_image_signature("png", b"# Tasks"));
        assert!(!bytes_match_image_signature("jpg", b"plain clipboard text"));
        assert!(!bytes_match_image_signature("gif", b""));
        assert!(!bytes_match_image_signature("webp", b"RIFF but not webp"));
        assert!(!bytes_match_image_signature("bmp", b"\x89PNG\r\n\x1a\n"));
        assert!(!bytes_match_image_signature(
            "bmp",
            b"BM text is not a bitmap"
        ));
        assert!(!bytes_match_image_signature("svg", b"<svg></svg>"));
    }

    #[test]
    fn desktop_notification_separates_option_like_titles() {
        let _guard = env_lock().lock().unwrap();
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
            std::env::set_var("DISPLAY", ":0");
        }

        let path =
            std::env::temp_dir().join(format!("herdr-notify-send-args-{}", std::process::id()));
        let script = "printf '%s\\n' \"$@\" > \"$HERDR_NOTIFY_ARGS\"";
        let shown = show_desktop_notification_with_command("-danger", Some("body"), |_| {
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(script)
                .arg("notify-send")
                .env("HERDR_NOTIFY_ARGS", &path);
            cmd
        })
        .expect("notification command should run");

        assert!(shown);
        let args = std::fs::read_to_string(&path).expect("args file");
        let _ = std::fs::remove_file(&path);
        assert_eq!(args, "--\n-danger\nbody\n");
    }

    #[test]
    fn scrollback_editor_argv_preserves_unix_editor_shell_semantics() {
        let path = std::path::Path::new("/tmp/herdr scrollback.txt");
        let argv = scrollback_editor_argv(path).unwrap();

        assert_eq!(argv[0], "/bin/sh");
        assert_eq!(argv[1], "-c");
        assert!(argv[2].contains("EDITOR:-vi"));
        assert!(argv[2].contains("/tmp/herdr scrollback.txt"));
    }
}
