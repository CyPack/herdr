//! Opus encode and decode, one frame at a time.

use super::{
    MediaError, CHANNELS, DEFAULT_BITRATE_BPS, FRAME_SAMPLES, MAX_BITRATE_BPS, MIN_BITRATE_BPS,
    SAMPLE_RATE_HZ,
};

/// Encodes one 20 ms stereo frame at a time.
///
/// Held by value rather than shared: Opus encoders carry per-stream state, so
/// two streams sharing one encoder would interleave their prediction history
/// and produce audio that decodes without error and sounds wrong.
pub struct AudioEncoder {
    inner: opus_rs::OpusEncoder,
    bitrate_bps: i32,
}

// Written by hand because the codec's own types are not Debug, and because
// what a reader of a failing test needs is the configuration, not the
// encoder's internal prediction state.
impl std::fmt::Debug for AudioEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioEncoder")
            .field("bitrate_bps", &self.bitrate_bps)
            .field("sample_rate_hz", &SAMPLE_RATE_HZ)
            .field("channels", &CHANNELS)
            .finish()
    }
}

impl AudioEncoder {
    /// Builds an encoder for the fixed stream shape at `bitrate_bps`.
    pub fn new(bitrate_bps: i32) -> Result<Self, MediaError> {
        if !(MIN_BITRATE_BPS..=MAX_BITRATE_BPS).contains(&bitrate_bps) {
            return Err(MediaError::Bitrate { asked: bitrate_bps });
        }
        let mut inner = opus_rs::OpusEncoder::new(
            SAMPLE_RATE_HZ as i32,
            CHANNELS as usize,
            // Audio, not Voip: a pane may be playing music as easily as
            // speech, and the voice profile spends its bits on intelligibility
            // in a way that is audible on anything else.
            opus_rs::Application::Audio,
        )
        .map_err(|err| MediaError::Codec(err.to_string()))?;
        inner.bitrate_bps = bitrate_bps;
        // Constant bitrate, and the reason is the shared channel rather than
        // audio quality. Measured with ffmpeg 8.1.2 at 64 kbps, 48 kHz stereo,
        // 20 ms frames: VBR spends 5850 B/s on pink noise but 10643 B/s on a
        // pure tone — above its own nominal rate — while CBR holds 8109 B/s on
        // both. A lane that shares a link with keystrokes needs a budget that
        // is knowable in advance, not one that doubles when the content
        // happens to be tonal.
        inner.use_cbr = true;
        Ok(Self { inner, bitrate_bps })
    }

    /// Encodes exactly one frame of interleaved samples into `out`.
    ///
    /// Returns how many bytes of `out` the packet occupies.
    pub fn encode(&mut self, pcm: &[f32], out: &mut [u8]) -> Result<usize, MediaError> {
        let expected = FRAME_SAMPLES * CHANNELS as usize;
        if pcm.len() != expected {
            return Err(MediaError::FrameSize {
                expected,
                got: pcm.len(),
            });
        }
        if out.len() < MIN_PACKET_BUFFER {
            return Err(MediaError::OutputTooSmall {
                needed: MIN_PACKET_BUFFER,
                got: out.len(),
            });
        }
        self.inner
            .encode(pcm, FRAME_SAMPLES, out)
            .map_err(|err| MediaError::Codec(err.to_string()))
    }
}

/// Decodes one packet at a time back into interleaved samples.
pub struct AudioDecoder {
    inner: opus_rs::OpusDecoder,
}

impl std::fmt::Debug for AudioDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioDecoder")
            .field("sample_rate_hz", &SAMPLE_RATE_HZ)
            .field("channels", &CHANNELS)
            .finish()
    }
}

impl AudioDecoder {
    pub fn new() -> Result<Self, MediaError> {
        let inner = opus_rs::OpusDecoder::new(SAMPLE_RATE_HZ as i32, CHANNELS as usize)
            .map_err(|err| MediaError::Codec(err.to_string()))?;
        Ok(Self { inner })
    }

    /// Decodes one packet into `pcm`, returning samples **per channel**.
    ///
    /// The count is per channel rather than total because that is what the
    /// playout clock counts in: a frame is 960 samples at 48 kHz whether it is
    /// mono or stereo, and converting in two places is how the two drift.
    pub fn decode(&mut self, packet: &[u8], pcm: &mut [f32]) -> Result<usize, MediaError> {
        let expected = FRAME_SAMPLES * CHANNELS as usize;
        if pcm.len() < expected {
            return Err(MediaError::OutputTooSmall {
                needed: expected,
                got: pcm.len(),
            });
        }
        self.inner
            .decode(packet, FRAME_SAMPLES, pcm)
            .map_err(|err| MediaError::Codec(err.to_string()))
    }
}

/// Smallest output buffer the encoder will write into.
///
/// The Opus RFC's own recommendation for a packet buffer. Refusing anything
/// smaller here is what turns a truncated packet — which decodes into noise
/// rather than into less audio — into an error the caller can see.
const MIN_PACKET_BUFFER: usize = 4000;

#[cfg(test)]
mod tests {
    use super::*;

    /// One 20 ms frame of a 440 Hz tone, interleaved stereo.
    ///
    /// A tone rather than silence or noise: silence encodes to almost nothing
    /// and would make the bitrate assertion meaningless, and noise is the
    /// worst case rather than a representative one.
    fn tone_frame() -> Vec<f32> {
        let mut pcm = Vec::with_capacity(FRAME_SAMPLES * CHANNELS as usize);
        for sample in 0..FRAME_SAMPLES {
            let t = sample as f32 / SAMPLE_RATE_HZ as f32;
            let value = (t * 440.0 * std::f32::consts::TAU).sin() * 0.5;
            for _ in 0..CHANNELS {
                pcm.push(value);
            }
        }
        pcm
    }

