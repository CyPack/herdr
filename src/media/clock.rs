//! Clock offset, jitter estimation, and the playout deadline.
//!
//! Three small pieces that together answer one question: *when* should a chunk
//! be played. Playing it on arrival is the failure mode with a name — every
//! wobble in the network becomes a wobble in the audio — so a chunk waits until
//! its presentation time has arrived in the *server's* clock, which means the
//! client has to know what the server's clock says.

use std::collections::VecDeque;

/// How many offset samples the median is taken over.
///
/// Odd, so the median is a real sample rather than an average of two. Nine
/// covers roughly a minute of probing at the rate a session will use without
/// holding measurements long enough to be stale on their own.
const OFFSET_SAMPLES: usize = 9;

/// After this long without a probe the stored offset is thrown away.
///
/// Not a tuning constant: a laptop that suspends comes back with a clock that
/// jumped, and a median built from before the jump outlives the jump by as
/// many probes as it holds. snapcast clears at a minute for the same reason,
/// and this fork has a dormancy feature that makes long gaps ordinary rather
/// than exceptional.
const STALE_AFTER_US: u64 = 60_000_000;

/// Tracks the offset between the client's clock and the server's.
#[derive(Debug, Default)]
pub struct ClockSync {
    samples: VecDeque<i64>,
    last_probe_us: Option<u64>,
    offset_us: i64,
}

impl ClockSync {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one completed probe.
    ///
    /// The four timestamps are NTP's: `t1` when the client sent, `t2` when the
    /// server received, `t3` when the server replied, `t4` when the client got
    /// the reply. Two stamps would give the round trip; four separate the trip
    /// from the offset, which is the only reason the reply carries both of the
    /// server's.
    pub fn observe(
        &mut self,
        t1_client_send: u64,
        t2_server_recv: u64,
        t3_server_send: u64,
        t4_client_recv: u64,
    ) {
        if let Some(last) = self.last_probe_us {
            if t4_client_recv.saturating_sub(last) > STALE_AFTER_US {
                // Everything before a gap this long describes a clock that may
                // no longer exist. Keeping it would average across a jump.
                self.samples.clear();
            }
        }
        self.last_probe_us = Some(t4_client_recv);

        let outbound = t2_server_recv as i64 - t1_client_send as i64;
        let inbound = t4_client_recv as i64 - t3_server_send as i64;
        let offset = (outbound - inbound) / 2;

        if self.samples.len() == OFFSET_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(offset);
        self.offset_us = median(self.samples.iter().copied());
    }

    /// Microseconds to add to a client timestamp to get the server's.
    pub fn offset_us(&self) -> i64 {
        self.offset_us
    }

    /// Whether any probe has landed yet.
    pub fn is_synced(&self) -> bool {
        !self.samples.is_empty()
    }

    /// Converts a client instant into the server's clock.
    pub fn to_server_us(&self, client_us: u64) -> i64 {
        client_us as i64 + self.offset_us
    }
}

fn median(values: impl Iterator<Item = i64>) -> i64 {
    let mut sorted: Vec<i64> = values.collect();
    if sorted.is_empty() {
        return 0;
    }
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

/// RFC 3550 interarrival jitter, in microseconds.
///
/// The formula is RTP's and is used here unchanged, with one adaptation: RTP
/// derives the sending interval from the sequence number and an assumed frame
/// rate, because that is all it has. Every chunk here carries `pts_us`, so the
/// sending interval is *known* rather than assumed — which keeps the estimate
/// honest when the source's frame rate is not what the constant says.
#[derive(Debug, Default)]
pub struct JitterEstimator {
    jitter_us: f64,
    last: Option<(u64, u64)>,
}

impl JitterEstimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one chunk's presentation time and the instant it arrived, both in
    /// the server's clock.
    pub fn observe(&mut self, pts_us: u64, arrival_us: i64) {
        let Some((last_pts, last_arrival)) = self.last else {
            self.last = Some((pts_us, arrival_us.max(0) as u64));
            return;
        };
        // Out-of-order chunks do not update the estimate. RFC 3550 makes the
        // same restriction: a reordered packet's transit difference describes
        // the reordering, not the jitter.
        if pts_us <= last_pts {
            return;
        }
        let sent = pts_us as i64 - last_pts as i64;
        let received = arrival_us - last_arrival as i64;
        let transit = (received - sent).abs() as f64;
        self.jitter_us += (transit - self.jitter_us) / 16.0;
        self.last = Some((pts_us, arrival_us.max(0) as u64));
    }

    pub fn jitter_us(&self) -> f64 {
        self.jitter_us
    }
}

/// Smallest playout delay the client will hold.
pub const MIN_PLAYOUT_DELAY_US: u32 = 10_000;
/// Largest playout delay the client will hold.
///
/// The phase's end-to-end target is 800 ms and Opus spends up to 26.5 ms of it;
/// half a second of buffer leaves the network the rest.
pub const MAX_PLAYOUT_DELAY_US: u32 = 500_000;
/// Safety factor applied to measured jitter.
const JITTER_MULTIPLIER: f64 = 3.0;
/// How much of the old target survives each update.
///
/// The target has to move slowly. Audio played at a shifting delay is audio
/// played at a shifting *rate*, and a rate change is heard where a delay is
/// not.
const DELAY_SMOOTHING: f64 = 0.99;

