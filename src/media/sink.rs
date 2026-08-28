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
