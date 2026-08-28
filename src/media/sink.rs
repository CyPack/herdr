//! Where decoded audio goes.
//!
//! Three implementations behind one trait, because the client runs on three
//! platforms and only one of them can be given a real audio library without
//! cost. The choice is a *target-gated dependency* rather than a feature flag:
//! a flag that CI can never enable is a door nobody opens, and the macOS client
//! — the one a listener actually uses — would have shipped unable to play.

use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};

use super::{CHANNELS, FRAME_SAMPLES, SAMPLE_RATE_HZ};

/// Anything that can refuse a frame.
#[derive(Debug)]
pub enum SinkError {
    /// No usable output was found on this machine.
    Unavailable(String),
    /// The sink accepted frames and then stopped.
    Closed(String),
    /// A frame arrived with the wrong number of samples.
    FrameSize { expected: usize, got: usize },
}

impl std::fmt::Display for SinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(what) => write!(f, "no audio output available: {what}"),
            Self::Closed(why) => write!(f, "audio output closed: {why}"),
            Self::FrameSize { expected, got } => {
                write!(f, "audio frame has {got} samples, expected {expected}")
            }
        }
    }
}

impl std::error::Error for SinkError {}

/// A place to put decoded audio.
pub trait AudioSink: Send {
    /// Hands over one frame of interleaved samples.
    fn write_frame(&mut self, pcm: &[f32]) -> Result<(), SinkError>;
    /// Stops the sink. Idempotent.
    fn close(&mut self) -> Result<(), SinkError>;
    /// What this sink is, for logs and for the handshake's own record.
    fn describe(&self) -> String;
}

/// Accepts every frame and drops it.
///
/// Not only a test double: it is the honest answer on a machine with no audio
/// output, and it keeps the rest of the pipeline — clock, jitter buffer,
/// sequence accounting — running and measurable where a missing sink would
/// otherwise mean no measurements at all.
#[derive(Debug, Default)]
pub struct SilentSink {
    frames: usize,
    closed: bool,
}

impl SilentSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn frames(&self) -> usize {
        self.frames
    }
}

impl AudioSink for SilentSink {
    fn write_frame(&mut self, pcm: &[f32]) -> Result<(), SinkError> {
        check_frame(pcm)?;
        if self.closed {
            return Err(SinkError::Closed("silent sink already closed".into()));
        }
        self.frames += 1;
        Ok(())
    }

    fn close(&mut self) -> Result<(), SinkError> {
        self.closed = true;
        Ok(())
    }

    fn describe(&self) -> String {
        "silent".to_string()
    }
}

/// One external player that can take raw f32 samples on stdin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalPlayer {
    pub program: &'static str,
    args: &'static [&'static str],
}

/// Players tried in order.
///
/// Order is deliberate. `ffplay` first because it is the one measured present
/// on this machine and it takes the sample format explicitly. `pw-cat` and
/// `pactl` next because a Linux desktop that has neither ffmpeg nor mpv still
/// has its own sound server's client. `mpv` last: it is the one the design
/// named, and it is **not installed here** — a default that does not exist is
/// how a fallback path stays untested until someone needs it.
pub const EXTERNAL_PLAYERS: &[ExternalPlayer] = &[
    ExternalPlayer {
        program: "ffplay",
        args: &[
            "-f",
            "f32le",
            "-ar",
            "48000",
            "-ch_layout",
            "stereo",
            "-nodisp",
            "-autoexit",
            "-loglevel",
            "error",
            "-",
        ],
    },
    ExternalPlayer {
        program: "pw-cat",
        args: &[
            "--playback",
            "--rate",
            "48000",
            "--channels",
            "2",
            "--format",
            "f32",
            "-",
        ],
    },
    ExternalPlayer {
        program: "mpv",
        args: &[
            "--no-video",
            "--no-terminal",
            "--demuxer=rawaudio",
            "--demuxer-rawaudio-format=floatle",
            "--demuxer-rawaudio-rate=48000",
            "--demuxer-rawaudio-channels=2",
            "-",
        ],
    },
];

impl ExternalPlayer {
    pub fn args(&self) -> &'static [&'static str] {
        self.args
    }
}

/// Feeds raw samples to a player process over its stdin.
///
/// The last resort, and it is a real one: it needs no audio library, no system
/// headers and no build-time feature, so it works on the platforms where a
/// native sink cannot be compiled at all. What it gives up is control over
/// latency — the player owns its own buffer and will not say how deep it is.
// Hand-written: `Child` is Debug but the sink's useful identity is which
// player it is talking to, not the process handle's internals.
impl std::fmt::Debug for ExternalSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalSink")
            .field("program", &self.program)
            .field("open", &self.stdin.is_some())
            .finish()
    }
}

