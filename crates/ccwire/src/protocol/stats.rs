//! How many frames came out as [`Other`](super::Inbound::Other) (a `type`
//! this version doesn't model) or [`Malformed`](super::Inbound::Malformed)
//! (a known `type` that didn't parse, or a line that isn't a JSON object):
//! process-wide counters, to watch for protocol drift.
//!
//! With the `tracing` feature each one is also logged: `Other` at debug,
//! `Malformed` at warn, with its `type`.

use core::sync::atomic::Ordering;

// A target without 64-bit atomics (thumbv7em, without `std`) counts in a
// `usize`.
#[cfg(target_has_atomic = "64")]
use core::sync::atomic::AtomicU64 as Counter;
#[cfg(not(target_has_atomic = "64"))]
use core::sync::atomic::AtomicUsize as Counter;

static OTHER: Counter = Counter::new(0);
static MALFORMED: Counter = Counter::new(0);

/// The counts so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Counts {
    /// Frames of a `type` this version doesn't model.
    pub other: u64,
    /// Frames of a known `type` that didn't parse, and lines that aren't
    /// JSON objects (or are too long).
    pub malformed: u64,
}

/// The counts since the process started.
pub fn counts() -> Counts {
    Counts {
        other: load(&OTHER),
        malformed: load(&MALFORMED),
    }
}

#[cfg(target_has_atomic = "64")]
fn load(c: &Counter) -> u64 {
    c.load(Ordering::Relaxed)
}

#[cfg(not(target_has_atomic = "64"))]
fn load(c: &Counter) -> u64 {
    // usize is at most 64 bits wide.
    c.load(Ordering::Relaxed) as u64
}

/// A frame came out as `Other`.
pub(crate) fn other(type_: &str) {
    OTHER.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "tracing")]
    tracing::debug!(target: "ccwire", type_, "stream-json frame of an unknown type");
    #[cfg(not(feature = "tracing"))]
    let _ = type_;
}

/// A frame came out as `Malformed`.
pub(crate) fn malformed(type_: &str, error: &str) {
    MALFORMED.fetch_add(1, Ordering::Relaxed);
    #[cfg(feature = "tracing")]
    tracing::warn!(target: "ccwire", type_, error, "stream-json frame that didn't parse");
    #[cfg(not(feature = "tracing"))]
    let _ = (type_, error);
}
