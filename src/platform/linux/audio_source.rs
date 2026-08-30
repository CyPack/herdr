//! Which PipeWire streams can be captured, and which process really made them.
//!
//! Reading the graph is the easy half. Naming the owner is the half that was
//! measured wrong twice, so the rule here is written from the graph as it
//! actually looked on a live desktop rather than from the property names' own
//! promises.
//!
//! `pipewire.sec.pid` is the kernel-verified pid of the *connection*, which
//! makes it the honest answer only when the program connected to PipeWire
//! itself. Everything that speaks the Pulse protocol reaches the graph through
//! one bridge, and every one of those connections carries the bridge's pid. On
//! the measured desktop seven of twelve clients — a browser, a second browser,
//! a speech daemon, two volume controls, a sound-effect library — all reported
//! the same verified pid, which belongs to none of them. A rule that trusts
//! that field alone attributes every ordinary application's sound to one
//! process whose ancestry reaches no pane at all, and so matches nothing,
//! forever, without ever looking wrong.
//!
//! The bridge is found from the graph itself instead of from a list of names:
//! a pid that fronts two or more clients is multiplexing, not producing. When
//! the verified pid turns out to be such a pid, the owner is taken from the
//! self-reported `application.process.id` and marked as self-reported. That is
//! safe here because the only thing done with a pid afterwards is an ancestry
//! walk through `/proc`: a lie would have to name a process that already sits
//! inside the pane's own tree, which is not a lie worth telling.
//!
//! TP-MEDIA-GRAPH-01.

// Unused until the supervisor that watches the graph lands: the reader is
// built and pinned first so the driver has something already tested to call.
//
// REMOVAL CONDITION: delete this attribute the moment `parse_output_streams`
// is called from the pane-audio supervisor — after that, a dead item here is a
// real leak, not a staged one.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Read;
use std::process::{Child, ChildStdout, Command, Stdio};

use serde_json::Value;

use crate::media::{CHANNELS, FRAME_SAMPLES, SAMPLE_RATE_HZ};

use crate::platform::{AudioOutputStream, PidTrust};

const AUDIO_OUTPUT_CLASS: &str = "Stream/Output/Audio";

/// Reads a `pw-dump` document into the output streams it describes.
pub(crate) fn parse_output_streams(
    dump: &str,
) -> Result<Vec<AudioOutputStream>, serde_json::Error> {
    let objects: Vec<Value> = serde_json::from_str(dump)?;

    // Clients first: a node points at the client that opened it, and the
    // owner's identity lives there rather than on the node.
    let mut clients: HashMap<u32, &Value> = HashMap::new();
    let mut fronted: HashMap<u32, usize> = HashMap::new();
    for object in &objects {
        if !is_interface(object, "Client") {
            continue;
        }
        let (Some(id), Some(props)) = (object_id(object), object_props(object)) else {
            continue;
        };
        clients.insert(id, props);
        if let Some(pid) = prop_u32(Some(props), "pipewire.sec.pid") {
            *fronted.entry(pid).or_default() += 1;
        }
    }

    let mut streams = Vec::new();
    for object in &objects {
        if !is_interface(object, "Node") {
            continue;
        }
        let (Some(node_id), Some(props)) = (object_id(object), object_props(object)) else {
            continue;
        };
        if prop_str(Some(props), "media.class") != Some(AUDIO_OUTPUT_CLASS) {
            continue;
        }
        let client = prop_u32(Some(props), "client.id")
            .and_then(|client_id| clients.get(&client_id).copied());
        let (pid, pid_trust) = resolve_owner(props, client, &fronted);
        let app_name = prop_str(Some(props), "application.name")
            .or_else(|| prop_str(Some(props), "node.name"))
            .or_else(|| prop_str(client, "application.name"))
            .map(str::to_owned);
        streams.push(AudioOutputStream {
            node_id,
            pid,
            pid_trust,
            app_name,
            object_serial: prop_u32(Some(props), "object.serial"),
        });
    }
    Ok(streams)
}

