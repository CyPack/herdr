#![allow(dead_code)]

use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Once, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

static PID_REGISTRY: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
static RUNTIME_DIR_REGISTRY: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
static INIT: Once = Once::new();
static CLEANUP_GUARD: OnceLock<CleanupGuard> = OnceLock::new();
const WATCHDOG_SCAN_INTERVAL: Duration = Duration::from_secs(1);
const RUNTIME_OWNER_MARKER: &str = ".herdr-test-owner-pid";
// Mirrors `src/protocol/wire.rs::PROTOCOL_VERSION` by hand, because the bin
// crate exposes no lib for tests to import. The wire's own comment explains
// why 19 was skipped. If this drifts, the protocol fixture tests fail fast —
// which is exactly how the drift gets noticed.
pub const CURRENT_PROTOCOL: u32 = 20;

pub fn register_spawned_herdr_pid(pid: Option<u32>) {
    let Some(pid) = pid else {
        return;
    };

    ensure_cleanup_hooks();
    let mut registry = pid_registry_lock();
    registry.insert(pid);
}

pub fn unregister_spawned_herdr_pid(pid: Option<u32>) {
    let Some(pid) = pid else {
        return;
    };

    if let Some(registry) = PID_REGISTRY.get() {
        let mut guard = registry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.remove(&pid);
    }
}

pub fn register_runtime_dir(path: &Path) {
    ensure_cleanup_hooks();

    let _ = fs::create_dir_all(path);
    let _ = fs::write(
        path.join(RUNTIME_OWNER_MARKER),
        std::process::id().to_string(),
    );

    let mut runtime_dirs = runtime_dir_registry_lock();
    runtime_dirs.insert(path.to_path_buf());
}

pub fn unregister_runtime_dir(path: &Path) {
    if let Some(registry) = RUNTIME_DIR_REGISTRY.get() {
        let mut guard = registry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.remove(path);
    }
}

#[cfg(target_os = "linux")]
pub fn herdr_server_pids_for_runtime_dir(runtime_dir: &Path) -> std::io::Result<Vec<u32>> {
    let mut pids = Vec::new();
    for pid in iter_worktree_server_pids()? {
        let Some(process_runtime_dir) = process_runtime_dir(pid)? else {
            continue;
        };
        if process_runtime_dir == runtime_dir {
            pids.push(pid);
        }
    }
    pids.sort_unstable();
    Ok(pids)
}

pub fn cleanup_test_base(base: &Path) {
    let runtime_dir = base.join("runtime");
    let runtime_dirs = HashSet::from([runtime_dir.clone()]);

    let all_stopped = terminate_servers_for_runtime_dirs(&runtime_dirs);
    unregister_runtime_dir(&runtime_dir);

    // Removing the directory takes the socket with it, and a server whose socket
    // is gone can no longer be addressed by anything: `herdr server stop` answers
    // "cannot be reached at <path>: No such file or directory" and the process
    // lives on until somebody finds it by pid. That is how a slow exit became a
    // permanent orphan — measured 2026-08-15, when a machine under load was
    // carrying 51 such directories and 21 such processes.
    //
    // So the sequence is now conditional: a base whose server is still running
    // stays on disk, keeping the leak addressable, and says so. A directory left
    // behind is cheap; a process nothing can name is not.
    if !all_stopped {
        eprintln!(
            "warning: leaving {} in place — a test server there has not exited yet, \
             and deleting its socket would leave a process nothing can address",
            base.display()
        );
        return;
    }

    let _ = fs::remove_dir_all(base);
}

/// How much longer than it says every wait in this file is actually given.
///
/// Same factor and same reason as [`REAP_SLACK`] and `tests/api_ping.rs`: a
/// stated timeout here is a budget for something a herdr server has to finish —
/// a cold start, a marker file, a message on a socket — and on a loaded machine
/// none of those is bounded by the number someone typed. The waits stay failure
/// detectors; they stop being performance assertions.
///
/// One constant rather than one per helper: the number is the same because the
/// reason is the same, and four names for it would be four things to keep in
/// agreement. Measured 2026-08-15: `wait_for_socket` was given slack while
/// `wait_for_file` was not, and a landing gate spent an afternoon failing on the
/// half that was missed.
const WAIT_SLACK: u32 = 12;