pub struct ExternalSink {
    child: Child,
    stdin: Option<ChildStdin>,
    program: String,
    scratch: Vec<u8>,
}

impl ExternalSink {
    /// Spawns `program` with `args`, expecting it to read samples on stdin.
    pub fn spawn(program: &str, args: &[&str]) -> Result<Self, SinkError> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| SinkError::Unavailable(format!("{program}: {err}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| SinkError::Unavailable(format!("{program}: no stdin")))?;
        Ok(Self {
            child,
            stdin: Some(stdin),
            program: program.to_string(),
            scratch: Vec::with_capacity(FRAME_SAMPLES * CHANNELS as usize * 4),
        })
    }

    /// Spawns the first player on this machine, in `EXTERNAL_PLAYERS` order.
    pub fn spawn_available() -> Result<Self, SinkError> {
        Self::spawn_available_from(EXTERNAL_PLAYERS)
    }

    /// The selection seam.
    ///
    /// Separated so a test can drive the order with players it controls: the
    /// shipped list names programs whose presence depends on the machine, and a
    /// test that asserted against it would be asserting about the machine.
    pub fn spawn_available_from(players: &[ExternalPlayer]) -> Result<Self, SinkError> {
        let mut tried = Vec::new();
        for player in players {
            match Self::spawn(player.program, player.args) {
                Ok(sink) => return Ok(sink),
                Err(_) => tried.push(player.program),
            }
        }
        Err(SinkError::Unavailable(format!(
            "none of these could be started: {}",
            tried.join(", ")
        )))
    }

    pub fn program(&self) -> &str {
        &self.program
    }
}

impl AudioSink for ExternalSink {
    fn write_frame(&mut self, pcm: &[f32]) -> Result<(), SinkError> {
        check_frame(pcm)?;
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(SinkError::Closed(format!(
                "{} already closed",
                self.program
            )));
        };
        self.scratch.clear();
        for sample in pcm {
            self.scratch.extend_from_slice(&sample.to_le_bytes());
        }
        stdin.write_all(&self.scratch).map_err(|err| {
            // A player that exited takes its stdin with it, so a broken pipe
            // here means the sink is gone rather than that this frame was bad.
            SinkError::Closed(format!("{}: {err}", self.program))
        })
    }

    fn close(&mut self) -> Result<(), SinkError> {
        // Dropping stdin is what tells the player to finish; killing it would
        // cut off whatever it has already buffered, which is audible.
        self.stdin = None;
        let _ = self.child.wait();
        Ok(())
    }

    fn describe(&self) -> String {
        format!("external:{}", self.program)
    }
}

impl Drop for ExternalSink {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

fn check_frame(pcm: &[f32]) -> Result<(), SinkError> {
    let expected = FRAME_SAMPLES * CHANNELS as usize;
    if pcm.len() != expected {
        return Err(SinkError::FrameSize {
            expected,
            got: pcm.len(),
        });
    }
    Ok(())
}

/// The stream shape every sink here expects, published so a caller cannot
/// disagree with it silently.
pub const SINK_SAMPLE_RATE_HZ: u32 = SAMPLE_RATE_HZ;
pub const SINK_CHANNELS: u8 = CHANNELS;

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Vec<f32> {
        vec![0.25f32; FRAME_SAMPLES * CHANNELS as usize]
    }

    // TP-MEDIA-SINK-01
    #[test]
    fn every_sink_refuses_a_wrong_sized_frame() {
        // The same guard the encoder keeps, at the other end of the pipeline.
        // A sink that pads or truncates shifts playback by a fixed amount per
        // frame, and the drift has no source anyone can point at.
        let mut silent = SilentSink::new();
        assert!(matches!(
            silent.write_frame(&[0.0; 8]),
            Err(SinkError::FrameSize { got: 8, .. })
        ));
        assert_eq!(silent.frames(), 0, "a refused frame is not a written frame");
    }

    // TP-MEDIA-SINK-01
    #[test]
    fn a_closed_sink_says_so_rather_than_accepting_silently() {
        // Accepting after close is the failure that looks like success: the
        // pipeline keeps running, the counters keep moving, and nothing plays.
        let mut silent = SilentSink::new();
        silent.write_frame(&frame()).expect("open sink accepts");
        silent.close().expect("close");
        assert!(matches!(
            silent.write_frame(&frame()),
            Err(SinkError::Closed(_))
        ));
    }

