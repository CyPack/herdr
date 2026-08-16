#[cfg(unix)]
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::fd::{AsRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::{Child, Command};
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use tracing::{info, warn};

#[cfg(unix)]
const HANDOFF_VERSION: u32 = 1;
#[cfg(unix)]
const READY_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(unix)]
const OWNED_ACK_TIMEOUT: Duration = Duration::from_millis(500);
// One SCM_RIGHTS message carries at most 253 fds (kernel SCM_MAX_FD, measured
// on Linux 7.1). 128 clears a measured 59-pane session with 2x headroom while
// staying under half the kernel ceiling, where low net.core.optmem_max settings
// can start refusing the control buffer. TP-HANDOFF-FD-01
#[cfg(unix)]
pub(crate) const MAX_FDS_PER_HANDOFF: usize = 128;
#[cfg(unix)]
pub(crate) const MAX_REPLAY_BYTES_PER_PANE: usize = 8 * 1024;
#[cfg(unix)]
pub(crate) const COMMIT_TIMEOUT: Duration = READY_TIMEOUT;

/// Give a timed-out protocol wait a name before it leaves this module.
///
/// Every read in the handoff conversation carries `SO_RCVTIMEO`, so a wait that runs
/// out arrives as a bare `EAGAIN` and travels up as "Resource temporarily unavailable
/// (os error 11)". That string reaches the person who ran `herdr update`, and it names
/// neither the step that stalled nor the budget it stalled against — on a machine busy
/// enough to miss the budget, it is the only record of what happened.
///
/// Only a timeout is renamed. Anything else already describes itself.
#[cfg(unix)]
fn name_handoff_wait(step: &'static str, budget: Duration, err: io::Error) -> io::Error {
    if !matches!(
        err.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    ) {
        return err;
    }
    io::Error::new(
        err.kind(),
        format!("handoff timed out waiting for {step} after {budget:?} ({err})"),
    )
}

#[cfg(unix)]
#[derive(Serialize, Deserialize)]
pub(crate) struct HandoffManifest {
    pub version: u32,
    pub source_version: String,
    pub source_protocol: u32,
    pub expected_version: Option<String>,
    pub expected_protocol: Option<u32>,
    pub snapshot: crate::persist::SessionSnapshot,
    pub panes: Vec<crate::handoff_runtime::HandoffRuntimeState>,
}

#[cfg(unix)]
pub(crate) struct ReceivedHandoff {
    pub manifest: HandoffManifest,
    pub fds: Vec<RawFd>,
    pub stream: UnixStream,
}

#[cfg(unix)]
pub(crate) fn handoff_socket_path() -> PathBuf {
    crate::session::data_dir().join(format!("herdr-handoff-{}.sock", std::process::id()))
}

#[cfg(unix)]
const FREIGHT_VERSION: u32 = 1;

/// Full pane histories, carried on disk beside the handoff socket.
///
/// The manifest line is read one byte per syscall (measured at 0.77 MiB/s —
/// the fd passing shares the stream, so it cannot be buffered), and its inline
/// replay is cut to [`MAX_REPLAY_BYTES_PER_PANE`] to keep that read short. A
/// session's real scrollback does not fit through that pipe, so it travels in
/// this file instead: the exporter writes it, the importer consumes it, and an
/// importer that predates it still gets the inline replay unchanged.
#[cfg(unix)]
#[derive(Serialize, Deserialize)]
pub(crate) struct HandoffHistoryFreight {
    pub version: u32,
    /// Keyed by the same `pane_id` the manifest's runtime states carry.
    pub panes: std::collections::HashMap<u32, String>,
}

/// Where the freight for a given handoff socket lives.
///
/// Derived from the socket path so the importer needs no new argument and the
/// manifest needs no new field — both sides already hold the socket path.
#[cfg(unix)]
pub(crate) fn handoff_history_freight_path(socket_path: &Path) -> PathBuf {
    socket_path.with_extension("history.json")
}