pub fn wait_for_socket(path: &Path, timeout: Duration) {
    let budget = timeout * WAIT_SLACK;
    let started = Instant::now();
    let deadline = started + budget;
    while Instant::now() < deadline {
        if path.exists() && UnixStream::connect(path).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!(
        "socket did not appear at {} after {:?} of a {budget:?} budget",
        path.display(),
        started.elapsed()
    );
}

pub fn wait_for_file(path: &Path, timeout: Duration) {
    let budget = timeout * WAIT_SLACK;
    let started = Instant::now();
    let deadline = started + budget;
    while Instant::now() < deadline {
        if path.exists() {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!(
        "file did not appear at {} after {:?} of a {budget:?} budget",
        path.display(),
        started.elapsed()
    );
}

pub fn encode_varint_u32(v: u32) -> Vec<u8> {
    if v < 251 {
        vec![v as u8]
    } else if v < 65536 {
        let mut buf = vec![251u8];
        buf.extend_from_slice(&(v as u16).to_le_bytes());
        buf
    } else {
        let mut buf = vec![252u8];
        buf.extend_from_slice(&v.to_le_bytes());
        buf
    }
}

pub fn encode_varint_u16(v: u16) -> Vec<u8> {
    if v < 251 {
        vec![v as u8]
    } else {
        let mut buf = vec![251u8];
        buf.extend_from_slice(&v.to_le_bytes());
        buf
    }
}

pub fn frame_message(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u32;
    let mut framed = len.to_le_bytes().to_vec();
    framed.extend_from_slice(payload);
    framed
}

pub fn decode_varint_u32(payload: &[u8], offset: usize) -> Result<(u32, usize), String> {
    if offset >= payload.len() {
        return Err("payload too short for varint".into());
    }
    let first_byte = payload[offset];
    match first_byte {
        0..=250 => Ok((first_byte as u32, 1)),
        251 => {
            if offset + 3 > payload.len() {
                return Err("payload too short for u16 varint".into());
            }
            let v = u16::from_le_bytes(
                payload[offset + 1..offset + 3]
                    .try_into()
                    .map_err(|e: std::array::TryFromSliceError| e.to_string())?,
            );
            Ok((v as u32, 3))
        }
        252 => {
            if offset + 5 > payload.len() {
                return Err("payload too short for u32 varint".into());
            }
            let v = u32::from_le_bytes(
                payload[offset + 1..offset + 5]
                    .try_into()
                    .map_err(|e: std::array::TryFromSliceError| e.to_string())?,
            );
            Ok((v, 5))
        }
        _ => Err(format!("unsupported varint tag: {first_byte}")),
    }
}

fn encode_varint_enum(variant_idx: u32, fields: &[&[u8]]) -> Vec<u8> {
    let mut buf = encode_varint_u32(variant_idx);
    for field in fields {
        buf.extend_from_slice(field);
    }
    buf
}

fn decode_welcome(payload: &[u8]) -> Result<(u32, Option<String>), String> {
    let mut offset = 0;
    let (variant, consumed) = decode_varint_u32(payload, offset)?;
    offset += consumed;
    if variant != 0 {
        return Err(format!(
            "expected Welcome (variant 0), got variant {variant}"
        ));
    }

    let (version, consumed) = decode_varint_u32(payload, offset)?;
    offset += consumed;

    let (_encoding, consumed) = decode_varint_u32(payload, offset)?;
    offset += consumed;

    if offset >= payload.len() {
        return Err("payload too short for Option tag".into());
    }
    let option_tag = payload[offset];
    offset += 1;

    let error = if option_tag == 1 {
        let (str_len, consumed) = decode_varint_u32(payload, offset)?;
        offset += consumed;
        let str_len = str_len as usize;
        if offset + str_len > payload.len() {
            return Err("payload too short for string content".into());
        }
        Some(
            String::from_utf8(payload[offset..offset + str_len].to_vec())
                .map_err(|e| e.to_string())?,
        )
    } else {
        None
    };

    Ok((version, error))
}

pub fn client_handshake(
    stream: &mut UnixStream,
    version: u32,
    cols: u16,
    rows: u16,
) -> Result<(u32, Option<String>), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let hello_payload = encode_varint_enum(
        0,
        &[
            &encode_varint_u32(version),
            &encode_varint_u16(cols),
            &encode_varint_u16(rows),
            &encode_varint_u32(8),  // cell_width_px
            &encode_varint_u32(16), // cell_height_px
            &encode_varint_u32(0),  // RenderEncoding::SemanticFrame
            &encode_varint_u32(0),  // ClientKeybindings::Server
            &encode_varint_u32(0),  // ClientLaunchMode::App
        ],
    );
    let framed = frame_message(&hello_payload);
    stream.write_all(&framed).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > 2 * 1024 * 1024 {
        return Err(format!("oversized response: {len}"));
    }

    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    decode_welcome(&payload)
}

pub fn read_server_message(stream: &mut UnixStream) -> Result<(u32, Vec<u8>), String> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .map_err(|e| format!("read length prefix: {e}"))?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > 2 * 1024 * 1024 {
        return Err(format!("oversized frame: {len} bytes"));
    }
    if len == 0 {
        return Err("zero-length frame".into());
    }

    let mut payload = vec![0u8; len];
    stream
        .read_exact(&mut payload)
        .map_err(|e| format!("read payload: {e}"))?;

    let (variant, consumed) = decode_varint_u32(&payload, 0)?;
    Ok((variant, payload[consumed..].to_vec()))
}

pub fn send_input(stream: &mut UnixStream, data: &[u8]) -> Result<(), String> {
    let mut buf = encode_varint_u32(1);
    buf.extend_from_slice(&encode_varint_u32(data.len() as u32));
    buf.extend_from_slice(data);
    let framed = frame_message(&buf);
    stream
        .write_all(&framed)
        .map_err(|e| format!("write input: {e}"))?;
    stream.flush().map_err(|e| format!("flush input: {e}"))?;
    Ok(())
}

pub fn send_detach(stream: &mut UnixStream) -> Result<(), String> {
    let detach_payload = encode_varint_u32(4);
    let framed = frame_message(&detach_payload);
    stream
        .write_all(&framed)
        .map_err(|e| format!("write detach: {e}"))?;
    stream.flush().map_err(|e| format!("flush detach: {e}"))?;
    Ok(())
}

pub fn drain_messages(stream: &mut UnixStream) {
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    while read_server_message(stream).is_ok() {}
    stream.set_read_timeout(None).unwrap();
}

pub fn wait_until<F>(timeout: Duration, interval: Duration, mut predicate: F) -> bool
where
    F: FnMut() -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        thread::sleep(interval);
    }
    predicate()
}

