//! A Rust SDK for Claude Code.
//!
//! ccwire drives the `claude` CLI over its stream-json protocol. This
//! release has the protocol types ([`protocol`]): pure data and serde,
//! no I/O and no async runtime, so anything that reads or writes stream-json
//! (a transcript viewer, a log analyzer, a process supervisor) can use them;
//! and a session's command line ([`Options`]), built without running it.
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

pub mod options;
pub mod protocol;

pub use options::Options;

/// The Claude Code version the golden corpus was recorded with: the
/// protocol these types are checked against.
pub const TESTED_CLI_VERSION: &str = "2.1.296";
