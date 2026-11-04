# ccwire

A Rust SDK for [Claude Code](https://docs.claude.com/en/docs/claude-code): it
drives the `claude` CLI over its stream-json protocol, the way the official
TypeScript and Python Agent SDKs do.

Status: early. This release is the protocol layer only, checked against
Claude Code 2.1.296 (`ccwire::TESTED_CLI_VERSION`).

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

It is built to survive a CLI that ships weekly:

- Parsing a JSON object never fails. An unknown `type` is `Other` with the raw
  JSON; a known `type` that no longer parses is `Malformed` with the error and
  the raw JSON, so a protocol change shows up instead of disappearing.
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

## Features

- `runtime`: the process and session layers (tokio). Reserved; empty for now.

## Dependencies

`serde`, `serde_json` and `thiserror`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.
