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
//! # Async sessions
//!
//! Enable `ccwire = { version = "0.1", features = ["runtime"] }` and run
//! inside a Tokio runtime. The caller owns process startup and cleanup:
//! [`Options`] builds argv, and `session::Session` handles the pipes.
//!
//! ```no_run
//! # #[cfg(feature = "runtime")]
//! # async fn example<R, W>(stdout: R, stdin: W) -> Result<(), ccwire::session::Error>
//! # where R: tokio::io::AsyncRead + Unpin + Send + 'static,
//! #       W: tokio::io::AsyncWrite + Unpin + Send + 'static {
//! use ccwire::session::Session;
//!
//! let mut session = Session::new(stdout, stdin);
//! session.send("Hello!").await?;
//! while let Some(frame) = session.next().await {
//!     let frame = frame?;
//!     println!("{frame:?}");
//!     if frame.ends_turn() {
//!         break;
//!     }
//! }
//! session.close().await?;
//! # Ok(()) }
//! ```
//!
//! See the [complete CLI example](https://github.com/talaagent/ccwire/blob/main/crates/ccwire/examples/chat.rs)
//! for spawning and reaping a child process.
//!
//! # Features
//!
//! | Feature | Default | Enables |
//! | --- | --- | --- |
//! | `runtime` | No | Tokio-based async framing and sessions |
//! | `tracing` | No | Debug logs for unknown frames; warnings for malformed frames |
//!
//! Without features, protocol parsing, synchronous framing, and CLI options
//! work without an async runtime. Checked against [`TESTED_CLI_VERSION`];
//! unknown protocol variants and extra fields are preserved.
//!
//! # License
//!
//! Licensed under the Apache License, Version 2.0 ([Apache-2.0](https://github.com/talaagent/ccwire/blob/main/LICENSE)).

#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]

extern crate alloc;

pub mod framing;
pub mod options;
pub mod protocol;
#[cfg(feature = "runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "runtime")))]
pub mod session;

pub use options::Options;

/// The Claude Code version the golden corpus was recorded with: the
/// protocol these types are checked against.
pub const TESTED_CLI_VERSION: &str = "2.1.296";

/// README examples, checked alongside the API examples.
#[cfg(all(doctest, feature = "runtime"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