    // TP-MEDIA-CODEC-01
    #[test]
    fn a_frame_survives_encoding_with_its_sample_count_intact() {
        // Not "the audio sounds the same" — Opus is lossy and that assertion
        // could not exist. The property that matters is that a frame in is a
        // frame out: a decoder that returns a different number of samples
        // shifts every later frame by that much, and the drift accumulates
        // silently until it is a second and someone notices the audio is
        // behind the picture.
        let mut encoder = AudioEncoder::new(DEFAULT_BITRATE_BPS).expect("encoder");
        let mut decoder = AudioDecoder::new().expect("decoder");

        let pcm = tone_frame();
        let mut packet = vec![0u8; 4000];
        let written = encoder.encode(&pcm, &mut packet).expect("encode");
        assert!(written > 0, "an encoded frame cannot be empty");

        let mut out = vec![0f32; FRAME_SAMPLES * CHANNELS as usize];
        let samples = decoder
            .decode(&packet[..written], &mut out)
            .expect("decode");

        assert_eq!(
            samples, FRAME_SAMPLES,
            "the decoder must return one frame's worth of samples per channel"
        );
    }

    // TP-MEDIA-CODEC-01
    #[test]
    fn a_wrong_sized_frame_is_refused_rather_than_padded() {
        // The failure this prevents is the quiet one: pad a short frame and
        // the stream gains silence every time; truncate a long one and it
        // loses audio. Either way the clock and the content disagree by a
        // fixed amount per frame, which is drift with no source to trace.
        let mut encoder = AudioEncoder::new(DEFAULT_BITRATE_BPS).expect("encoder");
        let mut packet = vec![0u8; 4000];

        let short = vec![0f32; FRAME_SAMPLES]; // one channel's worth, not two
        assert_eq!(
            encoder.encode(&short, &mut packet),
            Err(MediaError::FrameSize {
                expected: FRAME_SAMPLES * CHANNELS as usize,
                got: FRAME_SAMPLES,
            })
        );
    }

    // TP-MEDIA-CODEC-01
    #[test]
    fn a_bitrate_outside_the_supported_range_is_refused_at_construction() {
        // Refused where it is asked for, not clamped: a stream that silently
        // runs at a different bitrate than the one configured makes every
        // later bandwidth measurement wrong, including the ones this phase's
        // acceptance criteria rest on.
        assert_eq!(
            AudioEncoder::new(MIN_BITRATE_BPS - 1).unwrap_err(),
            MediaError::Bitrate {
                asked: MIN_BITRATE_BPS - 1
            }
        );
        assert_eq!(
            AudioEncoder::new(MAX_BITRATE_BPS + 1).unwrap_err(),
            MediaError::Bitrate {
                asked: MAX_BITRATE_BPS + 1
            }
        );
        assert!(AudioEncoder::new(MIN_BITRATE_BPS).is_ok());
        assert!(AudioEncoder::new(MAX_BITRATE_BPS).is_ok());
    }

    // TP-MEDIA-CODEC-01
    #[test]
    fn a_second_of_audio_at_the_default_bitrate_costs_what_the_design_measured() {
        // Anchored to an independent measurement rather than to arithmetic:
        // ffmpeg 8.1.2 encoding the same shape (64 kbps, 48 kHz stereo, 20 ms
        // frames, CBR) with libopus produces 8109 B/s, against the nominal
        // 8000. This test asserts our crate lands in the same place.
        //
        // The canonical design's 0.34 MB/min figure is VBR — reproduced here
        // at 0.335 MB/min on pink noise — and VBR on a pure tone runs to
        // 0.609 MB/min, above its own nominal rate. That is why the encoder
        // asks for CBR and why this assertion can be tight at all.
        //
        // A wrong bitrate or a wrong frame size is invisible to a roundtrip
        // test: the audio decodes perfectly, it just no longer fits the
        // channel it was sized for.
        let mut encoder = AudioEncoder::new(DEFAULT_BITRATE_BPS).expect("encoder");
        let pcm = tone_frame();
        let mut packet = vec![0u8; 4000];

        let frames_per_second = 1000 / super::super::FRAME_MS as usize;
        let mut total = 0usize;
        for _ in 0..frames_per_second {
            total += encoder.encode(&pcm, &mut packet).expect("encode");
        }

        let expected = DEFAULT_BITRATE_BPS as usize / 8; // 8000 bytes
        let low = expected * 85 / 100;
        let high = expected * 115 / 100;
        assert!(
            (low..=high).contains(&total),
            "a second of 64 kbps audio came to {total} bytes, outside {low}..={high}"
        );
    }

    // TP-MEDIA-CODEC-01
    #[test]
    fn a_too_small_output_buffer_is_refused_not_silently_truncated() {
        // A truncated Opus packet is not a quieter packet; it is a packet the
        // decoder rejects or, worse, decodes into noise. The encoder has to
        // say so rather than report how much it managed to write.
        let mut encoder = AudioEncoder::new(MAX_BITRATE_BPS).expect("encoder");
        let pcm = tone_frame();
        let mut tiny = [0u8; 2];
        assert!(matches!(
            encoder.encode(&pcm, &mut tiny),
            Err(MediaError::OutputTooSmall { .. }) | Err(MediaError::Codec(_))
        ));
    }
}
