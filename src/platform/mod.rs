//! Platform-specific process and filesystem operations.
//!
//! Centralizes OS-dependent behavior behind a clean boundary so core
//! modules don't scatter `#[cfg]` branches through product logic.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundProcess {
    pub pid: u32,
    pub name: String,
    pub argv0: Option<String>,
    pub argv: Option<Vec<String>>,
    pub cmdline: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundJob {
    pub process_group_id: u32,
    pub processes: Vec<ForegroundProcess>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Hangup,
    Terminate,
    Kill,
}

/// One entry from the bookmark list the host desktop file manager keeps, in the
/// order the user arranged it.
///
/// `label` is present only when the user renamed the entry. When it is absent
/// the directory name is authoritative and the caller derives the display text,
/// exactly as the desktop file manager does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DesktopBookmark {
    pub(crate) path: std::path::PathBuf,
    pub(crate) label: Option<String>,
}

/// A well-known user directory as the host actually keeps it.
///
/// The freedesktop layout is localized per path element, so the directory a
/// Turkish desktop calls `İndirilenler` holds the same identity an English one
/// calls `Downloads`. `kind` carries that identity; `path` carries the truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserDirectory {
    pub(crate) kind: UserDirectoryKind,
    pub(crate) path: std::path::PathBuf,
}

/// The subset of freedesktop user directories this rail gives its own identity
/// icon to, in the order it draws them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UserDirectoryKind {
    Desktop,
    Downloads,
    Documents,
    Pictures,
    Videos,
    Music,
}

impl UserDirectoryKind {
    pub(crate) const ALL: [Self; 6] = [
        Self::Desktop,
        Self::Downloads,
        Self::Documents,
        Self::Pictures,
        Self::Videos,
        Self::Music,
    ];

    /// The unlocalized name the freedesktop defaults start from, and the only
    /// name available when the host records no localized list.
    pub(crate) const fn default_child(self) -> &'static str {
        match self {
            Self::Desktop => "Desktop",
            Self::Downloads => "Downloads",
            Self::Documents => "Documents",
            Self::Pictures => "Pictures",
            Self::Videos => "Videos",
            Self::Music => "Music",
        }
    }
}

/// What to draw when the host records no localized list: the unlocalized
/// freedesktop names under `home`.
pub(crate) fn well_known_user_directories(home: &std::path::Path) -> Vec<UserDirectory> {
    UserDirectoryKind::ALL
        .iter()
        .map(|&kind| UserDirectory {
            kind,
            path: home.join(kind.default_child()),
        })
        .collect()
}

/// The user directories the host keeps, localized when the platform records
/// that. Only Linux publishes a machine-readable list; elsewhere the
/// unlocalized names are the truth.
#[cfg(not(target_os = "linux"))]
pub(crate) fn user_directories(home: &std::path::Path) -> Vec<UserDirectory> {
    well_known_user_directories(home)
}

/// Bookmarks curated in the host file manager. Only Linux keeps them in the
/// freedesktop text format; macOS Finder uses a binary property list and
/// Windows Quick Access is a shell-namespace concept, so both yield nothing and
/// the rail degrades without special-casing at the call site.
#[cfg(not(target_os = "linux"))]
pub(crate) fn desktop_bookmarks() -> Vec<DesktopBookmark> {
    Vec::new()
}

/// Volumes the host currently has mounted. Only Linux publishes a machine
/// readable mount table this reader trusts.
#[cfg(not(target_os = "linux"))]
pub(crate) fn mounted_volumes() -> Vec<std::path::PathBuf> {
    Vec::new()
}

/// Root of the host's mounted network shares, when the platform has one.
#[cfg(not(target_os = "linux"))]
pub(crate) fn network_mounts_root() -> Option<std::path::PathBuf> {
    None
}

pub(crate) fn detached_custom_command_process(command: &str) -> std::process::Command {
    let mut process = detached_custom_command_process_platform(command);
    configure_background_command(&mut process);
    process
}