/// Write the freight beside the socket. Failure is not a handoff failure —
/// the inline replay still flows — so the caller only gets a warning signal.
#[cfg(unix)]
pub(crate) fn write_history_freight(
    socket_path: &Path,
    panes: std::collections::HashMap<u32, String>,
) -> io::Result<()> {
    let path = handoff_history_freight_path(socket_path);
    let tmp = path.with_extension("history.json.tmp");
    let file = std::fs::File::create(&tmp)?;
    let mut writer = std::io::BufWriter::new(file);
    let freight = HandoffHistoryFreight {
        version: FREIGHT_VERSION,
        panes,
    };
    // to_writer streams the escaping: peak memory stays at the captured
    // histories themselves, which the exporting server already held.
    serde_json::to_writer(&mut writer, &freight).map_err(io::Error::other)?;
    writer.flush()?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Read and delete the freight. One shot: whether the parse succeeds or not,
/// the file is gone afterwards — freight is transfer luggage, not persistence.
#[cfg(unix)]
pub(crate) fn take_history_freight(
    socket_path: &Path,
) -> Option<std::collections::HashMap<u32, String>> {
    let path = handoff_history_freight_path(socket_path);
    let content = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    match serde_json::from_str::<HandoffHistoryFreight>(&content) {
        Ok(freight) if freight.version <= FREIGHT_VERSION => Some(freight.panes),
        Ok(freight) => {
            warn!(
                version = freight.version,
                "handoff history freight is newer than this server; using inline replay"
            );
            None
        }
        Err(err) => {
            warn!(err = %err, "handoff history freight did not parse; using inline replay");
            None
        }
    }
}

/// Remove the freight for a handoff that will not be imported.
#[cfg(unix)]
pub(crate) fn discard_history_freight(socket_path: &Path) {
    let _ = std::fs::remove_file(handoff_history_freight_path(socket_path));
}

/// Sweep freight files whose exporter is gone.
///
/// Normal lifecycles delete the freight (importer consumes it; a failed
/// handoff discards it). Only a crash of both sides leaves one behind, and
/// its filename carries the dead exporter's pid — the next export cleans up.
#[cfg(unix)]
pub(crate) fn sweep_dead_history_freight(data_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(data_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(pid) = name
            .strip_prefix("herdr-handoff-")
            .and_then(|rest| rest.strip_suffix(".history.json"))
            .and_then(|pid| pid.parse::<u32>().ok())
        else {
            continue;
        };
        if !crate::platform::process_exists(pid) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(unix)]
pub(crate) fn spawn_handoff_import(
    import_exe: Option<&Path>,
    socket_path: &Path,
    token: &str,
) -> io::Result<Child> {
    let fallback_exe;
    let exe = if let Some(import_exe) = import_exe {
        import_exe
    } else {
        fallback_exe = std::env::current_exe().map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("failed to determine herdr executable path: {err}"),
            )
        })?;
        &fallback_exe
    };
    let mut command = Command::new(exe);
    command
        .arg("server")
        .arg("--handoff-import")
        .arg(socket_path)
        .arg(token)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if crate::session::explicit_session_requested() {
        // The import child no longer has the original `--session` argument, so
        // stale socket overrides must not mask the inherited HERDR_SESSION.
        command
            .env_remove(crate::api::SOCKET_PATH_ENV_VAR)
            .env_remove(crate::server::socket_paths::CLIENT_SOCKET_PATH_ENV_VAR);
    }
    crate::platform::detach_server_daemon_command(&mut command);
    command.spawn().map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "failed to spawn handoff import server at {}: {err}",
                exe.display()
            ),
        )
    })
}

#[cfg(unix)]
pub(crate) fn cleanup_failed_import_child(child: &mut Child) {
    let pid = child.id();
    match child.try_wait() {
        Ok(Some(status)) => {
            info!(pid, status = %status, "handoff import server exited during rollback");
            return;
        }
        Ok(None) => {}
        Err(err) => {
            warn!(pid, err = %err, "failed to inspect handoff import server before rollback");
        }
    }

    if let Err(err) = child.kill() {
        warn!(pid, err = %err, "failed to kill handoff import server during rollback");
    }
    match child.wait() {
        Ok(status) => {
            info!(pid, status = %status, "handoff import server reaped during rollback");
        }
        Err(err) => {
            warn!(pid, err = %err, "failed to reap handoff import server during rollback");
        }
    }
}

#[cfg(unix)]
pub(crate) fn bind_listener(socket_path: &Path) -> io::Result<UnixListener> {
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
    listener.set_nonblocking(true)?;
    restrict_socket_permissions(socket_path)?;
    Ok(listener)
}