pub fn wait_for_message_variant(
    stream: &mut UnixStream,
    timeout: Duration,
    variant: u32,
) -> Result<bool, String> {
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + timeout * WAIT_SLACK;
    while Instant::now() < deadline {
        match read_server_message(stream) {
            Ok((got, _)) if got == variant => return Ok(true),
            Ok(_) => continue,
            Err(_) => continue,
        }
    }
    Ok(false)
}

pub fn wait_for_disconnect(stream: &mut UnixStream, timeout: Duration) -> Result<bool, String> {
    stream.set_nonblocking(true).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + timeout * WAIT_SLACK;
    let mut idle_since = None;
    let result = loop {
        match read_server_message(stream) {
            Ok(_) => idle_since = None,
            Err(err)
                if err.to_ascii_lowercase().contains("would block")
                    || err.contains("Resource temporarily unavailable") =>
            {
                let idle_started = *idle_since.get_or_insert_with(Instant::now);
                if idle_started.elapsed() >= Duration::from_millis(200) {
                    break Ok(true);
                }
            }
            Err(_) => break Ok(true),
        }
        if Instant::now() >= deadline {
            break Ok(false);
        }
        thread::sleep(Duration::from_millis(25));
    };
    let _ = stream.set_nonblocking(false);
    result
}

pub fn cleanup_registered_herdr_pids() {
    let pids: Vec<u32> = {
        let mut registry = pid_registry_lock();
        registry.drain().collect()
    };

    // Counted rather than ignored. This is the last chance the suite gets, so a
    // survivor here is exactly the process that shows up hours later on a
    // loaded machine with nobody able to say where it came from.
    let survivors = pids
        .into_iter()
        .filter(|pid| !terminate_pid(*pid))
        .collect::<Vec<_>>();
    if !survivors.is_empty() {
        eprintln!("warning: test servers still running after teardown: {survivors:?}");
    }

    let runtime_dirs: HashSet<PathBuf> = {
        let mut runtime_dirs = runtime_dir_registry_lock();
        runtime_dirs.drain().collect()
    };

    if !terminate_servers_for_runtime_dirs(&runtime_dirs) {
        eprintln!(
            "warning: a test server under {} runtime dir(s) outlived teardown",
            runtime_dirs.len()
        );
    }
    let _ = cleanup_servers_with_missing_runtime_dir();
}