/// Names the process behind a stream, preferring what the kernel saw over what
/// the program says — except where the kernel saw a multiplexer.
fn resolve_owner(
    node: &Value,
    client: Option<&Value>,
    fronted: &HashMap<u32, usize>,
) -> (Option<u32>, Option<PidTrust>) {
    for props in [Some(node), client].into_iter().flatten() {
        if let Some(pid) = prop_u32(Some(props), "pipewire.sec.pid") {
            if !is_multiplexer(pid, fronted) {
                return (Some(pid), Some(PidTrust::Verified));
            }
        }
    }
    for props in [Some(node), client].into_iter().flatten() {
        if let Some(pid) = prop_u32(Some(props), "application.process.id") {
            return (Some(pid), Some(PidTrust::SelfReported));
        }
    }
    (None, None)
}

/// A pid that fronts more than one client is forwarding other programs' sound,
/// so it names the bridge rather than the producer. Read from the graph, not
/// from a list of process names, because the list would be wrong on the first
/// desktop that bridges through something else.
fn is_multiplexer(pid: u32, fronted: &HashMap<u32, usize>) -> bool {
    fronted.get(&pid).is_some_and(|clients| *clients > 1)
}

fn is_interface(object: &Value, suffix: &str) -> bool {
    object
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.ends_with(suffix))
}

fn object_id(object: &Value) -> Option<u32> {
    prop_u32(Some(object), "id")
}

fn object_props(object: &Value) -> Option<&Value> {
    object.get("info")?.get("props")
}

/// Props survive several PipeWire versions in which the same key is a number
/// in one and a string in the next, so both are read.
fn prop_u32(props: Option<&Value>, key: &str) -> Option<u32> {
    let value = props?.get(key)?;
    match value {
        Value::Number(number) => number.as_u64().and_then(|n| u32::try_from(n).ok()),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn prop_str<'a>(props: Option<&'a Value>, key: &str) -> Option<&'a str> {
    props?.get(key)?.as_str()
}

/// One frame of the audio protocol, in bytes.
///
/// Derived, never written out. The same number lives in `app::pane_audio` as
/// `FRAME_BYTES`, and `pcm_from_f32le` refuses a body of any other length — so
/// a literal here would be a second definition of one truth. Change the frame
/// length and a literal keeps recording at the old size: every frame is then
/// refused, the listener hears nothing, and no test turns red, because each
/// side stays consistent with its own copy.
pub(crate) const SOURCE_FRAME_BYTES: usize = FRAME_SAMPLES * CHANNELS as usize * 4;

/// The platform-neutral error, under the name this module has always used.
///
/// It moved up rather than being duplicated: the supervisor that handles these
/// errors compiles on platforms this module does not, and two enums with the
/// same variants would be one truth in two places again.
pub(crate) use crate::platform::AudioSourceError as SourceError;

/// Cuts an arbitrary byte stream into whole frames, keeping the remainder.
///
/// The recorder promises nothing about block sizes, and the protocol refuses a
/// partial frame by contract: a frame quietly padded or truncated drifts the
/// clock by that much on *every* frame — inaudible once, obvious after a
/// minute, and impossible to trace back afterwards.
pub(crate) struct Reframer {
    frame_bytes: usize,
    buffer: Vec<u8>,
}

impl Reframer {
    pub(crate) fn new() -> Self {
        Self::with_frame_bytes(SOURCE_FRAME_BYTES)
    }

    /// The seam a test uses to work in small numbers instead of 7680 at a time.
    pub(crate) fn with_frame_bytes(frame_bytes: usize) -> Self {
        Self {
            frame_bytes: frame_bytes.max(1),
            buffer: Vec::with_capacity(frame_bytes.max(1) * 2),
        }
    }

    pub(crate) fn push(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
    }

    pub(crate) fn next_frame(&mut self) -> Option<Vec<u8>> {
        if self.buffer.len() < self.frame_bytes {
            return None;
        }
        Some(self.buffer.drain(..self.frame_bytes).collect())
    }

    /// Bytes held back because they do not yet make a whole frame.
    pub(crate) fn pending(&self) -> usize {
        self.buffer.len()
    }
}