#[cfg(unix)]
pub(crate) fn accept_and_validate_on(
    listener: UnixListener,
    socket_path: &Path,
    token: &str,
    manifest: &HandoffManifest,
) -> io::Result<UnixStream> {
    let (mut stream, _) = accept_with_timeout(&listener, READY_TIMEOUT).map_err(|err| {
        name_handoff_wait("the replacement server to connect", READY_TIMEOUT, err)
    })?;
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(READY_TIMEOUT))?;
    stream.set_write_timeout(Some(READY_TIMEOUT))?;
    let token_line = read_line_unbuffered(&mut stream)
        .map_err(|err| name_handoff_wait("the replacement server's token", READY_TIMEOUT, err))?;
    if token_line.trim_end() != token {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "handoff import token mismatch",
        ));
    }

    serde_json::to_writer(&mut stream, manifest).map_err(io::Error::other)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    stream.set_read_timeout(Some(READY_TIMEOUT))?;
    let validated = read_line_unbuffered(&mut stream)
        .map_err(|err| name_handoff_wait("the manifest to be validated", READY_TIMEOUT, err))?;
    if validated.trim_end() != "validated" {
        return Err(io::Error::other("handoff import did not validate manifest"));
    }
    let _ = std::fs::remove_file(socket_path);
    Ok(stream)
}

