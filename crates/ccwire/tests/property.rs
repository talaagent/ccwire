//! Property: any JSON object parses as an `Inbound` without panicking or
//! failing, and serializes back to the same value. Arbitrary objects come
//! from a small seeded generator biased towards the protocol's own keys, so
//! the typed paths (and their failures) get exercised, not only `Other`.

use ccwire::protocol::{Inbound, Message};
use serde_json::{Map, Number, Value};

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap_or(1)).unwrap_or(0)
    }

    fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
        xs[self.below(xs.len())]
    }
}

const TYPES: &[&str] = &[
    "user",
    "assistant",
    "system",
    "result",
    "stream_event",
    "rate_limit_event",
    "control_request",
    "control_response",
    "control_cancel_request",
    "text",
    "thinking",
    "redacted_thinking",
    "tool_use",
    "tool_result",
    "server_tool_use",
    "image",
    "message",
    "something_new",
    "",
];

const SUBTYPES: &[&str] = &[
    "init",
    "compact_boundary",
    "task_started",
    "task_progress",
    "task_updated",
    "task_notification",
    "status",
    "success",
    "error",
    "error_max_turns",
    "error_during_execution",
    "interrupt",
    "initialize",
    "set_permission_mode",
    "set_model",
    "stop_task",
    "can_use_tool",
    "hook_callback",
    "mcp_message",
    "rewind_files",
];

const KEYS: &[&str] = &[
    "message",
    "content",
    "role",
    "text",
    "thinking",
    "signature",
    "data",
    "id",
    "name",
    "input",
    "tool_use_id",
    "is_error",
    "source",
    "session_id",
    "uuid",
    "parent_tool_use_id",
    "model",
    "usage",
    "input_tokens",
    "output_tokens",
    "result",
    "duration_ms",
    "num_turns",
    "total_cost_usd",
    "errors",
    "modelUsage",
    "event",
    "rate_limit_info",
    "resetsAt",
    "utilization",
    "request_id",
    "request",
    "response",
    "error",
    "tool_name",
    "callback_id",
    "server_name",
    "mode",
    "task_id",
    "patch",
    "mcp_servers",
    "tools",
    "status",
    "compact_metadata",
    "pre_tokens",
    "behavior",
    "x",
];

fn scalar(r: &mut Rng) -> Value {
    match r.below(8) {
        0 => Value::Null,
        1 => Value::Bool(r.below(2) == 0),
        2 => Value::from(r.next() % 1000),
        3 => Value::from(-i64::try_from(r.next() % 1000).unwrap_or(0)),
        4 => Number::from_f64(f64::from(u32::try_from(r.next() % 10_000).unwrap_or(0)) / 7.0)
            .map_or(Value::Null, Value::Number),
        5 => Value::from(r.pick(TYPES)),
        6 => Value::from(r.pick(SUBTYPES)),
        _ => Value::from(format!("s{}", r.next() % 100)),
    }
}

fn value(r: &mut Rng, depth: u32) -> Value {
    if depth == 0 {
        return scalar(r);
    }
    match r.below(6) {
        0 | 1 => scalar(r),
        2 => Value::Array((0..r.below(4)).map(|_| value(r, depth - 1)).collect()),
        _ => Value::Object(object(r, depth - 1)),
    }
}

fn object(r: &mut Rng, depth: u32) -> Map<String, Value> {
    let mut m = Map::new();
    if r.below(5) != 0 {
        let t = if r.below(10) == 0 {
            scalar(r)
        } else {
            Value::from(r.pick(TYPES))
        };
        m.insert("type".into(), t);
    }
    if r.below(2) == 0 {
        m.insert("subtype".into(), Value::from(r.pick(SUBTYPES)));
    }
    for _ in 0..r.below(7) {
        let k = r.pick(KEYS).to_owned();
        let v = value(r, depth);
        m.insert(k, v);
    }
    m
}

#[test]
fn arbitrary_objects_always_parse_and_round_trip() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut typed, mut other, mut malformed) = (0, 0, 0);
    for _ in 0..10_000 {
        let raw = Value::Object(object(&mut r, 4));
        let line = raw.to_string();
        let inbound = Inbound::from_line(&line)
            .unwrap_or_else(|e| panic!("an object must always parse: {e}: {line}"));
        // Compared with the line as serde_json reads it: without its
        // `float_roundtrip` feature (ccwire doesn't turn it on), parsing
        // text may move a float's last digit, before ccwire sees it.
        let parsed: Value = serde_json::from_str(&line).expect("valid JSON");
        let back = serde_json::to_value(&inbound).expect("serializes");
        assert_eq!(back, parsed, "round trip changed {line}");
        match Message::try_from(inbound) {
            Ok(Message::Other { .. }) => other += 1,
            Ok(Message::Malformed { .. }) => malformed += 1,
            _ => typed += 1,
        }
    }
    // The generator reaches every path.
    assert!(
        typed > 100 && other > 100 && malformed > 100,
        "{typed} {other} {malformed}"
    );
}

#[test]
fn non_objects_are_errors_not_panics() {
    for line in [
        "",
        "null",
        "1",
        "\"x\"",
        "[]",
        "[{\"type\":\"user\"}]",
        "{",
        "nope",
    ] {
        assert!(Inbound::from_line(line).is_err(), "{line:?}");
    }
}