/// The recorder's argument list, built where it can be read in a test rather
/// than assembled inside a spawn nobody can see.
///
/// `--target` takes the stream's **serial**, not the graph object id: both are
/// valid ids for something, so aiming with the wrong one records the wrong
/// stream and reports no error at all.
pub(crate) fn capture_args(object_serial: u32) -> Vec<String> {
    // The rate and the channel count are the protocol's, not the recorder's:
    // a recorder aimed with different numbers produces frames the server will
    // refuse, and it reports no error while doing it.
    [
        "--target",
        &object_serial.to_string(),
        "--rate",
        &SAMPLE_RATE_HZ.to_string(),
        "--channels",
        &CHANNELS.to_string(),
        "--format",
        "f32",
        // stdout, so the frames arrive on a pipe rather than in a file nobody
        // asked for.
        "-",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// The program that records one stream off the graph.
///
/// Named here rather than inline so a test can say which program it means and
/// so the one place that has to exist on the machine is visible in the source.
pub(crate) const RECORDER: &str = "pw-record";

/// One recorder process, read as whole protocol frames.
///
/// TP-MEDIA-RECORDER-01.
///
/// The mirror of `media::sink::ExternalSink`, and deliberately *not* its exact
/// reflection at close: a player is told to stop by dropping its stdin,
/// because killing it would cut off audio it has already buffered. A recorder
/// has nothing buffered to lose and will not notice a closed pipe until its
/// next write — which on a silent stream may never come. So the source kills,
/// and then waits, because a kill without a wait leaves a zombie and a video
/// opened twice an hour would leave a row of them.
// Hand-written: `Child` is Debug, but what identifies a source is which
// recorder it is and whether it is still readable, not the handle's innards.
impl std::fmt::Debug for ExternalSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalSource")
            .field("program", &self.stream.program())
            .field("open", &self.stream.is_open())
            .finish()
    }
}

/// A child process read as a stream of bytes and stopped by killing it.
///
/// Extracted because two things need it now — the recorder and the graph
/// watcher — and the part that would have been copied is exactly the part that
/// leaves a live process behind when it is copied wrong. One implementation,
/// one set of tests, one place to be right.
///
/// TP-MEDIA-RECORDER-01.
pub(crate) struct ChildStream {
    child: Child,
    stdout: Option<ChildStdout>,
    program: String,
}

impl ChildStream {
    /// Starts `program` with its stdout on a pipe and its stderr discarded.
    pub(crate) fn spawn<S: AsRef<std::ffi::OsStr>>(
        program: &str,
        args: &[S],
    ) -> Result<Self, SourceError> {
        let mut command = Command::new(program);
        // The server is a daemon and its environment is not the user's login
        // environment: measured live, it had lost XDG_RUNTIME_DIR entirely.
        // Every one of these programs finds the sound server through that path
        // and answers "can't connect" without it — which arrives here as a
        // process that starts, says nothing on stdout, and ends, i.e. as
        // silence with no error anywhere.
        if let Some(dir) = crate::platform::session_runtime_dir_for_child(
            std::env::var_os("XDG_RUNTIME_DIR").as_deref(),
        ) {
            command.env("XDG_RUNTIME_DIR", dir);
        }
        let mut child = command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            // These programs narrate to stderr on every graph change; kept off
            // the terminal because this runs under a live session.
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| SourceError::Unavailable(format!("{program}: {err}")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SourceError::Unavailable(format!("{program}: no stdout")))?;
        Ok(Self {
            child,
            stdout: Some(stdout),
            program: program.to_string(),
        })
    }

    /// Reads whatever is ready into `buf`. `None` means the stream has ended.
    ///
    /// The caller owns the buffer so this hands back a count rather than a
    /// borrow: a borrow would forbid the recorder from pushing those bytes into
    /// its own reframer while still holding it.
    pub(crate) fn read_into(&mut self, buf: &mut [u8]) -> Result<Option<usize>, SourceError> {
        let Some(stdout) = self.stdout.as_mut() else {
            return Ok(None);
        };
        let read = stdout
            .read(buf)
            .map_err(|err| SourceError::Closed(format!("{}: {err}", self.program)))?;
        if read == 0 {
            self.stdout = None;
            return Ok(None);
        }
        Ok(Some(read))
    }

    /// Stops the process. Idempotent, because `Drop` calls it too.
    pub(crate) fn close(&mut self) -> Result<(), SourceError> {
        // Order matters. Dropping the pipe first means the child's next write
        // fails, which is the only signal it would ever get on its own — and a
        // producer with nothing to say never makes that write, so the kill is
        // what actually ends it. The wait is not optional: a killed child that
        // is never reaped is a zombie, and a pane whose video is opened and
        // closed through an afternoon would leave a row of them.
        //
        // The kill-then-wait shape is `sound::terminate_and_reap`'s, including
        // the part that looks redundant and is not: a kill can fail because the
        // child is already gone, which is fine, or because it is still there
        // and could not be signalled, which is not — and waiting on the second
        // case blocks the caller forever. `try_wait` tells the two apart.
        self.stdout = None;
        if let Err(kill_err) = self.child.kill() {
            match self.child.try_wait() {
                Ok(Some(_)) => {}
                _ => {
                    return Err(SourceError::Closed(format!(
                        "{}: could not be stopped: {kill_err}",
                        self.program
                    )));
                }
            }
        }
        self.child
            .wait()
            .map(|_| ())
            .map_err(|err| SourceError::Closed(format!("{}: {err}", self.program)))
    }