#[cfg(unix)]
pub(crate) fn send_fds_and_wait_restored(stream: &mut UnixStream, fds: &[RawFd]) -> io::Result<()> {
    if fds.len() > MAX_FDS_PER_HANDOFF {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("handoff supports at most {MAX_FDS_PER_HANDOFF} pane file descriptors at once"),
        ));
    }
    send_fds(stream, fds)?;

    stream.set_read_timeout(Some(READY_TIMEOUT))?;
    let restored = read_line_unbuffered(&mut *stream)
        .map_err(|err| name_handoff_wait("the pane runtimes to be restored", READY_TIMEOUT, err))?;
    if restored.trim_end() != "restored" {
        return Err(io::Error::other(
            "handoff import did not report restored runtimes",
        ));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn wait_ready(stream: &mut UnixStream) -> io::Result<()> {
    stream.set_read_timeout(Some(READY_TIMEOUT))?;
    let ready = read_line_unbuffered(&mut *stream).map_err(|err| {
        name_handoff_wait("the replacement server to become ready", READY_TIMEOUT, err)
    })?;
    if ready.trim_end() != "ready" {
        return Err(io::Error::other("handoff import did not report ready"));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn report_committed(stream: &mut UnixStream) -> io::Result<()> {
    stream.write_all(b"committed\n")?;
    stream.flush()
}

#[cfg(unix)]
pub(crate) fn wait_owned_ack(stream: &mut UnixStream) {
    if let Err(err) = stream.set_read_timeout(Some(OWNED_ACK_TIMEOUT)) {
        warn!(err = %err, "failed to set handoff ownership ack timeout");
        return;
    }
    match read_line_unbuffered(&mut *stream) {
        Ok(owned) if owned.trim_end() == "owned" => {}
        Ok(other) => {
            warn!(
                response = %other.trim_end(),
                "handoff import sent unexpected ownership ack after commit"
            );
        }
        Err(err) => {
            warn!(err = %err, "handoff import ownership ack was not received after commit");
        }
    }
}

#[cfg(unix)]
pub(crate) fn receive(socket_path: &Path, token: &str) -> io::Result<ReceivedHandoff> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(token.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let manifest_line = read_line_unbuffered(&mut stream)?;
    let manifest: HandoffManifest =
        serde_json::from_str(&manifest_line).map_err(io::Error::other)?;
    if manifest.version != HANDOFF_VERSION {
        return Err(io::Error::other(format!(
            "unsupported handoff version {}",
            manifest.version
        )));
    }
    if manifest
        .expected_protocol
        .is_some_and(|protocol| protocol != crate::protocol::PROTOCOL_VERSION)
    {
        return Err(io::Error::other(format!(
            "handoff expected protocol {}, but this server speaks protocol {}",
            manifest.expected_protocol.unwrap_or_default(),
            crate::protocol::PROTOCOL_VERSION
        )));
    }
    if manifest
        .expected_version
        .as_deref()
        .is_some_and(|version| version != crate::build_info::version())
    {
        return Err(io::Error::other(format!(
            "handoff expected herdr v{}, but this server is v{}",
            manifest.expected_version.as_deref().unwrap_or("unknown"),
            crate::build_info::version()
        )));
    }
    stream.write_all(b"validated\n")?;
    stream.flush()?;
    let fds = recv_fds(&stream, manifest.panes.len())?;
    Ok(ReceivedHandoff {
        manifest,
        fds,
        stream,
    })
}

#[cfg(unix)]
pub(crate) fn report_restored(stream: &mut UnixStream) -> io::Result<()> {
    stream.write_all(b"restored\n")?;
    stream.flush()
}

#[cfg(unix)]
pub(crate) fn report_ready(stream: &mut UnixStream) -> io::Result<()> {
    stream.write_all(b"ready\n")?;
    stream.flush()
}

#[cfg(unix)]
pub(crate) fn wait_committed(stream: &mut UnixStream) -> io::Result<()> {
    stream.set_read_timeout(Some(READY_TIMEOUT))?;
    let committed = read_line_unbuffered(&mut *stream)
        .map_err(|err| name_handoff_wait("the commit acknowledgement", READY_TIMEOUT, err))?;
    if committed.trim_end() != "committed" {
        return Err(io::Error::other("handoff source did not commit"));
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn report_owned(stream: &mut UnixStream) -> io::Result<()> {
    stream.write_all(b"owned\n")?;
    stream.flush()
}

#[cfg(unix)]
pub(crate) fn manifest_for(
    snapshot: crate::persist::SessionSnapshot,
    panes: Vec<crate::handoff_runtime::HandoffRuntimeState>,
    expected_protocol: Option<u32>,
    expected_version: Option<String>,
) -> HandoffManifest {
    HandoffManifest {
        version: HANDOFF_VERSION,
        source_version: crate::build_info::version(),
        source_protocol: crate::protocol::PROTOCOL_VERSION,
        expected_version,
        expected_protocol,
        snapshot,
        panes,
    }
}

#[cfg(unix)]
fn restrict_socket_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(unix)]
fn accept_with_timeout(
    listener: &UnixListener,
    timeout: Duration,
) -> io::Result<(UnixStream, std::os::unix::net::SocketAddr)> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok(accepted) => return Ok(accepted),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out waiting for handoff import connection",
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
}

#[cfg(unix)]
fn read_line_unbuffered(stream: &mut UnixStream) -> io::Result<String> {
    let mut bytes = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = stream.read(&mut byte)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "handoff stream closed while reading line",
            ));
        }
        bytes.push(byte[0]);
        if byte[0] == b'\n' {
            return String::from_utf8(bytes)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err));
        }
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "handoff line exceeded maximum size",
            ));
        }
    }
}