pub(crate) fn pane_custom_command_pty_builder(command: &str) -> portable_pty::CommandBuilder {
    pane_custom_command_pty_builder_platform(command)
}

/// Stable same-object identity captured from the host filesystem. Core file
/// operations use this opaque pair to reject path replacement between intent,
/// preflight, and the final write boundary without importing OS APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FileIdentity {
    first: u64,
    second: u64,
}

impl FileIdentity {
    pub(crate) const fn new(first: u64, second: u64) -> Self {
        Self { first, second }
    }
}
pub(crate) fn apply_pane_runtime_marker(command: &mut portable_pty::CommandBuilder) {
    apply_pane_runtime_marker_platform(command);
}

#[cfg(not(windows))]
pub(crate) fn terminal_title_for_presentation(title: &str) -> &str {
    title
}

#[cfg(not(windows))]
fn apply_pane_runtime_marker_platform(_command: &mut portable_pty::CommandBuilder) {}

pub(crate) fn configure_background_command(command: &mut std::process::Command) {
    configure_background_command_platform(command);
}

#[cfg(not(windows))]
fn configure_background_command_platform(_command: &mut std::process::Command) {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlatformCapabilities {
    pub(crate) live_handoff: bool,
    pub(crate) direct_terminal_attach: bool,
    pub(crate) preserve_legacy_doubled_escape_input: bool,
}

pub(crate) const fn capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        live_handoff: cfg!(unix),
        direct_terminal_attach: cfg!(unix),
        preserve_legacy_doubled_escape_input: cfg!(target_os = "macos"),
    }
}