    /// Whether the process has already been reaped.
    pub(crate) fn exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// Whether the pipe is still readable.
    pub(crate) fn is_open(&self) -> bool {
        self.stdout.is_some()
    }

    pub(crate) fn program(&self) -> &str {
        &self.program
    }
}

impl Drop for ChildStream {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

pub(crate) struct ExternalSource {
    stream: ChildStream,
    reframer: Reframer,
    scratch: Vec<u8>,
}

impl ExternalSource {
    /// Starts `program`, expecting whole f32 samples on its stdout.
    pub(crate) fn spawn<S: AsRef<std::ffi::OsStr>>(
        program: &str,
        args: &[S],
    ) -> Result<Self, SourceError> {
        Ok(Self {
            stream: ChildStream::spawn(program, args)?,
            reframer: Reframer::new(),
            scratch: vec![0u8; SOURCE_FRAME_BYTES],
        })
    }

    /// Aims the shipped recorder at one stream.
    pub(crate) fn capture(object_serial: u32) -> Result<Self, SourceError> {
        Self::spawn(RECORDER, &capture_args(object_serial))
    }

    /// The next whole frame, or `None` once the recorder has ended.
    ///
    /// Blocks, on purpose. The recorder produces in real time — one frame every
    /// twenty milliseconds — so a caller that could not wait would be spinning
    /// through the other nineteen. The supervisor reads it on a thread of its
    /// own, which is where `ExternalSink` sits too.
    pub(crate) fn next_frame(&mut self) -> Result<Option<Vec<u8>>, SourceError> {
        loop {
            if let Some(frame) = self.reframer.next_frame() {
                return Ok(Some(frame));
            }
            let Some(read) = self.stream.read_into(&mut self.scratch)? else {
                // End of stream. Whatever is still held back cannot make a
                // whole frame, and a partial frame is not ours to send: the
                // protocol refuses it and padding it would shift the clock.
                return Ok(None);
            };
            self.reframer.push(&self.scratch[..read]);
        }
    }

    /// Stops the recorder. Idempotent, because the stream's `Drop` closes it.
    pub(crate) fn close(&mut self) -> Result<(), SourceError> {
        self.stream.close()
    }

    /// Whether the recorder has already been reaped.
    pub(crate) fn exited(&mut self) -> bool {
        self.stream.exited()
    }

    pub(crate) fn program(&self) -> &str {
        self.stream.program()
    }
}

/// The program that reports graph changes.
pub(crate) const WATCHER: &str = "pw-mon";

/// Watches the graph and says only that it moved.
///
/// The output is never parsed. `pw-mon` has no machine-readable mode and its
/// text is a version's habit rather than a contract, so a parser here would
/// turn a PipeWire upgrade into silence — the failure this whole feature exists
/// to stop. What is needed from it is one bit, "something changed", and one bit
/// survives any reformatting. The graph itself is then read with `pw-dump`,
/// which does have a contract.
///
/// The read blocks, which is what makes it cheap: a watcher that polled would
/// cost something on an idle desktop, and an idle desktop is the case that has
/// to cost nothing.
///
/// TP-MEDIA-WATCH-02.
pub(crate) struct GraphWatcher {
    stream: ChildStream,
    scratch: Vec<u8>,
}

impl std::fmt::Debug for GraphWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphWatcher")
            .field("program", &self.stream.program())
            .field("open", &self.stream.is_open())
            .finish()
    }
}

impl GraphWatcher {
    /// Starts the shipped watcher.
    pub(crate) fn start() -> Result<Self, SourceError> {
        Self::spawn(WATCHER, &[] as &[&str])
    }