/// Turns measured jitter into the delay a chunk waits before playing.
#[derive(Debug)]
pub struct PlayoutDelay {
    target_us: f64,
}

impl PlayoutDelay {
    /// Starts at the server's suggestion, clamped into the supported range.
    pub fn new(suggested_us: u32) -> Self {
        Self {
            target_us: suggested_us.clamp(MIN_PLAYOUT_DELAY_US, MAX_PLAYOUT_DELAY_US) as f64,
        }
    }

    /// Folds a fresh jitter estimate into the target.
    pub fn update(&mut self, jitter_us: f64) {
        let raw = jitter_us * JITTER_MULTIPLIER;
        let clamped = raw.clamp(MIN_PLAYOUT_DELAY_US as f64, MAX_PLAYOUT_DELAY_US as f64);
        self.target_us = self.target_us * DELAY_SMOOTHING + clamped * (1.0 - DELAY_SMOOTHING);
    }

    pub fn target_us(&self) -> u32 {
        self.target_us.round() as u32
    }

    /// Whether a chunk with this presentation time is due.
    ///
    /// Early chunks wait. Late chunks are the caller's problem to drop — this
    /// only answers the question, because "is it due" and "is it too late" have
    /// different answers and merging them is how a late chunk gets played.
    pub fn is_due(&self, pts_us: u64, now_server_us: i64) -> bool {
        now_server_us >= pts_us as i64 + self.target_us as i64
    }

