//! A single-producer, single-consumer ring of samples.
//!
//! The seam between the playout thread and a native audio callback. The
//! callback runs on a real-time thread the OS owns, and the one rule of that
//! thread is that it never waits: no lock, no allocation, no syscall. A mutex
//! around a `VecDeque` would work almost always, and "almost always" on the
//! audio thread is an audible click at the worst possible moment — when the
//! playout thread happens to hold the lock while the device asks for samples.
//!
//! So: a fixed buffer, two atomic indices, and the two sides never touch each
//! other's index except to read it.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct Shared {
    buf: Box<[std::cell::UnsafeCell<f32>]>,
    /// Next slot the producer writes. Only the producer stores it.
    head: AtomicUsize,
    /// Next slot the consumer reads. Only the consumer stores it.
    tail: AtomicUsize,
}

// SAFETY: each slot is written only by the producer, at an index the consumer
// cannot yet read (head is published with Release after the write), and read
// only by the consumer, at an index the producer cannot yet overwrite (tail is
// published with Release after the read). The two indices are the whole
// protocol; the buffer itself needs no other synchronisation.
unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

/// Writes samples in. Held by the playout thread.
pub struct Producer {
    shared: Arc<Shared>,
}

/// Reads samples out. Held by the audio callback.
pub struct Consumer {
    shared: Arc<Shared>,
}

/// Creates a ring able to hold `capacity` samples.
///
/// One slot is always kept empty so that `head == tail` means empty and never
/// full; the ring therefore holds `capacity - 1` samples at most.
pub fn ring(capacity: usize) -> (Producer, Consumer) {
    let capacity = capacity.max(2);
    let buf: Vec<std::cell::UnsafeCell<f32>> = (0..capacity)
        .map(|_| std::cell::UnsafeCell::new(0.0))
        .collect();
    let shared = Arc::new(Shared {
        buf: buf.into_boxed_slice(),
        head: AtomicUsize::new(0),
        tail: AtomicUsize::new(0),
    });
    (
        Producer {
            shared: Arc::clone(&shared),
        },
        Consumer { shared },
    )
}

impl Producer {
    /// Samples the ring can accept right now.
    pub fn free(&self) -> usize {
        let cap = self.shared.buf.len();
        let head = self.shared.head.load(Ordering::Relaxed);
        let tail = self.shared.tail.load(Ordering::Acquire);
        (tail + cap - head - 1) % cap
    }

    /// Writes as many of `samples` as fit and returns how many that was.
    ///
    /// Never blocks and never overwrites: audio that does not fit is the
    /// caller's to drop, and it knows which frames those are.
    pub fn push(&mut self, samples: &[f32]) -> usize {
        let cap = self.shared.buf.len();
        let head = self.shared.head.load(Ordering::Relaxed);
        let tail = self.shared.tail.load(Ordering::Acquire);
        let free = (tail + cap - head - 1) % cap;
        let n = samples.len().min(free);
        for (i, sample) in samples[..n].iter().enumerate() {
            let slot = (head + i) % cap;
            // SAFETY: slot is in (tail - 1, head + free], which the consumer
            // does not read until head is published below.
            unsafe { *self.shared.buf[slot].get() = *sample };
        }
        self.shared.head.store((head + n) % cap, Ordering::Release);
        n
    }
}

impl Consumer {
    /// Samples waiting to be read.
    pub fn available(&self) -> usize {
        let cap = self.shared.buf.len();
        let head = self.shared.head.load(Ordering::Acquire);
        let tail = self.shared.tail.load(Ordering::Relaxed);
        (head + cap - tail) % cap
    }

