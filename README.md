# ccwire

A Rust SDK for [Claude Code](https://docs.claude.com/en/docs/claude-code): it
drives the `claude` CLI over its stream-json protocol, the way the official
TypeScript and Python Agent SDKs do.

Status: early. This release has the protocol layer, a session's command
line (`Options`), the line framing and the session core, checked against Claude Code 2.1.296
(`ccwire::TESTED_CLI_VERSION`). Spawning and reaping the `claude` process,
`query()` and permission/hook/MCP helpers come next.

## Install

```sh
cargo add ccwire                     # the protocol types and sync framing
cargo add ccwire --features runtime  # plus the async framing and the session core
```

Until the first crates.io release, use the git repository:

```toml
[dependencies]
ccwire = { git = "https://github.com/talaagent/ccwire", features = ["runtime"] }
```

The workspace also has [`ccwire-oauth`](https://github.com/talaagent/ccwire/tree/main/ccwire-oauth), a typed client for
the Claude account endpoints an OAuth login can call.

## `ccwire::protocol`

The stream-json protocol as Rust types: pure data and serde, with no I/O and
no async runtime.

- `Inbound`: every frame the CLI writes (`user`, `assistant`, `system`,
  `result`, `stream_event`, `rate_limit_event`, `control_request`,
  `control_response`, `control_cancel_request`).
- `Message`: the conversation part of it; `Control`: the control channel.
  `Inbound::split` gives one or the other.
- `UserInput`: a user message to write to the CLI's stdin.
- `ControlRequest`, `ControlResponse`, `PermissionResult`: the control
  channel, both directions.
- `Partial`, `PartialMessage`: `stream_event` deltas
  (`--include-partial-messages`), read and assembled into a message's text.

It is built to survive a CLI that ships weekly:

- Parsing a JSON object never fails. An unknown `type` is `Other` with the raw
  JSON; a known `type` that no longer parses is `Malformed` with the error and
  the raw JSON, so a protocol change shows up instead of disappearing.
  `protocol::stats::counts()` counts both (and, with the `tracing` feature,
  each is logged with its type).
- Every struct keeps the keys it doesn't model (`extra`), and optional keys
  tell absent from `null` (`Field`), so a frame serializes back to the same
  JSON value.
- Public enums are `#[non_exhaustive]`.
- A `result` frame ends a turn, so it parses leniently, and
  `Inbound::ends_turn` also matches a `result` that came out `Malformed`.

```rust
use ccwire::protocol::{Frame, Inbound, Message};

fn on_line(line: &str) {
    let Ok(frame) = Inbound::from_line(line) else { return };
    match frame.split() {
        Frame::Message(Message::Assistant(a)) => {
            for block in a.content() {
                if let Some(text) = block.as_text() {
                    println!("{text}");
                }
            }
        }
        Frame::Message(Message::Result(r)) => println!("cost: {:?}", r.cost_usd()),
        Frame::Message(m) if m.ends_turn() => println!("turn over (unparsed result)"),
        _ => {}
    }
}
```

## `ccwire::framing`

One JSON frame per line. `LineReader` (sync) and `AsyncLineReader` split a
byte stream into lines under a maximum length (64 MiB by default): a longer
line is skipped and reported, and reading goes on. `FrameReader` turns lines
into `Inbound` frames and never fails on content: garbage, a cut-off last
line or an oversized line comes out as `Inbound::Malformed` (a `result` among
them still ends the turn). Every line carries the byte offset past it, so a
reader can resume a file where it stopped. `FrameWriter` writes frames.

## `ccwire::session` (feature `runtime`)

A session over any `AsyncRead` (the CLI's output) and `AsyncWrite` (its
input), not tied to a child process:

```rust,ignore
use ccwire::{protocol::Inbound, session::Session};

let mut session = Session::new(child_stdout, child_stdin);
session.send("hi").await?;
while let Some(frame) = session.next().await {
    let frame = frame?;
    if frame.ends_turn() {
        break;
    }
}
session.interrupt().await?; // a control round trip
session.close().await?;
```

- Control round trips: `request` (with `interrupt`, `set_model`,
  `set_permission_mode`, `stop_task` and `request_raw` on top) gets a fresh
  `request_id` and resolves on the matching `control_response`.
- The CLI's own control requests (`can_use_tool`, ...) go to a
  `ControlHandler`, each on its own task, or come out of the stream to answer
  with `respond`. A `control_cancel_request` aborts the handler.
- `SessionState` (`Idle`, `InTurn`, `Closed`), watchable.
- The run-end rule: `finish()` closes the input at once without callbacks,
  and once idle with them.
- One writer task, so dropping a `send` or `request` future never leaves half
  a line; dropping the `Session` aborts its tasks.

## Features

- `runtime` (off by default): the async framing and the session core, on
  tokio (`io-util`, `sync`, `rt`).
- `tracing` (off by default): log each `Other` (debug) and `Malformed` (warn)
  frame with its type.

## Dependencies

`serde`, `serde_json` and `thiserror`; `tokio` with `runtime`; `tracing` with
`tracing`.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