fn ensure_cleanup_hooks() {
    INIT.call_once(|| {
        let _ = cleanup_servers_with_missing_runtime_dir();
        start_global_watchdog();

        let _ = CLEANUP_GUARD.set(CleanupGuard);

        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            cleanup_registered_herdr_pids();
            previous_hook(panic_info);
        }));

        let _ = ctrlc::set_handler(|| {
            cleanup_registered_herdr_pids();
            std::process::exit(130);
        });

        unsafe {
            libc::atexit(run_atexit_cleanup);
        }
    });
}

fn pid_registry_lock() -> std::sync::MutexGuard<'static, HashSet<u32>> {
    PID_REGISTRY
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn runtime_dir_registry_lock() -> std::sync::MutexGuard<'static, HashSet<PathBuf>> {
    RUNTIME_DIR_REGISTRY
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn registered_runtime_dirs_snapshot() -> HashSet<PathBuf> {
    if let Some(runtime_dirs) = RUNTIME_DIR_REGISTRY.get() {
        runtime_dirs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    } else {
        HashSet::new()
    }
}

fn should_terminate_runtime_dir(
    runtime_dir: &Path,
    registered_runtime_dirs: &HashSet<PathBuf>,
) -> bool {
    if !registered_runtime_dirs.contains(runtime_dir) {
        return false;
    }

    if !runtime_dir.exists() {
        return true;
    }

    !runtime_dir_owner_alive(runtime_dir)
}

fn start_global_watchdog() {
    thread::spawn(|| loop {
        thread::sleep(WATCHDOG_SCAN_INTERVAL);

        if let Err(err) = cleanup_servers_with_missing_runtime_dir() {
            eprintln!("herdr test cleanup watchdog error: {err}");
        }
    });
}

fn cleanup_servers_with_missing_runtime_dir() -> std::io::Result<()> {
    let registered_runtime_dirs = registered_runtime_dirs_snapshot();
    if registered_runtime_dirs.is_empty() {
        return Ok(());
    }

    for pid in iter_worktree_server_pids()? {
        let Some(runtime_dir) = process_runtime_dir(pid)? else {
            continue;
        };

        if should_terminate_runtime_dir(&runtime_dir, &registered_runtime_dirs)
            && !terminate_pid(pid)
        {
            // These are the orphans by definition: their runtime dir is already
            // gone, so `herdr server stop` can no longer reach them and only a
            // pid can. One that survives even this is worth naming out loud.
            eprintln!("warning: orphaned test server {pid} did not exit");
        }
    }

    Ok(())
}

/// Stop every test server belonging to these runtime dirs, reporting whether
/// they all actually went.
///
/// The return value is the whole point. The caller is about to delete the
/// directory these sockets live in, and it may only do that once nothing is
/// still listening on them.
#[must_use]
fn terminate_servers_for_runtime_dirs(runtime_dirs: &HashSet<PathBuf>) -> bool {
    if runtime_dirs.is_empty() {
        return true;
    }

    let Ok(pids) = iter_worktree_server_pids() else {
        // The pid scan failing is not the same as nothing running, and treating
        // it as "all clear" is what would license deleting a live server's
        // socket on the strength of a reading that never happened.
        return false;
    };

    let mut all_stopped = true;
    for pid in pids {
        let Ok(runtime_dir) = process_runtime_dir(pid) else {
            continue;
        };

        let Some(runtime_dir) = runtime_dir else {
            continue;
        };

        if runtime_dirs.contains(&runtime_dir) {
            all_stopped &= terminate_pid(pid);
        }
    }
    all_stopped
}

fn iter_worktree_server_pids() -> std::io::Result<Vec<u32>> {
    let own_pid = std::process::id();
    let mut pids = Vec::new();

    let proc_entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };

    for entry in proc_entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(pid) = file_name.to_str().and_then(|name| name.parse::<u32>().ok()) else {
            continue;
        };

        if pid == own_pid {
            continue;
        }

        if is_test_herdr_server_process(pid) {
            pids.push(pid);
        }
    }

    Ok(pids)
}