#[cfg(not(windows))]
pub fn launch_server_daemon_command(command: &mut std::process::Command) -> std::io::Result<u32> {
    command.spawn().map(|child| child.id())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn detach_server_daemon_command(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;

    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn current_process_is_detached_server_daemon() -> bool {
    unsafe { libc::getsid(0) == libc::getpid() }
}

/// Raised by the SIGWINCH handler, consumed by the host resize watcher.
#[cfg(unix)]
static TERMINAL_RESIZE_SIGNALLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn record_terminal_resize_signal(_signal: libc::c_int) {
    TERMINAL_RESIZE_SIGNALLED.store(true, std::sync::atomic::Ordering::Release);
}

/// Records SIGWINCH events that size polling can miss.
#[cfg(unix)]
pub(crate) fn watch_terminal_resize_signal() {
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction =
        record_terminal_resize_signal as extern "C" fn(libc::c_int) as libc::sighandler_t;
    // Keep blocking stdin and socket reads from failing with EINTR.
    action.sa_flags = libc::SA_RESTART;
    unsafe {
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGWINCH, &action, std::ptr::null_mut());
    }
}

#[cfg(not(unix))]
pub(crate) fn watch_terminal_resize_signal() {}

/// Returns whether a terminal size change was signalled since the last call.
#[cfg(unix)]
pub(crate) fn take_terminal_resize_signal() -> bool {
    TERMINAL_RESIZE_SIGNALLED.swap(false, std::sync::atomic::Ordering::AcqRel)
}

/// Windows relies on size polling.
#[cfg(not(unix))]
pub(crate) fn take_terminal_resize_signal() -> bool {
    false
}

#[cfg(unix)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardCommand {
    pub program: &'static str,
    pub args: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardImage {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LimitedRead {
    Empty,
    Complete(Vec<u8>),
    Oversized,
}

pub(crate) fn read_limited_reader(
    mut reader: impl std::io::Read,
    max_bytes: usize,
) -> std::io::Result<LimitedRead> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];

    while bytes.len() < max_bytes {
        let remaining = max_bytes - bytes.len();
        let read_len = remaining.min(buffer.len());
        let bytes_read = match reader.read(&mut buffer[..read_len]) {
            Ok(bytes_read) => bytes_read,
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        };
        if bytes_read == 0 {
            return if bytes.is_empty() {
                Ok(LimitedRead::Empty)
            } else {
                Ok(LimitedRead::Complete(bytes))
            };
        }
        bytes.extend_from_slice(&buffer[..bytes_read]);
    }

    let mut sentinel = [0_u8; 1];
    loop {
        return match reader.read(&mut sentinel) {
            Ok(0) if bytes.is_empty() => Ok(LimitedRead::Empty),
            Ok(0) => Ok(LimitedRead::Complete(bytes)),
            Ok(_) => Ok(LimitedRead::Oversized),
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(err) => Err(err),
        };
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteSshConfigPaths {
    pub(crate) user_config: Option<std::path::PathBuf>,
    pub(crate) system_config: Option<std::path::PathBuf>,
    pub(crate) multiplexing: bool,
}

#[cfg(unix)]
mod unix_common;
#[cfg(unix)]
pub(crate) use unix_common::{begin_cli_output, end_cli_output};

#[cfg(not(unix))]
pub(crate) fn begin_cli_output() {}

#[cfg(not(unix))]
pub(crate) fn end_cli_output() {}

/// Opens this platform's native audio output, when it has one wired.
///
/// `None` means "no native path on this platform", which is different from
/// `Some(Err)`: the first sends the caller straight to an external player, the
/// second is a device that exists and declined, worth a line in the log.
#[cfg(target_os = "macos")]
pub(crate) fn native_audio_sink(
) -> Option<Result<Box<dyn crate::media::sink::AudioSink>, crate::media::sink::SinkError>> {
    Some(macos::audio::open())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn native_audio_sink(
) -> Option<Result<Box<dyn crate::media::sink::AudioSink>, crate::media::sink::SinkError>> {
    None
}

/// The user session's runtime directory, when this platform has one and it is
/// really there.
///
/// Everything that talks to the sound server finds its socket through this
/// path, so a process that does not have it is deaf — and silently, because
/// the audio libraries fall back to a null device rather than failing. The
/// directory is checked rather than assumed: pointing a pane at a path that
/// does not exist would trade one silent failure for another.
#[cfg(target_os = "linux")]
pub(crate) fn session_runtime_dir() -> Option<std::path::PathBuf> {
    // SAFETY: geteuid takes no arguments and has no preconditions.
    let uid = unsafe { libc::geteuid() };
    let dir = std::path::PathBuf::from(format!("/run/user/{uid}"));
    dir.is_dir().then_some(dir)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn session_runtime_dir() -> Option<std::path::PathBuf> {
    None
}

/// The runtime directory a child process should be told about, or `None` when
/// it will inherit a usable one.
///
/// One rule, one place. Two callers need it — the pane spawn and the audio
/// tools this server runs itself — and both would otherwise write out "an
/// inherited value wins, an empty one is not a value" in their own words,
/// which is how the two drift apart without either looking wrong.
pub(crate) fn session_runtime_dir_for_child(
    inherited: Option<&std::ffi::OsStr>,
) -> Option<std::path::PathBuf> {
    child_runtime_dir(inherited, session_runtime_dir())
}

/// The rule itself, with both readings handed in.
///
/// Pure so it can be tested on a machine that has no session runtime directory
/// at all — which is every build box this repository runs its tests on.
pub(crate) fn child_runtime_dir(
    inherited: Option<&std::ffi::OsStr>,
    session: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    // An empty value is set and useless at once: treating it as configured
    // leaves the child deaf while looking configured.
    if inherited.is_some_and(|value| !value.is_empty()) {
        return None;
    }
    session
}

/// Why a capture source could not be had, or stopped being had.
///
/// Platform-neutral on purpose: the supervisor that reads it is cross-platform
/// and must compile where nothing can be captured at all.
// The variants are constructed only where a platform can capture, so on a
// target that cannot they are dead by construction rather than by neglect —
// and the cross-target lint is the only thing that sees it. `just lint` runs
// for the host and stays green; `just check` runs `windows-lint` and does not.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum AudioSourceError {
    /// The source could not be started at all.
    Unavailable(String),
    /// It was running and stopped.
    Closed(String),
}

impl std::fmt::Display for AudioSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(detail) => write!(f, "audio source unavailable: {detail}"),
            Self::Closed(detail) => write!(f, "audio source closed: {detail}"),
        }
    }
}