    /// Fills `out` from the ring, zero-filling whatever the ring cannot
    /// supply. Returns how many samples were real.
    ///
    /// The zero fill is the underrun made audible as silence rather than as
    /// whatever was left in the device buffer, and the return value is how the
    /// caller counts it.
    pub fn pop_into(&mut self, out: &mut [f32]) -> usize {
        let cap = self.shared.buf.len();
        let head = self.shared.head.load(Ordering::Acquire);
        let tail = self.shared.tail.load(Ordering::Relaxed);
        let available = (head + cap - tail) % cap;
        let n = out.len().min(available);
        for (i, slot_out) in out[..n].iter_mut().enumerate() {
            let slot = (tail + i) % cap;
            // SAFETY: slot is in [tail, head), written and published by the
            // producer before head reached it.
            *slot_out = unsafe { *self.shared.buf[slot].get() };
        }
        for slot_out in &mut out[n..] {
            *slot_out = 0.0;
        }
        self.shared.tail.store((tail + n) % cap, Ordering::Release);
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TP-MEDIA-RING-01
    #[test]
    fn samples_come_out_in_the_order_they_went_in() {
        let (mut p, mut c) = ring(8);
        assert_eq!(p.push(&[1.0, 2.0, 3.0]), 3);
        let mut out = [0.0; 3];
        assert_eq!(c.pop_into(&mut out), 3);
        assert_eq!(out, [1.0, 2.0, 3.0]);
    }

    // TP-MEDIA-RING-01
    #[test]
    fn a_full_ring_refuses_rather_than_overwriting() {
        // Overwriting would drop the *oldest* audio, which is the audio about
        // to play. Refusing drops the newest, which the playout thread can
        // count and which the jitter buffer would have dropped soon anyway.
        let (mut p, mut c) = ring(4); // holds 3
        assert_eq!(p.push(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3);
        assert_eq!(p.free(), 0);
        let mut out = [0.0; 3];
        assert_eq!(c.pop_into(&mut out), 3);
        assert_eq!(out, [1.0, 2.0, 3.0]);
    }

    // TP-MEDIA-RING-01
    #[test]
    fn an_empty_ring_fills_the_callback_with_silence_and_says_how_much_was_real() {
        // The callback must always return a full buffer; what it cannot get
        // from the ring it fills with zeros, and the count is what turns that
        // silence into an underrun the main loop can see.
        let (mut p, mut c) = ring(8);
        p.push(&[0.5, 0.5]);
        let mut out = [9.0; 5];
        assert_eq!(c.pop_into(&mut out), 2);
        assert_eq!(out, [0.5, 0.5, 0.0, 0.0, 0.0]);
        assert_eq!(c.available(), 0);
    }

    // TP-MEDIA-RING-01
    #[test]
    fn the_ring_wraps_without_losing_a_sample() {
        // Wrap-around is where index arithmetic goes wrong, and it goes wrong
        // silently: samples in the wrong order sound like a click, not like a
        // panic.
        let (mut p, mut c) = ring(5); // holds 4
        let mut expected = Vec::new();
        let mut got = Vec::new();
        let mut next = 0.0f32;
        for round in 0..50 {
            let n = (round % 4) + 1;
            let batch: Vec<f32> = (0..n)
                .map(|_| {
                    next += 1.0;
                    next
                })
                .collect();
            let pushed = p.push(&batch);
            expected.extend_from_slice(&batch[..pushed]);
            let mut out = vec![0.0; pushed];
            assert_eq!(c.pop_into(&mut out), pushed);
            got.extend_from_slice(&out);
        }
        assert_eq!(got, expected);
    }

    // TP-MEDIA-RING-01
    #[test]
    fn producer_and_consumer_agree_across_threads() {
        // The whole point of the atomics: a producer on one thread and a
        // consumer on another see a consistent sequence, with no lock between
        // them.
        let (mut p, mut c) = ring(64);
        let total = 10_000usize;
        let producer = std::thread::spawn(move || {
            let mut i = 0usize;
            while i < total {
                let batch: Vec<f32> = (i..(i + 7).min(total)).map(|v| v as f32).collect();
                let n = p.push(&batch);
                i += n;
                if n == 0 {
                    std::thread::yield_now();
                }
            }
        });
        let mut seen = 0usize;
        let mut out = [0.0f32; 5];
        while seen < total {
            let n = c.pop_into(&mut out);
            for sample in &out[..n] {
                assert_eq!(*sample, seen as f32, "sample out of order at {seen}");
                seen += 1;
            }
            if n == 0 {
                std::thread::yield_now();
            }
        }
        producer.join().expect("producer");
    }
}