    /// The selection seam, so a test can watch something it controls instead of
    /// a program whose presence depends on the machine.
    pub(crate) fn spawn<S: AsRef<std::ffi::OsStr>>(
        program: &str,
        args: &[S],
    ) -> Result<Self, SourceError> {
        Ok(Self {
            stream: ChildStream::spawn(program, args)?,
            // Small on purpose: the bytes are thrown away, and a big buffer
            // would only mean holding more of what is not read.
            scratch: vec![0u8; 4096],
        })
    }

    /// Blocks until the graph moves. `false` means the watcher itself ended.
    ///
    /// Whatever arrived is discarded without being looked at. Two changes that
    /// land in one read are one signal, which is correct: the debouncer would
    /// have collapsed them anyway.
    pub(crate) fn next_signal(&mut self) -> Result<bool, SourceError> {
        Ok(self.stream.read_into(&mut self.scratch)?.is_some())
    }

    /// Stops the watcher. Idempotent.
    pub(crate) fn close(&mut self) -> Result<(), SourceError> {
        self.stream.close()
    }

    /// Whether the watcher has already been reaped.
    pub(crate) fn exited(&mut self) -> bool {
        self.stream.exited()
    }

    pub(crate) fn program(&self) -> &str {
        self.stream.program()
    }
}

/// The program that prints the graph.
pub(crate) const GRAPH_READER: &str = "pw-dump";

/// Runs the graph reader and parses what it printed.
///
/// Split from `parse_output_streams` so the half that needs a live sound server
/// is the only half that needs one: every rule about what the graph *means* is
/// tested from fixtures, on machines that can capture nothing.
pub(crate) fn read_output_streams() -> Result<Vec<AudioOutputStream>, SourceError> {
    let output = Command::new(GRAPH_READER)
        .output()
        .map_err(|err| SourceError::Unavailable(format!("{GRAPH_READER}: {err}")))?;
    if !output.status.success() {
        return Err(SourceError::Unavailable(format!(
            "{GRAPH_READER} exited with {}",
            output.status
        )));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|err| SourceError::Closed(format!("{GRAPH_READER}: {err}")))?;
    parse_output_streams(&text).map_err(|err| SourceError::Closed(format!("{GRAPH_READER}: {err}")))
}

impl crate::platform::FrameSource for ExternalSource {
    fn next_frame(&mut self) -> Result<Option<Vec<u8>>, SourceError> {
        ExternalSource::next_frame(self)
    }

    fn close(&mut self) -> Result<(), SourceError> {
        ExternalSource::close(self)
    }
}

impl crate::platform::GraphSignals for GraphWatcher {
    fn next_signal(&mut self) -> Result<bool, SourceError> {
        GraphWatcher::next_signal(self)
    }