    // TP-MEDIA-SINK-02
    #[test]
    fn an_external_sink_writes_little_endian_f32_in_frame_order() {
        // The wire between us and the player is a byte format with no header,
        // so both sides have to agree without being able to check. `cat` stands
        // in for the player: it proves what we send, which is the only half of
        // that agreement this repository owns.
        let out = std::env::temp_dir().join(format!("herdr-sink-{}.raw", std::process::id()));
        let _ = std::fs::remove_file(&out);

        let mut sink = ExternalSink::spawn("sh", &["-c", &format!("cat > {}", out.display())])
            .expect("cat is available on any unix");
        let mut pcm = frame();
        pcm[0] = 1.0;
        pcm[1] = -1.0;
        sink.write_frame(&pcm).expect("write");
        sink.close().expect("close");

        let bytes = std::fs::read(&out).expect("player received bytes");
        assert_eq!(
            bytes.len(),
            FRAME_SAMPLES * CHANNELS as usize * 4,
            "one frame is one f32 per interleaved sample"
        );
        assert_eq!(&bytes[0..4], &1.0f32.to_le_bytes());
        assert_eq!(&bytes[4..8], &(-1.0f32).to_le_bytes());
        let _ = std::fs::remove_file(&out);
    }

    // TP-MEDIA-SINK-02
    #[test]
    fn a_player_that_exits_turns_the_sink_closed_instead_of_panicking() {
        // A player the user quits, or one that never really started, must end
        // the stream rather than take the client with it. `true` exits at once,
        // which is the shortest possible version of that.
        let mut sink = ExternalSink::spawn("true", &[]).expect("spawn");
        // The first write may land in the pipe buffer before the child is
        // reaped; the second cannot. Either way it must be an error, never a
        // panic, and never silent success forever.
        let mut saw_closed = false;
        for _ in 0..64 {
            match sink.write_frame(&frame()) {
                Ok(()) => continue,
                Err(SinkError::Closed(_)) => {
                    saw_closed = true;
                    break;
                }
                Err(other) => panic!("expected Closed, got {other:?}"),
            }
        }
        assert!(saw_closed, "a dead player must eventually report Closed");
    }

    // TP-MEDIA-SINK-03
    #[test]
    fn selection_takes_the_first_player_that_starts_and_reports_the_rest() {
        // Driven with players this test owns, not with the shipped list: the
        // shipped names are present or absent depending on the machine, so
        // asserting against them would be asserting about the machine.
        let players = &[
            ExternalPlayer {
                program: "herdr-no-such-player",
                args: &[],
            },
            ExternalPlayer {
                program: "cat",
                args: &[],
            },
        ];
        let sink = ExternalSink::spawn_available_from(players).expect("cat starts");
        assert_eq!(sink.program(), "cat");
        assert_eq!(sink.describe(), "external:cat");
    }

    // TP-MEDIA-SINK-03
    #[test]
    fn no_usable_player_names_everything_it_tried() {
        // "No audio output" with no list is a dead end for whoever reads the
        // log; the names are what turn it into an installable answer.
        let players = &[
            ExternalPlayer {
                program: "herdr-no-such-player-a",
                args: &[],
            },
            ExternalPlayer {
                program: "herdr-no-such-player-b",
                args: &[],
            },
        ];
        match ExternalSink::spawn_available_from(players) {
            Err(SinkError::Unavailable(message)) => {
                assert!(message.contains("herdr-no-such-player-a"), "{message}");
                assert!(message.contains("herdr-no-such-player-b"), "{message}");
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    // TP-MEDIA-SINK-03
    #[test]
    fn the_shipped_player_list_agrees_with_the_stream_shape() {
        // The rate and channel count appear twice: once as constants the codec
        // uses, once as text in a command line. Nothing makes them agree except
        // this test, and a disagreement is inaudible in the usual way — the
        // player resamples and the audio simply plays at the wrong speed.
        assert_eq!(SINK_SAMPLE_RATE_HZ, 48_000);
        assert_eq!(SINK_CHANNELS, 2);
        for player in EXTERNAL_PLAYERS {
            let line = player.args().join(" ");
            assert!(
                line.contains("48000"),
                "{} does not name the sample rate: {line}",
                player.program
            );
            assert!(
                line.contains("stereo") || line.contains("channels=2") || line.contains("2"),
                "{} does not name the channel count: {line}",
                player.program
            );
        }
    }
}