fn is_test_herdr_server_process(pid: u32) -> bool {
    let Some(exe_path) = proc_link_target(pid, "exe") else {
        return false;
    };

    // Path shape alone goes blind the moment CARGO_TARGET_DIR moves the
    // build out of the checkout: the exe stops matching, every reaper
    // predicate turns false, and the servers this suite spawned keep running
    // with nobody responsible for them — measured live as 24 handoff
    // grandchildren surviving their test run by hours. Cargo stamps
    // CARGO_MANIFEST_DIR into the test process's environment, children
    // inherit it, and handoff grandchildren inherit it again, so the
    // inherited stamp identifies exactly the processes this checkout's suite
    // is answerable for, wherever the build artifacts live.
    if !is_test_herdr_binary(&exe_path)
        && !(is_herdr_executable_name(&exe_path) && process_spawned_by_this_suite(pid))
    {
        return false;
    }

    let Ok(cmdline) = read_cmdline(pid) else {
        return false;
    };

    cmdline.iter().any(|arg| arg == "server")
}

fn proc_link_target(pid: u32, link: &str) -> Option<PathBuf> {
    fs::read_link(format!("/proc/{pid}/{link}")).ok()
}

fn read_cmdline(pid: u32) -> std::io::Result<Vec<String>> {
    let cmdline = fs::read(format!("/proc/{pid}/cmdline"))?;
    Ok(cmdline
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect())
}

fn process_runtime_dir(pid: u32) -> std::io::Result<Option<PathBuf>> {
    let environ = fs::read(format!("/proc/{pid}/environ"))?;

    let mut socket_path: Option<PathBuf> = None;

    for entry in environ.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }

        let kv = String::from_utf8_lossy(entry);
        if let Some(value) = kv.strip_prefix("XDG_RUNTIME_DIR=") {
            return Ok(Some(PathBuf::from(value)));
        }

        if let Some(value) = kv.strip_prefix("HERDR_SOCKET_PATH=") {
            socket_path = Some(PathBuf::from(value));
        }
    }

    Ok(socket_path.and_then(|path| path.parent().map(Path::to_path_buf)))
}

fn runtime_dir_owner_alive(runtime_dir: &Path) -> bool {
    let marker = runtime_dir.join(RUNTIME_OWNER_MARKER);
    let Ok(contents) = fs::read_to_string(marker) else {
        return false;
    };

    let Ok(owner_pid) = contents.trim().parse::<libc::pid_t>() else {
        return false;
    };

    process_exists(owner_pid)
}

fn current_checkout_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn is_test_herdr_binary(path: &Path) -> bool {
    path.ends_with("target/debug/herdr") && path.starts_with(current_checkout_root())
}

/// Whether this executable name is a herdr binary, including one whose file
/// was rebuilt or deleted while the process kept running.
fn is_herdr_executable_name(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.strip_suffix(" (deleted)").unwrap_or(name) == "herdr"
}

/// Whether this process inherited its environment from a run of *this*
/// checkout's suite. See `is_test_herdr_server_process` for why identity
/// cannot rest on the executable path alone.
fn process_spawned_by_this_suite(pid: u32) -> bool {
    let Ok(environ) = fs::read(format!("/proc/{pid}/environ")) else {
        return false;
    };
    environ_declares_manifest_dir(&environ, current_checkout_root())
}

fn environ_declares_manifest_dir(environ: &[u8], root: &Path) -> bool {
    environ.split(|byte| *byte == 0).any(|entry| {
        String::from_utf8_lossy(entry)
            .strip_prefix("CARGO_MANIFEST_DIR=")
            .is_some_and(|value| Path::new(value) == root)
    })
}

extern "C" fn run_atexit_cleanup() {
    cleanup_registered_herdr_pids();
}

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        cleanup_registered_herdr_pids();
    }
}

/// How much longer than they say the reap waits below are actually given.
///
/// Same reasoning, and the same measured machine, as the slack in
/// `tests/api_ping.rs`: these are failure detectors, not speed assertions. A
/// server that needs four seconds to die on a loaded box has done nothing
/// wrong — but the old 2-second ceiling returned anyway, and the caller then
/// deleted the socket out from under it. Being slow was quietly converted into
/// being unkillable.
const REAP_SLACK: u32 = 12;