#[cfg(unix)]
fn send_fds(stream: &UnixStream, fds: &[RawFd]) -> io::Result<()> {
    if fds.is_empty() {
        return Ok(());
    }
    let byte = [b'F'];
    let iov = [libc::iovec {
        iov_base: byte.as_ptr() as *mut libc::c_void,
        iov_len: byte.len(),
    }];
    let fd_bytes = std::mem::size_of_val(fds);
    let mut control = vec![0u8; unsafe { libc::CMSG_SPACE(fd_bytes as u32) as usize }];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = iov.as_ptr() as *mut libc::iovec;
    msg.msg_iovlen = iov.len() as _;
    msg.msg_control = control.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = control.len() as _;

    unsafe {
        let cmsg = libc::CMSG_FIRSTHDR(&msg);
        if cmsg.is_null() {
            return Err(io::Error::other("failed to allocate fd control message"));
        }
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(fd_bytes as u32) as _;
        std::ptr::copy_nonoverlapping(fds.as_ptr() as *const u8, libc::CMSG_DATA(cmsg), fd_bytes);
        if libc::sendmsg(stream.as_raw_fd(), &msg, 0) < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(unix)]
fn recv_fds(stream: &UnixStream, expected: usize) -> io::Result<Vec<RawFd>> {
    if expected == 0 {
        return Ok(Vec::new());
    }
    let mut byte = [0u8; 1];
    let mut iov = [libc::iovec {
        iov_base: byte.as_mut_ptr() as *mut libc::c_void,
        iov_len: byte.len(),
    }];
    let fd_bytes = expected * std::mem::size_of::<RawFd>();
    let mut control = vec![0u8; unsafe { libc::CMSG_SPACE(fd_bytes as u32) as usize }];
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = iov.as_mut_ptr();
    msg.msg_iovlen = iov.len() as _;
    msg.msg_control = control.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = control.len() as _;

    let read = unsafe { libc::recvmsg(stream.as_raw_fd(), &mut msg, 0) };
    if read < 0 {
        return Err(io::Error::last_os_error());
    }
    if msg.msg_flags & libc::MSG_CTRUNC != 0 {
        return Err(io::Error::other("handoff fd control message was truncated"));
    }

    let mut out = Vec::new();
    unsafe {
        let cmsg = libc::CMSG_FIRSTHDR(&msg);
        if cmsg.is_null()
            || (*cmsg).cmsg_level != libc::SOL_SOCKET
            || (*cmsg).cmsg_type != libc::SCM_RIGHTS
        {
            return Err(io::Error::other("handoff fd message missing SCM_RIGHTS"));
        }
        let data_len = ((*cmsg).cmsg_len as usize).saturating_sub(libc::CMSG_LEN(0) as usize);
        let count = data_len / std::mem::size_of::<RawFd>();
        let data = libc::CMSG_DATA(cmsg) as *const RawFd;
        for idx in 0..count {
            out.push(*data.add(idx));
        }
    }
    if out.len() != expected {
        for fd in out {
            let _ = unsafe { libc::close(fd) };
        }
        return Err(io::Error::other(format!(
            "expected {expected} handoff fds, received fewer"
        )));
    }
    Ok(out)
}

#[cfg(unix)]
pub(crate) fn log_import_result(panes: usize) {
    info!(panes, "handoff import ready");
}

#[cfg(all(unix, test))]
mod tests {
    use super::*;

    #[test]
    fn a_timed_out_handoff_wait_names_its_step_and_budget() {
        let named = name_handoff_wait(
            "replacement server ready",
            Duration::from_secs(30),
            io::Error::from(io::ErrorKind::WouldBlock),
        );
        let message = named.to_string();

        // TP-SRV-HANDOFF-DIAG-01: a handoff wait that runs out of time says which
        // wait it was and how long it was given. Every read in this protocol carries
        // SO_RCVTIMEO, so exhausting one arrives as a bare EAGAIN — "Resource
        // temporarily unavailable (os error 11)" — and that string is what the person
        // running `herdr update` on a busy machine is left holding.
        assert!(
            message.contains("replacement server ready"),
            "the message has to name the step that stalled, got {message:?}"
        );
        assert!(
            message.contains("30s"),
            "the message has to state the budget it stalled against, got {message:?}"
        );
    }

    #[test]
    fn a_handoff_failure_that_is_not_a_timeout_is_left_alone() {
        let original = io::Error::new(
            io::ErrorKind::PermissionDenied,
            "handoff import token mismatch",
        );
        let passed = name_handoff_wait("replacement server ready", READY_TIMEOUT, original);

        // TP-SRV-HANDOFF-DIAG-02: only a timeout is renamed. A token mismatch, a
        // closed stream or a version refusal already says what it is, and wrapping
        // those in waiting language would describe the wrong failure.
        assert_eq!(passed.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(passed.to_string(), "handoff import token mismatch");
    }

    fn freight_test_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "herdr-freight-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create freight test dir");
        dir
    }

    #[test]
    fn handoff_history_freight_path_derives_from_the_socket_path() {
        // TP-HANDOFF-HIST-01 plumbing: the importer holds only the socket path,
        // so the freight has to be findable from it alone — no new manifest
        // field, no new argument, no protocol bump.
        let socket = PathBuf::from("/some/data/dir/herdr-handoff-4242.sock");

        let freight = handoff_history_freight_path(&socket);

        assert_eq!(
            freight,
            PathBuf::from("/some/data/dir/herdr-handoff-4242.history.json")
        );
    }

    #[test]
    fn handoff_history_freight_roundtrips_full_pane_history() {
        let dir = freight_test_dir("roundtrip");
        let socket = dir.join("herdr-handoff-4242.sock");
        let mut panes = std::collections::HashMap::new();
        panes.insert(7u32, "x".repeat(64 * 1024));
        panes.insert(
            9u32,
            "short\r\nwith \u{1b}[31mansi\u{1b}[0m\r\n".to_string(),
        );

        write_history_freight(&socket, panes.clone()).expect("write freight");
        let taken = take_history_freight(&socket).expect("freight present");

        assert_eq!(taken, panes);
        // Consume semantics: the freight is transfer luggage, and luggage left
        // at the platform is a 64 KiB-per-pane disk leak on every update.
        assert!(
            !handoff_history_freight_path(&socket).exists(),
            "taking the freight has to delete the file"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_corrupt_freight_file_degrades_to_inline_replay() {
        let dir = freight_test_dir("degrade");
        let socket = dir.join("herdr-handoff-4242.sock");

        // Missing: an old exporter never wrote one.
        assert!(take_history_freight(&socket).is_none());

        // Corrupt: freight is an enhancement, never a reason to fail the
        // import — and the broken file must not linger for the next attempt.
        std::fs::write(handoff_history_freight_path(&socket), b"{not json").unwrap();
        assert!(take_history_freight(&socket).is_none());
        assert!(
            !handoff_history_freight_path(&socket).exists(),
            "a corrupt freight file has to be deleted, not retried forever"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_freight_files_from_dead_exporters_are_swept() {
        let dir = freight_test_dir("sweep");
        // A pid nothing on this machine holds: pid_max on Linux caps below
        // 2^22 by default, and even a raised pid_max stays under u32::MAX.
        let dead = dir.join(format!("herdr-handoff-{}.history.json", u32::MAX - 1));
        let alive = dir.join(format!("herdr-handoff-{}.history.json", std::process::id()));
        let unrelated = dir.join("herdr-handoff-1234.sock");
        std::fs::write(&dead, b"{}").unwrap();
        std::fs::write(&alive, b"{}").unwrap();
        std::fs::write(&unrelated, b"").unwrap();

        sweep_dead_history_freight(&dir);

        assert!(!dead.exists(), "a dead exporter's freight has to be swept");
        assert!(
            alive.exists(),
            "a live exporter's freight is an in-flight handoff, not garbage"
        );
        assert!(unrelated.exists(), "the sweep only touches freight files");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_sixty_five_pane_session_fits_through_one_handoff() {
        // TP-HANDOFF-FD-01: the fd budget stays ahead of real sessions. A live
        // session was measured at 59 panes while the cap sat at 64 — six more
        // panes and `herdr update --handoff` would refuse the whole session.
        // The kernel takes 253 fds per SCM_RIGHTS message (measured on this
        // machine; SCM_MAX_FD in the kernel source), so the cap is policy, not
        // physics — it just has to clear sessions that actually exist.
        let panes = 65usize;
        assert!(
            panes <= MAX_FDS_PER_HANDOFF,
            "a measured 59-pane session leaves the 64-fd cap six panes of headroom; \
             the cap has to clear at least {panes} (got {MAX_FDS_PER_HANDOFF})"
        );

        // And the cap has to be real: that many fds must survive the same
        // sendmsg/recvmsg pair the live handoff uses, not just the comparison.
        let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
        let fds: Vec<RawFd> = (0..panes)
            .map(|_| {
                let fd = unsafe { libc::dup(devnull.as_raw_fd()) };
                assert!(fd >= 0, "dup(/dev/null) failed");
                fd
            })
            .collect();
        let (sender, receiver) = UnixStream::pair().expect("socketpair");

        send_fds(&sender, &fds).expect("send_fds refused a payload under the cap");
        let received = recv_fds(&receiver, panes).expect("recv_fds lost part of the payload");

        assert_eq!(received.len(), panes);
        for fd in received.iter().chain(fds.iter()) {
            let mut stat: libc::stat = unsafe { std::mem::zeroed() };
            assert_eq!(
                unsafe { libc::fstat(*fd, &mut stat) },
                0,
                "a handed-off fd arrived unusable"
            );
        }
        for fd in received.into_iter().chain(fds) {
            let _ = unsafe { libc::close(fd) };
        }
    }
}
