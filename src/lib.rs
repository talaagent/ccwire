//! A Rust SDK for Claude Code.
//!
//! ccwire drives the `claude` CLI over its stream-json protocol.
//!
//! - [`protocol`]: the protocol types, pure data and serde, no I/O and no
//!   async runtime, so anything that reads or writes stream-json (a
//!   transcript viewer, a log analyzer, a process supervisor) can use them.
//! - [`Options`]: a session's command line, built without running it.
//! - [`framing`]: one JSON frame per line, with a bounded line length (a
//!   sync reader always; the async reader and writer with `runtime`).
//! - `session` (feature `runtime`): a session over any `AsyncRead` and
//!   `AsyncWrite` (user messages, the frame stream, control round trips,
//!   interrupt, the run-end rule), not tied to a child process.
//!
//! ```
//! use ccwire::protocol::{Frame, Inbound, Message};
//!
//! let line = r#"{"type":"result","subtype":"success","num_turns":1,"session_id":"s1"}"#;
//! let inbound = Inbound::from_line(line).expect("a JSON object");
//! match inbound.split() {
//!     Frame::Message(Message::Result(r)) => assert!(r.is_success()),
//!     other => panic!("unexpected {other:?}"),
//! }
//! ```
//!
//! # License
//!
//! Licensed under the Apache License, Version 2.0 (`LICENSE-APACHE`).

extern crate alloc;

pub mod framing;
pub mod options;
pub mod protocol;
#[cfg(feature = "runtime")]
pub mod session;

pub use options::Options;

/// The Claude Code version the golden corpus was recorded with: the
/// protocol these types are checked against.
pub const TESTED_CLI_VERSION: &str = "2.1.296";