    fn close(&mut self) -> Result<(), SourceError> {
        GraphWatcher::close(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of a real `pw-dump`, with the identifying content replaced.
    /// Two clients front the same verified pid (the bridge), one connects on
    /// its own, one stream names no process at all, and one node is an input
    /// rather than an output.
    const DUMP: &str = r#"[
      {"id": 34, "type": "PipeWire:Interface:Client",
       "info": {"props": {"pipewire.sec.pid": 900, "application.name": "pipewire"}}},
      {"id": 118, "type": "PipeWire:Interface:Client",
       "info": {"props": {"pipewire.sec.pid": 900, "application.process.id": 4711,
                          "application.name": "Browser"}}},
      {"id": 119, "type": "PipeWire:Interface:Client",
       "info": {"props": {"pipewire.sec.pid": 900, "application.process.id": 4712,
                          "application.name": "Reader"}}},
      {"id": 70, "type": "PipeWire:Interface:Client",
       "info": {"props": {"pipewire.sec.pid": 5150, "application.name": "Shell"}}},
      {"id": 200, "type": "PipeWire:Interface:Node",
       "info": {"props": {"media.class": "Stream/Output/Audio", "client.id": 118,
                          "object.serial": 2374,
                          "application.name": "Browser", "media.name": "playback"}}},
      {"id": 201, "type": "PipeWire:Interface:Node",
       "info": {"props": {"media.class": "Stream/Output/Audio", "client.id": 70,
                          "application.name": "Shell"}}},
      {"id": 202, "type": "PipeWire:Interface:Node",
       "info": {"props": {"media.class": "Stream/Input/Audio", "client.id": 118,
                          "application.name": "Browser"}}},
      {"id": 203, "type": "PipeWire:Interface:Node",
       "info": {"props": {"media.class": "Stream/Output/Audio", "node.name": "orphan"}}}
    ]"#;

    fn stream(streams: &[AudioOutputStream], node_id: u32) -> &AudioOutputStream {
        streams
            .iter()
            .find(|stream| stream.node_id == node_id)
            .unwrap_or_else(|| panic!("node {node_id} missing from {streams:?}"))
    }

    /// RFR-1 — a partial tail is never handed on. The protocol refuses it, and
    /// padding it would move the clock on every frame thereafter.
    #[test]
    fn a_partial_tail_is_held_back() {
        let mut reframer = Reframer::with_frame_bytes(4);
        reframer.push(&[1, 2, 3]);
        assert_eq!(reframer.next_frame(), None);
        assert_eq!(reframer.pending(), 3);
    }

    /// RFR-2 — the boundary case: a chunk that ends exactly on a frame edge
    /// leaves nothing behind. Off-by-one here drifts silently.
    #[test]
    fn a_chunk_ending_on_the_boundary_leaves_nothing() {
        let mut reframer = Reframer::with_frame_bytes(4);
        reframer.push(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(reframer.next_frame(), Some(vec![1, 2, 3, 4]));
        assert_eq!(reframer.next_frame(), Some(vec![5, 6, 7, 8]));
        assert_eq!(reframer.next_frame(), None);
        assert_eq!(reframer.pending(), 0);
    }

    /// RFR-3 — a recorder that trickles bytes still produces whole frames.
    #[test]
    fn trickled_bytes_accumulate_into_a_frame() {
        let mut reframer = Reframer::with_frame_bytes(4);
        for byte in [9_u8, 8, 7] {
            reframer.push(&[byte]);
            assert_eq!(reframer.next_frame(), None);
        }
        reframer.push(&[6]);
        assert_eq!(reframer.next_frame(), Some(vec![9, 8, 7, 6]));
    }

    /// RFR-4 — the shipped frame size is the protocol's, not a local choice.
    #[test]
    fn the_default_frame_is_the_protocol_frame() {
        assert_eq!(SOURCE_FRAME_BYTES, 7680);
        assert_eq!(Reframer::new().pending(), 0);
    }

    /// SRC-ARGS — the measured working invocation, pinned where it can be read.
    /// The serial is what `pw-record --target` wants; the graph object id is a
    /// different number and aiming with it records someone else's stream.
    #[test]
    fn the_frame_and_the_recorder_speak_the_protocols_numbers() {
        // One truth, one definition. This is green today because the numbers
        // happen to agree; it exists to turn red on the day one of them moves
        // and the other does not — the failure that would otherwise be silence.
        assert_eq!(SOURCE_FRAME_BYTES, crate::app::pane_audio::FRAME_BYTES);
        let args = capture_args(1);
        let rate = args.iter().position(|a| a == "--rate").expect("--rate");
        let channels = args
            .iter()
            .position(|a| a == "--channels")
            .expect("--channels");
        assert_eq!(args[rate + 1], SAMPLE_RATE_HZ.to_string());
        assert_eq!(args[channels + 1], CHANNELS.to_string());
    }

    #[test]
    fn the_recorder_is_aimed_with_the_streams_serial() {
        let args = capture_args(2374);
        let joined = args.join(" ");
        assert!(joined.contains("--target 2374"), "{joined}");
        assert!(joined.contains("--rate 48000"), "{joined}");
        assert!(joined.contains("--channels 2"), "{joined}");
        assert!(joined.contains("--format f32"), "{joined}");
        assert_eq!(args.last().map(String::as_str), Some("-"), "{joined}");
    }

    /// PW-1 — the measured failure: a bridged client's verified pid belongs to
    /// the bridge, so the owner has to come from what the program says about
    /// itself.
    #[test]
    fn a_bridged_stream_resolves_to_the_program_not_the_bridge() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        let browser = stream(&streams, 200);
        assert_eq!(browser.pid, Some(4711));
        assert_eq!(browser.pid_trust, Some(PidTrust::SelfReported));
    }

    /// PW-2 — a program that connected on its own keeps its verified pid.
    #[test]
    fn a_direct_client_keeps_its_verified_pid() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        let shell = stream(&streams, 201);
        assert_eq!(shell.pid, Some(5150));
        assert_eq!(shell.pid_trust, Some(PidTrust::Verified));
    }