    /// Whether a chunk is so late that playing it would move the stream
    /// backwards.
    pub fn is_expired(&self, pts_us: u64, now_server_us: i64, frame_us: u32) -> bool {
        now_server_us > pts_us as i64 + self.target_us as i64 + frame_us as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One probe with a known offset and a known one-way delay.
    fn probe(sync: &mut ClockSync, client_send: u64, offset: i64, one_way: u64) {
        let server_recv = (client_send as i64 + offset + one_way as i64) as u64;
        let server_send = server_recv;
        let client_recv = client_send + 2 * one_way;
        sync.observe(client_send, server_recv, server_send, client_recv);
    }

    // TP-MEDIA-CLOCK-01
    #[test]
    fn a_symmetric_probe_recovers_the_offset_exactly() {
        // One line of arithmetic, and a sign error in it is invisible: the
        // stream still plays, just permanently early or permanently late by
        // twice the network delay, which looks like a slow link rather than
        // like a bug.
        let mut sync = ClockSync::new();
        probe(&mut sync, 1_000_000, 250_000, 5_000);
        assert_eq!(sync.offset_us(), 250_000);
        assert_eq!(sync.to_server_us(1_000_000), 1_250_000);
    }

    // TP-MEDIA-CLOCK-01
    #[test]
    fn an_unsynced_clock_says_so_rather_than_guessing_zero() {
        // Offset zero is a legitimate answer — two clocks can agree — so the
        // caller cannot use it to mean "no measurement yet". Asking is the only
        // way to tell "the clocks match" from "we have not looked".
        let sync = ClockSync::new();
        assert!(!sync.is_synced());
        assert_eq!(sync.offset_us(), 0);
    }

    // TP-MEDIA-CLOCK-01
    #[test]
    fn one_delayed_probe_does_not_move_the_offset() {
        // The reason this is a median and not an average. A single probe that
        // waited behind a large frame reports an offset off by half that wait,
        // and an average carries the error forever, spread thin. The median
        // discards it outright.
        let mut sync = ClockSync::new();
        for i in 0..8 {
            probe(&mut sync, 1_000_000 + i * 100_000, 250_000, 5_000);
        }
        assert_eq!(sync.offset_us(), 250_000);

        // One probe whose reply sat behind a 3 second write.
        sync.observe(2_000_000, 2_250_000 + 5_000, 2_255_000, 5_005_000);

        assert_eq!(
            sync.offset_us(),
            250_000,
            "a single outlier must not move a median built from eight good samples"
        );
    }

    // TP-MEDIA-CLOCK-01
    #[test]
    fn a_gap_longer_than_a_minute_throws_the_old_samples_away() {
        // A suspended laptop wakes with a clock that jumped. Every sample from
        // before the jump describes a relationship that no longer exists, and a
        // median holding nine of them outlives the jump by nine probes — during
        // which playback is wrong by the size of the jump and nothing reports
        // it.
        let mut sync = ClockSync::new();
        for i in 0..9 {
            probe(&mut sync, 1_000_000 + i * 100_000, 250_000, 5_000);
        }
        assert_eq!(sync.offset_us(), 250_000);

        // Same probe shape, a new offset, and a two-minute gap in front of it.
        let after_gap = 1_000_000 + 9 * 100_000 + 120_000_000;
        probe(&mut sync, after_gap, -750_000, 5_000);

        assert_eq!(
            sync.offset_us(),
            -750_000,
            "after a long gap the first fresh sample is the whole estimate"
        );
    }

    // TP-MEDIA-JITTER-01
    #[test]
    fn perfectly_regular_arrivals_produce_no_jitter() {
        let mut jitter = JitterEstimator::new();
        for frame in 0..50u64 {
            let pts = frame * 20_000;
            jitter.observe(pts, (pts + 100_000) as i64);
        }
        assert!(
            jitter.jitter_us() < 1.0,
            "regular arrivals reported {} us of jitter",
            jitter.jitter_us()
        );
    }

    // TP-MEDIA-JITTER-01
    #[test]
    fn a_late_arrival_raises_the_estimate_and_a_calm_link_lowers_it_again() {
        let mut jitter = JitterEstimator::new();
        for frame in 0..20u64 {
            let pts = frame * 20_000;
            jitter.observe(pts, (pts + 100_000) as i64);
        }
        let calm = jitter.jitter_us();

        // One chunk arrives 40 ms behind schedule.
        jitter.observe(20 * 20_000, (20 * 20_000 + 140_000) as i64);
        let disturbed = jitter.jitter_us();
        assert!(
            disturbed > calm,
            "a late arrival must raise the estimate ({calm} -> {disturbed})"
        );

        // The estimate has to come back down, or one bad moment raises the
        // playout delay for the rest of the session.
        for frame in 21..120u64 {
            let pts = frame * 20_000;
            jitter.observe(pts, (pts + 140_000) as i64);
        }
        assert!(
            jitter.jitter_us() < disturbed,
            "a steady link must bring the estimate back down"
        );
    }

    // TP-MEDIA-JITTER-01
    #[test]
    fn an_out_of_order_chunk_is_not_treated_as_jitter() {
        // RFC 3550 restricts the update to in-order packets, and the reason is
        // that a reordered chunk's transit difference measures the reordering.
        // Counting it inflates the playout delay in response to something the
        // buffer already handles.
        let mut jitter = JitterEstimator::new();
        for frame in 0..20u64 {
            let pts = frame * 20_000;
            jitter.observe(pts, (pts + 100_000) as i64);
        }
        let before = jitter.jitter_us();

        jitter.observe(5 * 20_000, (20 * 20_000 + 100_000) as i64);

        assert_eq!(jitter.jitter_us(), before);
    }

    // TP-MEDIA-JITTER-01
    #[test]
    fn the_playout_target_stays_inside_its_bounds() {
        // The lower bound keeps the buffer from vanishing on a quiet link; the
        // upper bound is what makes the phase's 800 ms end-to-end target
        // reachable at all, since the buffer is the largest term in it.
        let mut delay = PlayoutDelay::new(0);
        assert_eq!(delay.target_us(), MIN_PLAYOUT_DELAY_US);

        for _ in 0..10_000 {
            delay.update(10_000_000.0);
        }
        assert_eq!(delay.target_us(), MAX_PLAYOUT_DELAY_US);

        for _ in 0..10_000 {
            delay.update(0.0);
        }
        assert_eq!(delay.target_us(), MIN_PLAYOUT_DELAY_US);
    }

    // TP-MEDIA-JITTER-01
    #[test]
    fn the_playout_target_moves_slowly_rather_than_jumping() {
        // Audio played at a shifting delay is audio played at a shifting rate.
        // A rate change is audible where a delay is not, so the target has to
        // creep: a single update may not move it by a perceptible amount.
        let mut delay = PlayoutDelay::new(100_000);
        let before = delay.target_us();
        delay.update(MAX_PLAYOUT_DELAY_US as f64);
        let after = delay.target_us();

        assert!(after > before, "the target must respond at all");
        assert!(
            after - before < 10_000,
            "one update moved the target by {} us, which is audible",
            after - before
        );
    }

    // TP-MEDIA-JITTER-01
    #[test]
    fn an_early_chunk_waits_and_a_due_one_does_not() {
        // The rule the whole layer exists for. Playing on arrival hands every
        // network wobble straight to the speaker; waiting for the presentation
        // time is what converts a variable delay into a constant one.
        let delay = PlayoutDelay::new(100_000);
        let pts = 1_000_000;

        assert!(!delay.is_due(pts, 1_050_000), "50 ms early is still early");
        assert!(delay.is_due(pts, 1_100_000), "exactly due counts as due");
        assert!(delay.is_due(pts, 1_200_000));
    }

    // TP-MEDIA-JITTER-01
    #[test]
    fn a_chunk_past_its_frame_is_expired_and_due_is_not_the_same_question() {
        // Due and expired are separate questions with separate answers. Merging
        // them plays a chunk that arrived after its slot, which pushes the
        // whole stream backwards by one frame and keeps it there.
        let delay = PlayoutDelay::new(100_000);
        let pts = 1_000_000;
        let frame_us = 20_000;

        assert!(delay.is_due(pts, 1_100_000));
        assert!(!delay.is_expired(pts, 1_100_000, frame_us));
        assert!(!delay.is_expired(pts, 1_120_000, frame_us));
        assert!(delay.is_expired(pts, 1_120_001, frame_us));
    }
}
