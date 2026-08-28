//! Opus encode and decode, one frame at a time.

use super::{
    MediaError, CHANNELS, DEFAULT_BITRATE_BPS, FRAME_SAMPLES, MAX_BITRATE_BPS, MIN_BITRATE_BPS,
    SAMPLE_RATE_HZ,
};

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

    // TP-MEDIA-OPUS-01
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

    // TP-MEDIA-OPUS-01
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

    // TP-MEDIA-OPUS-01
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

    // TP-MEDIA-BITRATE-01
    #[test]
    fn a_second_of_audio_at_the_default_bitrate_costs_what_the_design_measured() {
        // The canonical design measured Opus 64 kbps at 0.34 MB per minute
        // with ffmpeg — 5.7 KB per second. If this crate produces a materially
        // different size, then either the bitrate is not being applied or the
        // frame size is wrong, and both are configuration mistakes that no
        // roundtrip test can see: the audio still decodes perfectly, it just
        // does not fit the channel it was sized for.
        let mut encoder = AudioEncoder::new(DEFAULT_BITRATE_BPS).expect("encoder");
        let pcm = tone_frame();
        let mut packet = vec![0u8; 4000];

        let frames_per_second = 1000 / super::super::FRAME_MS as usize;
        let mut total = 0usize;
        for _ in 0..frames_per_second {
            total += encoder.encode(&pcm, &mut packet).expect("encode");
        }

        let expected = DEFAULT_BITRATE_BPS as usize / 8; // 8000 bytes
        let low = expected * 70 / 100;
        let high = expected * 130 / 100;
        assert!(
            (low..=high).contains(&total),
            "a second of 64 kbps audio came to {total} bytes, outside {low}..={high}"
        );
    }

    // TP-MEDIA-OPUS-01
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