/// Anything that hands over whole protocol frames until it ends.
///
/// `Send`, because the only way to read a blocking source without freezing the
/// server loop is to read it on a thread of its own.
pub(crate) trait FrameSource: Send {
    /// The next whole frame, or `None` once the source has ended.
    fn next_frame(&mut self) -> Result<Option<Vec<u8>>, AudioSourceError>;
    /// Stops it. Idempotent.
    fn close(&mut self) -> Result<(), AudioSourceError>;
}

/// Anything that says the audio graph moved, and nothing about how.
pub(crate) trait GraphSignals: Send {
    /// Blocks until the graph moves. `false` means the watcher itself ended.
    fn next_signal(&mut self) -> Result<bool, AudioSourceError>;
    /// Stops it. Idempotent.
    fn close(&mut self) -> Result<(), AudioSourceError>;
}

/// Starts this platform's graph watcher, or `None` where there is none.
#[cfg(target_os = "linux")]
pub(crate) fn start_graph_watcher() -> Option<Result<Box<dyn GraphSignals>, AudioSourceError>> {
    Some(
        linux::audio_source::GraphWatcher::start()
            .map(|watcher| Box::new(watcher) as Box<dyn GraphSignals>),
    )
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn start_graph_watcher() -> Option<Result<Box<dyn GraphSignals>, AudioSourceError>> {
    None
}

/// Aims this platform's recorder at one stream, or `None` where there is none.
#[cfg(target_os = "linux")]
pub(crate) fn capture_stream(
    object_serial: u32,
) -> Option<Result<Box<dyn FrameSource>, AudioSourceError>> {
    Some(
        linux::audio_source::ExternalSource::capture(object_serial)
            .map(|source| Box::new(source) as Box<dyn FrameSource>),
    )
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn capture_stream(
    _object_serial: u32,
) -> Option<Result<Box<dyn FrameSource>, AudioSourceError>> {
    None
}

/// Reads this platform's audio graph, or `None` where there is none.
///
/// Running the reader and parsing what it printed are separate on purpose: the
/// parsing half is tested from fixtures on every platform, and only this half
/// needs a live sound server.
#[cfg(target_os = "linux")]
pub(crate) fn read_output_streams() -> Option<Result<Vec<AudioOutputStream>, AudioSourceError>> {
    Some(linux::audio_source::read_output_streams())
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn read_output_streams() -> Option<Result<Vec<AudioOutputStream>, AudioSourceError>> {
    None
}

// Staged: the graph reader fills these in and the supervisor's rules read
// them, but the driver that connects the two has not landed yet.
//
// REMOVAL CONDITION: drop both attributes once the pane-audio supervisor
// consumes an `AudioOutputStream` outside tests.
#[allow(dead_code)]
/// How much the pid can be trusted, which is worth carrying because the two
/// answers come from different places and deserve different log lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PidTrust {
    /// The kernel told PipeWire who connected.
    Verified,
    /// The program told PipeWire who it is.
    SelfReported,
}

#[allow(dead_code)]
/// One capturable audio output stream, as this platform's audio graph
/// describes it. Plain data with no `cfg`: the rules that read it are
/// cross-platform and testable on machines that can capture nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AudioOutputStream {
    pub(crate) node_id: u32,
    pub(crate) pid: Option<u32>,
    pub(crate) pid_trust: Option<PidTrust>,
    pub(crate) app_name: Option<String>,
    /// What `pw-record --target` wants. The graph's object id and this serial
    /// are different numbers, and targeting with the wrong one captures the
    /// wrong stream — quietly, because both are valid ids for something.
    pub(crate) object_serial: Option<u32>,
}

// Staged with the types above: the supervisor's rules read these, the driver
// that calls them has not landed yet.
//
// REMOVAL CONDITION: drop the four attributes below once the pane-audio
// supervisor walks a process chain outside tests.
/// The parent of `pid`, or `None` at the top of the tree.
///
/// `init` is never returned: every process descends from it, so treating it as
/// an ancestor would make every stream look like it belonged to every pane.
#[allow(dead_code)]
#[cfg(unix)]
pub(crate) fn process_parent(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parent_pid_from_stat(&stat)
}

#[allow(dead_code)]
#[cfg(not(unix))]
pub(crate) fn process_parent(_pid: u32) -> Option<u32> {
    None
}

/// Parses the parent out of a `/proc/<pid>/stat` line.
///
/// Split from the read on purpose, the way the CPU counters are: the parsing
/// is the part that can be wrong, and it is tested with fixtures on every
/// platform, including the ones with no `/proc` at all.
#[allow(dead_code)]
fn parent_pid_from_stat(stat: &str) -> Option<u32> {
    // "pid (comm) state ppid ..." — comm may hold spaces and parens, so the
    // fields are counted from the last ')'.
    let rest = stat.get(stat.rfind(')')? + 2..)?;
    let ppid: u32 = rest.split_whitespace().nth(1)?.parse().ok()?;
    (ppid > 1).then_some(ppid)
}

/// Whether `pid`'s environment carries `key=value`.
#[allow(dead_code)]
#[cfg(unix)]
pub(crate) fn process_environ_has(pid: u32, key: &str, value: &str) -> bool {
    let Ok(environ) = std::fs::read(format!("/proc/{pid}/environ")) else {
        return false;
    };
    environ_has(&environ, key, value)
}

#[allow(dead_code)]
#[cfg(not(unix))]
pub(crate) fn process_environ_has(_pid: u32, _key: &str, _value: &str) -> bool {
    false
}

/// The pure half of the environment check: a NUL-separated block, matched
/// whole rather than by substring so `HERDR_WEB_SESSIONX` cannot pass for
/// `HERDR_WEB_SESSION`.
#[allow(dead_code)]
fn environ_has(environ: &[u8], key: &str, value: &str) -> bool {
    let wanted = format!("{key}={value}");
    environ
        .split(|&byte| byte == 0)
        .any(|record| record == wanted.as_bytes())
}

/// Whether a native output device exists, without opening it. Read at the
/// handshake, where opening a device for a stream nobody has sent would be
/// both slow and presumptuous.
#[cfg(target_os = "macos")]
pub(crate) fn native_audio_sink_available() -> bool {
    macos::audio::probe()
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn native_audio_sink_available() -> bool {
    false
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod fallback;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub use fallback::*;

/// The machine's CPU counters, where this build knows how to find them.
///
/// Only Linux has a reader today, and that is a deliberate limit rather than an
/// oversight: the counters come out of `/proc`, which macOS does not have and
/// Windows spells entirely differently. Reaching for a portable crate to close
/// the gap was measured and declined — on Windows it brings a tree of seven
/// `windows-*` crates for a meter, and every dependency added here is a
/// `Cargo.lock` conflict on every upstream sync of this fork.
///
/// The platforms without a reader are not left guessing. `None` travels all the
/// way to the screen and renders as `--`, which is the same thing a Linux box
/// with an unreadable `/proc` shows. Swapping in a portable implementation
/// later changes only these two functions.
// TP-RES-06: a platform with no reader says so; it never reports zero.
#[cfg(not(target_os = "linux"))]
pub(crate) fn read_cpu_times() -> Option<crate::resource::CpuTimes> {
    None
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn read_memory() -> (
    Option<crate::resource::Usage>,
    Option<crate::resource::Usage>,
) {
    (None, None)
}

/// The four metrics added after the first three, on the platforms with no
/// reader for them yet.
///
/// `None` rather than a plausible number, for the same reason the two above
/// return `None`: a section showing `--` says "herdr cannot read this here",
/// and one showing `0%` says "your battery is flat". Only one of those is true,
/// and the false one is the one somebody would act on.
// TP-RES-06: a platform with no reader says so; it never reports zero.
#[cfg(not(target_os = "linux"))]
pub(crate) fn read_disk() -> Option<crate::resource::Usage> {
    None
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn read_battery() -> Option<f32> {
    None
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn read_net_total() -> Option<u64> {
    None
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn read_temperature() -> Option<f32> {
    None
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn available_pane_shell_from_job(child_pid: u32, job: ForegroundJob) -> Option<String> {
    if job.process_group_id != child_pid
        || job.processes.iter().any(|process| process.pid != child_pid)
    {
        return None;
    }
    job.processes
        .into_iter()
        .find(|process| process.pid == child_pid)
        .map(|process| process.name)
        .filter(|name| is_pane_shell_process_name(name))
}

fn normalized_process_name(name: &str) -> String {
    name.rsplit(['/', '\\'])
        .next()
        .unwrap_or(name)
        .trim_start_matches('-')
        .trim_end_matches(".exe")
        .to_ascii_lowercase()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn is_powershell_process_name(name: &str) -> bool {
    matches!(
        normalized_process_name(name).as_str(),
        "pwsh" | "powershell"
    )
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn interactive_unix_shell_command(
    argv: &[String],
    shell_name: &str,
    quote_posix_arg: fn(&str) -> String,
) -> Option<String> {
    let quote = if is_powershell_process_name(shell_name) {
        quote_powershell_arg
    } else {
        quote_posix_arg
    };
    let mut parts = argv.iter();
    let mut command = quote(parts.next()?);
    for part in parts {
        command.push(' ');
        command.push_str(&quote(part));
    }
    Some(command)
}

pub(crate) fn quote_powershell_arg(value: &str) -> String {
    if !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':' | b'+' | b'=')
        })
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "''"))
}

pub(crate) fn is_pane_shell_process_name(name: &str) -> bool {
    let normalized = normalized_process_name(name);
    matches!(
        normalized.as_str(),
        "sh" | "bash"
            | "dash"
            | "zsh"
            | "fish"
            | "ksh"
            | "mksh"
            | "csh"
            | "tcsh"
            | "elvish"
            | "xonsh"
            | "nu"
            | "pwsh"
            | "powershell"
            | "cmd"
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn process_agent_hint(_pid: u32) -> Option<crate::detect::Agent> {
    None
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn parse_agent_env_hint(environ: &[u8]) -> Option<crate::detect::Agent> {
    for record in environ.split(|&byte| byte == 0) {
        let Some(value) = record.strip_prefix(b"HERDR_AGENT=") else {
            continue;
        };
        return crate::detect::parse_agent_label(std::str::from_utf8(value).ok()?);
    }
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[derive(Debug)]
pub(crate) struct InputSourceRestore;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn switch_to_ascii_input_source() -> Option<InputSourceRestore> {
    None
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn pump_input_source_runloop() {}

/// Switches the host keyboard input source while prefix mode is active.
///
/// `App` drives this through a trait so the prefix-mode transitions can be
/// tested with a fake, without touching the real macOS APIs or leaking a
/// platform-specific restore type into `App`.
pub(crate) trait PrefixInputSource {
    /// Switch to an ASCII-capable input source for prefix commands. No-op if
    /// the current source is already ASCII-capable, the platform is
    /// unsupported, or the switch fails. Calling it again before `restore`
    /// keeps the source saved by the first call.
    fn switch_to_ascii(&mut self);

    /// Restore whatever `switch_to_ascii` saved. No-op if nothing was switched.
    fn restore(&mut self);
}

/// Production [`PrefixInputSource`] backed by the per-platform API.
#[derive(Default)]
pub(crate) struct RealPrefixInputSource {
    restore: Option<InputSourceRestore>,
}

impl PrefixInputSource for RealPrefixInputSource {
    fn switch_to_ascii(&mut self) {
        if self.restore.is_none() {
            // Drain pending input-source-change notifications so the read below is fresh (see
            // `pump_input_source_runloop`); a no-op on non-macOS.
            pump_input_source_runloop();
            self.restore = switch_to_ascii_input_source();
        }
    }

    fn restore(&mut self) {
        let _ = self.restore.take();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn terminal_resize_signal_is_recorded_once_per_delivery() {
        watch_terminal_resize_signal();
        assert!(!take_terminal_resize_signal());

        unsafe {
            libc::raise(libc::SIGWINCH);
        }

        assert!(take_terminal_resize_signal());
        assert!(!take_terminal_resize_signal());
    }

    /// PP-1 — the ordinary line.
    #[test]
    fn the_parent_is_read_from_a_stat_line() {
        assert_eq!(
            parent_pid_from_stat("4242 (electron) S 4200 4242 4200 0 -1 0"),
            Some(4200)
        );
    }

    /// PP-2 — a command name may hold spaces and parens, which is exactly why
    /// the fields are counted from the last ')' and not split from the front.
    #[test]
    fn a_command_name_with_spaces_and_parens_does_not_shift_the_fields() {
        assert_eq!(
            parent_pid_from_stat("7 (Web Content (main)) S 3 7 3 0 -1 0"),
            Some(3)
        );
    }

    /// PP-3 — init is not an ancestor worth having: every process descends
    /// from it, so returning it would make every stream look like every
    /// pane's.
    #[test]
    fn init_is_not_reported_as_a_parent() {
        assert_eq!(
            parent_pid_from_stat("900 (pipewire) S 1 900 1 0 -1 0"),
            None
        );
    }

    /// PP-4 — an unreadable line is no parent, never a panic in a server loop.
    #[test]
    fn an_unparseable_stat_line_has_no_parent() {
        assert_eq!(parent_pid_from_stat("garbage"), None);
        assert_eq!(parent_pid_from_stat(""), None);
    }

    /// EN-1 — the marker is matched whole.
    #[test]
    fn an_environment_marker_is_found() {
        let environ = b"PATH=/usr/bin\0HERDR_WEB_SESSION=abc\0HOME=/home\0";
        assert!(environ_has(environ, "HERDR_WEB_SESSION", "abc"));
    }

    /// EN-2 — a longer name that merely starts the same must not pass, and
    /// neither must a value that merely starts the same.
    #[test]
    fn a_near_miss_environment_marker_does_not_pass() {
        let environ = b"HERDR_WEB_SESSIONX=abc\0HERDR_WEB_SESSION=abcdef\0";
        assert!(!environ_has(environ, "HERDR_WEB_SESSION", "abc"));
    }

    /// EN-3 — an absent marker is absent, and an unreadable one is too.
    #[test]
    fn a_missing_environment_marker_is_absent() {
        assert!(!environ_has(b"HOME=/home\0", "HERDR_WEB_SESSION", "abc"));
        assert!(!environ_has(b"", "HERDR_WEB_SESSION", "abc"));
    }

    #[test]
    fn pane_shell_process_names_reject_exec_replacement_programs() {
        for shell in ["bash", "-zsh", "/bin/fish", "pwsh", "powershell.exe"] {
            assert!(is_pane_shell_process_name(shell), "{shell}");
        }
        for program in ["vim", "nvim", "cargo", "test-runner", "opencode"] {
            assert!(!is_pane_shell_process_name(program), "{program}");
        }
    }

    #[test]
    fn detached_custom_command_preserves_unix_login_shell_flag() {
        let cmd = detached_custom_command_process("echo hello");
        assert_eq!(cmd.get_program(), std::ffi::OsStr::new("/bin/sh"));
        assert_eq!(
            cmd.get_args().collect::<Vec<_>>(),
            [
                std::ffi::OsStr::new("-lc"),
                std::ffi::OsStr::new("echo hello")
            ]
        );
    }

    #[test]
    fn pane_custom_command_builder_preserves_unix_shell_flag() {
        let expected: Vec<std::ffi::OsString> =
            vec!["/bin/sh".into(), "-c".into(), "echo hello".into()];
        assert_eq!(
            pane_custom_command_pty_builder("echo hello").get_argv(),
            &expected
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn parse_agent_env_hint_accepts_known_agents() {
        assert_eq!(
            parse_agent_env_hint(b"PATH=/bin\0HERDR_AGENT=claude\0TERM=xterm\0"),
            Some(crate::detect::Agent::Claude)
        );
        assert_eq!(
            parse_agent_env_hint(b"HERDR_AGENT=codex"),
            Some(crate::detect::Agent::Codex)
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn parse_agent_env_hint_ignores_missing_or_unknown_agents() {
        assert_eq!(parse_agent_env_hint(b"PATH=/bin\0TERM=xterm\0"), None);
        assert_eq!(parse_agent_env_hint(b"HERDR_AGENT=not-an-agent\0"), None);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn interactive_shell_command_quotes_for_posix_and_powershell() {
        let argv = vec![
            "pi".into(),
            String::new(),
            "two words".into(),
            "a'b".into(),
            "$HOME".into(),
            "semi;colon".into(),
            "@options".into(),
        ];
        assert_eq!(
            interactive_shell_command(&argv, "bash").as_deref(),
            Some("pi '' 'two words' 'a'\\''b' '$HOME' 'semi;colon' @options")
        );
        assert_eq!(
            interactive_shell_command(&argv, "pwsh").as_deref(),
            Some("pi '' 'two words' 'a''b' '$HOME' 'semi;colon' '@options'")
        );
    }

    #[test]
    fn read_limited_reader_returns_complete_data_under_limit() {
        let input = std::io::Cursor::new(b"image".to_vec());
        assert_eq!(
            read_limited_reader(input, 16).expect("limited read"),
            LimitedRead::Complete(b"image".to_vec())
        );
    }

    #[test]
    fn read_limited_reader_returns_empty_for_empty_input() {
        let input = std::io::Cursor::new(Vec::<u8>::new());
        assert_eq!(
            read_limited_reader(input, 16).expect("limited read"),
            LimitedRead::Empty
        );
    }

    #[test]
    fn read_limited_reader_accepts_data_exactly_at_limit() {
        let input = std::io::Cursor::new(b"four".to_vec());
        assert_eq!(
            read_limited_reader(input, 4).expect("limited read"),
            LimitedRead::Complete(b"four".to_vec())
        );
    }

    #[test]
    fn read_limited_reader_rejects_data_over_limit() {
        let input = std::io::Cursor::new(b"oversized".to_vec());
        assert_eq!(
            read_limited_reader(input, 4).expect("limited read"),
            LimitedRead::Oversized
        );
    }

    #[test]
    fn read_limited_reader_retries_interrupted_reads() {
        struct InterruptedOnce {
            interrupted: bool,
            inner: std::io::Cursor<Vec<u8>>,
        }

        impl std::io::Read for InterruptedOnce {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if !self.interrupted {
                    self.interrupted = true;
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                self.inner.read(buffer)
            }
        }

        let input = InterruptedOnce {
            interrupted: false,
            inner: std::io::Cursor::new(b"image".to_vec()),
        };
        assert_eq!(
            read_limited_reader(input, 16).expect("limited read"),
            LimitedRead::Complete(b"image".to_vec())
        );
    }
}