    /// PW-3 — capture is about what leaves the machine; an input stream is a
    /// microphone and has no business here.
    #[test]
    fn input_streams_are_not_capture_candidates() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        assert!(streams.iter().all(|stream| stream.node_id != 202));
    }

    /// PW-4 — a stream nobody claims still gets listed: the name rule may yet
    /// recognise it, and dropping it here would hide it from every later rule.
    #[test]
    fn a_stream_without_any_pid_is_still_listed() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        let orphan = stream(&streams, 203);
        assert_eq!(orphan.pid, None);
        assert_eq!(orphan.pid_trust, None);
        assert_eq!(orphan.app_name.as_deref(), Some("orphan"));
    }

    /// PW-5 — the browser's name travels with it, because the weakest match
    /// rule has nothing else to work with.
    #[test]
    fn the_producer_name_survives_the_read() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        assert_eq!(stream(&streams, 200).app_name.as_deref(), Some("Browser"));
    }

    /// PW-9 — the capture target is the stream's serial, not the graph's
    /// object id: `pw-record --target` reads the serial, and the two numbers
    /// differ, so carrying only the object id would aim the recorder at
    /// whatever else happens to hold that number.
    #[test]
    fn the_capture_target_is_the_streams_serial() {
        let streams = parse_output_streams(DUMP).expect("dump parses");
        assert_eq!(stream(&streams, 200).object_serial, Some(2374));
        assert_eq!(stream(&streams, 203).object_serial, None);
    }

    /// PW-6 — an unreadable graph is an error to report, never a panic in a
    /// server loop, and never an empty list that reads as silence.
    #[test]
    fn a_malformed_dump_is_an_error_not_a_panic() {
        assert!(parse_output_streams("{not json").is_err());
    }

    /// PW-7 — an empty graph is silence, which is legal.
    #[test]
    fn an_empty_graph_is_no_streams() {
        assert_eq!(
            parse_output_streams("[]").expect("empty parses"),
            Vec::new()
        );
    }

    /// PW-8 — versions differ on whether a prop is a number or a string; both
    /// have to read the same.
    #[test]
    fn props_are_read_as_numbers_or_strings() {
        let dump = r#"[
          {"id": 5, "type": "PipeWire:Interface:Client",
           "info": {"props": {"pipewire.sec.pid": "77", "application.name": "Text"}}},
          {"id": 6, "type": "PipeWire:Interface:Node",
           "info": {"props": {"media.class": "Stream/Output/Audio", "client.id": "5"}}}
        ]"#;
        let streams = parse_output_streams(dump).expect("dump parses");
        assert_eq!(stream(&streams, 6).pid, Some(77));
    }

    // ── A4a · ExternalSource (SRC-1..SRC-8) ───────────────────────────────
    //
    // Every process spawned below is a POSIX utility, so the same assertions
    // run unchanged on a build box with no sound server at all. A test that
    // needed `pw-record` would be a test about the machine.

    /// A source that emits exactly `bytes` zero bytes and then ends.
    fn one_shot(bytes: usize) -> ExternalSource {
        ExternalSource::spawn(
            "head",
            &["-c".to_string(), bytes.to_string(), "/dev/zero".to_string()],
        )
        .expect("head starts")
    }

    #[test]
    fn a_recorder_that_cannot_start_is_unavailable_rather_than_a_panic() {
        let err =
            ExternalSource::spawn("herdr-no-such-recorder", &["-"]).expect_err("no such program");
        assert!(matches!(err, SourceError::Unavailable(_)), "{err}");
    }

    #[test]
    fn a_recorder_that_ends_without_bytes_reads_as_end_of_stream() {
        let mut source = ExternalSource::spawn("true", &[] as &[&str]).expect("true starts");
        assert!(source
            .next_frame()
            .expect("end of stream is not an error")
            .is_none());
    }

    #[test]
    fn closing_leaves_no_recorder_behind() {
        let mut source = ExternalSource::spawn("sleep", &["30"]).expect("sleep starts");
        source.close().expect("close");
        assert!(source.exited(), "the recorder outlived its close");
    }

    #[test]
    fn closing_twice_is_not_an_error() {
        let mut source = ExternalSource::spawn("sleep", &["30"]).expect("sleep starts");
        source.close().expect("first close");
        source.close().expect("second close");
    }

    #[test]
    fn one_whole_frame_arrives_and_then_the_stream_ends() {
        let mut source = one_shot(SOURCE_FRAME_BYTES);
        let frame = source.next_frame().expect("read").expect("a whole frame");
        assert_eq!(frame.len(), SOURCE_FRAME_BYTES);
        assert!(source.next_frame().expect("read").is_none());
    }

    #[test]
    fn a_partial_frame_is_never_handed_on() {
        let mut source = one_shot(100);
        assert!(source.next_frame().expect("read").is_none());
    }

    #[test]
    fn frames_keep_coming_until_the_recorder_ends() {
        let mut source = one_shot(SOURCE_FRAME_BYTES * 2);
        assert!(source.next_frame().expect("read").is_some());
        assert!(source.next_frame().expect("read").is_some());
        assert!(source.next_frame().expect("read").is_none());
    }

    #[test]
    fn aiming_the_capture_needs_the_recorder_but_never_panics() {
        match ExternalSource::capture(2374) {
            Ok(mut source) => {
                assert_eq!(source.program(), RECORDER);
                source.close().expect("close");
            }
            Err(err) => assert!(matches!(err, SourceError::Unavailable(_)), "{err}"),
        }
    }

    // ── A4c · GraphWatcher (WCH-1, WCH-5, WCH-6) ──────────────────────────
    //
    // WCH-2..WCH-4 live with the `Debouncer` in `app::pane_audio_source`: the
    // timing rule is pure and belongs where it can be tested without a process.

    #[test]
    fn output_in_an_unknown_shape_is_still_a_signal() {
        // The point of not parsing: whatever pw-mon prints, in whatever version's
        // format, means the same one thing here.
        let mut watcher = GraphWatcher::spawn(
            "printf",
            // printf turns these escapes into real bytes, so the pipe carries
            // something no parser would accept — which is the point.
            &[r"}}not json at all{{ ÿþ binary too".to_string()],
        )
        .expect("printf starts");
        assert!(watcher.next_signal().expect("read"));
    }

    #[test]
    fn a_watcher_that_ends_stops_signalling() {
        let mut watcher = GraphWatcher::spawn("true", &[] as &[&str]).expect("true starts");
        assert!(!watcher
            .next_signal()
            .expect("end of stream is not an error"));
    }

    #[test]
    fn closing_leaves_no_watcher_behind() {
        let mut watcher = GraphWatcher::spawn("sleep", &["30"]).expect("sleep starts");
        watcher.close().expect("close");
        assert!(watcher.exited(), "the watcher outlived its close");
    }

    #[test]
    fn a_watcher_that_cannot_start_is_unavailable_rather_than_a_panic() {
        let err =
            GraphWatcher::spawn("herdr-no-such-watcher", &["-"]).expect_err("no such program");
        assert!(matches!(err, SourceError::Unavailable(_)), "{err}");
    }

    #[test]
    fn starting_the_watcher_needs_the_program_but_never_panics() {
        match GraphWatcher::start() {
            Ok(mut watcher) => {
                assert_eq!(watcher.program(), WATCHER);
                watcher.close().expect("close");
            }
            Err(err) => assert!(matches!(err, SourceError::Unavailable(_)), "{err}"),
        }
    }

    // ── A4d-1 · platform seam (SEAM-1..SEAM-3) ────────────────────────────

    #[test]
    fn the_seam_hands_back_a_source_on_this_platform() {
        // A seam that returned None here would make the feature compile,
        // pass every test, and produce silence.
        let handed = crate::platform::capture_stream(1);
        assert!(handed.is_some(), "linux must offer a capture source");
        if let Some(Ok(mut source)) = handed {
            source.close().expect("close");
        }
    }

    #[test]
    fn the_seam_hands_back_a_watcher_on_this_platform() {
        let handed = crate::platform::start_graph_watcher();
        assert!(handed.is_some(), "linux must offer a graph watcher");
        if let Some(Ok(mut watcher)) = handed {
            watcher.close().expect("close");
        }
    }

    #[test]
    fn reading_the_graph_without_the_reader_is_an_error_not_a_panic() {
        // Either the machine has pw-dump and answers, or it does not and says
        // so. Both are results; neither is a crash in the server loop.
        match read_output_streams() {
            Ok(streams) => {
                for stream in &streams {
                    assert!(stream.node_id > 0, "a graph node needs an id");
                }
            }
            Err(err) => assert!(
                matches!(err, SourceError::Unavailable(_) | SourceError::Closed(_)),
                "{err}"
            ),
        }
    }
}