/// Ask one test server to exit, and say whether it did.
///
/// The boolean is load-bearing: the caller deletes the directory holding this
/// process's socket, and a `false` here is the only thing standing between a
/// slow exit and an orphan no command can name afterwards.
#[must_use]
fn terminate_pid(pid: u32) -> bool {
    let pid_t = pid as libc::pid_t;

    if process_exists(pid_t) {
        unsafe {
            libc::kill(pid_t, libc::SIGTERM);
        }
    }

    if wait_for_pid_exit(pid_t, Duration::from_millis(400) * REAP_SLACK) {
        return true;
    }

    if process_exists(pid_t) {
        unsafe {
            libc::kill(pid_t, libc::SIGKILL);
        }
    }

    // Returned rather than discarded. `let _ =` here was the whole defect: the
    // code already knew the process might still be alive, and threw that away
    // one line before the caller deleted its socket.
    wait_for_pid_exit(pid_t, Duration::from_secs(2) * REAP_SLACK)
}

fn wait_for_pid_exit(pid: libc::pid_t, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        if !process_exists(pid) {
            return true;
        }

        let mut status = 0;
        let result = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if result == pid {
            return true;
        }

        if result == -1 {
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ECHILD) => {
                    // Not our child (or already reaped elsewhere). Poll /proc existence
                    // until the process is truly gone.
                    if !process_exists(pid) {
                        return true;
                    }
                }
                Some(libc::ESRCH) => return true,
                _ => {
                    if !process_exists(pid) {
                        return true;
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(20));
    }

    !process_exists(pid)
}

fn process_exists(pid: libc::pid_t) -> bool {
    let result = unsafe { libc::kill(pid, 0) };
    if result == 0 {
        true
    } else {
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_missing_runtime_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "herdr-watchdog-scoping-{label}-{}-{unique}",
            std::process::id()
        ))
    }

    #[test]
    fn watchdog_scoping_does_not_terminate_missing_unregistered_runtime_dir() {
        let runtime_dir = unique_missing_runtime_dir("unregistered");
        let registered_runtime_dirs = HashSet::new();

        assert!(
            !should_terminate_runtime_dir(&runtime_dir, &registered_runtime_dirs),
            "missing runtime dirs must not be killable until they are proven session-owned"
        );
    }

    #[test]
    fn watchdog_scoping_terminates_missing_registered_runtime_dir() {
        let runtime_dir = unique_missing_runtime_dir("registered");
        let mut registered_runtime_dirs = HashSet::new();
        registered_runtime_dirs.insert(runtime_dir.clone());

        assert!(
            should_terminate_runtime_dir(&runtime_dir, &registered_runtime_dirs),
            "missing runtime dirs that are session-owned should be considered killable"
        );
    }

    #[test]
    fn test_binary_matcher_accepts_current_checkout_debug_binary() {
        let binary = current_checkout_root().join("target/debug/herdr");
        assert!(
            is_test_herdr_binary(&binary),
            "current checkout debug binary should be considered test-owned"
        );
    }

    #[test]
    fn test_binary_matcher_rejects_installed_binary() {
        assert!(
            !is_test_herdr_binary(Path::new("/home/can/.local/bin/herdr")),
            "installed binaries must not be considered test-owned"
        );
    }

    #[test]
    fn herdr_executable_names_match_wherever_the_target_dir_lives() {
        // The exact path shape CARGO_TARGET_DIR produces — the reason
        // identity cannot rest on the path alone.
        assert!(is_herdr_executable_name(Path::new(
            "/var/tmp/wt-orphan-reap-target/debug/herdr"
        )));
        assert!(is_herdr_executable_name(Path::new(
            "/tmp/x/target/debug/herdr (deleted)"
        )));
        assert!(!is_herdr_executable_name(Path::new(
            "/tmp/x/target/debug/herdr-web"
        )));
        assert!(!is_herdr_executable_name(Path::new("/usr/bin/cargo")));
    }

    #[test]
    fn suite_identity_comes_from_the_inherited_manifest_dir() {
        let root = current_checkout_root();
        let matching = format!("A=1\0CARGO_MANIFEST_DIR={}\0B=2\0", root.display());
        assert!(environ_declares_manifest_dir(matching.as_bytes(), root));
        assert!(!environ_declares_manifest_dir(
            b"CARGO_MANIFEST_DIR=/somewhere/else\0",
            root
        ));
        assert!(!environ_declares_manifest_dir(b"A=1\0B=2\0", root));
        // The running test process carries the stamp itself, so the reaper
        // recognizes this suite's own spawns however the target dir is set.
        // If this fails, the suite is running outside cargo entirely and the
        // reaper is blind by construction.
        assert!(process_spawned_by_this_suite(std::process::id()));
    }
}
