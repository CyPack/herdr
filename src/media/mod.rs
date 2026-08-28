//! Media encoding for pane audio streams.
//!
//! The codec lives behind this seam rather than being called directly, for a
//! reason the measurement made concrete: the chosen crate is a young pure-Rust
//! port, and the fallback — libopus bindings — brings a C toolchain the build
//! box cannot satisfy. Whichever way that trade eventually settles, only this
//! module should have to change.
//!
//! Unconditional, not behind `media-sink`. The flag exists to keep an audio
//! *device* out of the default binary, which is what `sound.rs` decided to
//! avoid; encoding needs no device, and a server that cannot encode depending
//! on how it was built would make the capability handshake answer differently
//! for reasons the client can never see.

pub mod clock;
pub mod opus;

/// Sample rate every audio stream uses.
///
/// Opus resamples internally to 48 kHz whatever it is given, so choosing
/// anything else only adds a conversion nobody asked for.
pub const SAMPLE_RATE_HZ: u32 = 48_000;

/// Channels per audio stream.
pub const CHANNELS: u8 = 2;

/// Frame length in milliseconds.
///
/// 20 ms is Opus's default and the size the design specifies: short enough that
/// one lost packet is a concealable gap, long enough that per-packet overhead
/// stays small.
pub const FRAME_MS: u32 = 20;

/// Samples **per channel** in one frame: 48000 × 20 / 1000.
pub const FRAME_SAMPLES: usize = (SAMPLE_RATE_HZ as usize * FRAME_MS as usize) / 1000;

/// Default bitrate.
///
/// 64 kbps measured at 0.34 MB per minute — a thirty-second of raw PCM, and
/// small enough to share the existing control channel without a second
/// transport (canonical design §7.5).
pub const DEFAULT_BITRATE_BPS: i32 = 64_000;

/// Lowest and highest bitrate a stream may be configured with.
pub const MIN_BITRATE_BPS: i32 = 32_000;
pub const MAX_BITRATE_BPS: i32 = 128_000;

/// Anything the codec layer can refuse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaError {
    /// The codec itself refused. Carries whatever it said.
    Codec(String),
    /// A frame arrived with the wrong number of samples.
    ///
    /// Its own variant rather than a codec error because it is a caller
    /// mistake, and because a frame that is quietly padded or truncated makes
    /// audio drift by exactly that much per frame — inaudible once, obvious
    /// after a minute, and impossible to trace back.
    FrameSize { expected: usize, got: usize },
    /// The output buffer was too small for the encoded packet.
    OutputTooSmall { needed: usize, got: usize },
    /// A bitrate outside the supported range.
    Bitrate { asked: i32 },
}

impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Codec(message) => write!(f, "audio codec refused: {message}"),
            Self::FrameSize { expected, got } => {
                write!(f, "audio frame has {got} samples, expected {expected}")
            }
            Self::OutputTooSmall { needed, got } => {
                write!(f, "encoded packet needs {needed} bytes, buffer has {got}")
            }
            Self::Bitrate { asked } => write!(
                f,
                "bitrate {asked} is outside {MIN_BITRATE_BPS}..={MAX_BITRATE_BPS}"
            ),
        }
    }
}

impl std::error::Error for MediaError {}
