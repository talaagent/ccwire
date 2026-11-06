//! The golden corpus (`tests/corpus/`): every line parses, the core types
//! never come out as `Other` or `Malformed`, and every frame serializes back
//! to the same JSON value.

extern crate alloc;

use alloc::collections::BTreeMap;

use ccwire::protocol::{
    ContentBlock, Control, Frame, Inbound, Message, MessageContent, System, ToolResultContent,
};
use serde_json::Value;

const FILES: &[(&str, &str)] = &[
    ("activity.jsonl", include_str!("corpus/activity.jsonl")),
    (
        "partial-text.jsonl",
        include_str!("corpus/partial-text.jsonl"),
    ),
    (
        "partial-tool.jsonl",
        include_str!("corpus/partial-tool.jsonl"),
    ),
    ("multiturn.jsonl", include_str!("corpus/multiturn.jsonl")),
    ("stream.jsonl", include_str!("corpus/stream.jsonl")),
    (
        "subagents-background.jsonl",
        include_str!("corpus/subagents-background.jsonl"),
    ),
    (
        "subagents-fanout.jsonl",
        include_str!("corpus/subagents-fanout.jsonl"),
    ),
    (
        "subagents-nested.jsonl",
        include_str!("corpus/subagents-nested.jsonl"),
    ),
    (
        "subagents-self_resume.jsonl",
        include_str!("corpus/subagents-self_resume.jsonl"),
    ),
    (
        "subagents-single.jsonl",
        include_str!("corpus/subagents-single.jsonl"),
    ),
    ("synthetic.jsonl", include_str!("corpus/synthetic.jsonl")),
];

/// Top-level types the corpus has that ccwire leaves as `Other` on purpose.
const OTHER_TYPES: &[&str] = &[];
/// `system` subtypes left as `System::Other` on purpose.
const OTHER_SYSTEM: &[&str] = &["hook_response", "thinking_tokens"];
/// Content block types left as `ContentBlock::Other` on purpose.
const OTHER_BLOCKS: &[&str] = &["web_search_tool_result"];

fn lines() -> impl Iterator<Item = (String, Value)> {
    FILES.iter().flat_map(|(name, text)| {
        text.lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .map(move |(i, l)| {
                let v: Value = serde_json::from_str(l)
                    .unwrap_or_else(|e| panic!("{name}:{}: not JSON: {e}", i + 1));
                (format!("{name}:{}", i + 1), v)
            })
    })
}

fn check_blocks(at: &str, blocks: &[ContentBlock], kinds: &mut BTreeMap<String, usize>) {
    for b in blocks {
        *kinds.entry(format!("block/{}", b.tag())).or_default() += 1;
        match b {
            ContentBlock::Other { type_, .. } => {
                assert!(
                    OTHER_BLOCKS.contains(&type_.as_str()),
                    "{at}: unexpected Other block {type_}"
                );
            }
            ContentBlock::ToolResult(r) => {
                if let Some(ToolResultContent::Blocks(inner)) = r.content.get() {
                    check_blocks(at, inner, kinds);
                }
            }
            _ => {}
        }
    }
}

fn check_message(at: &str, m: &Message, kinds: &mut BTreeMap<String, usize>) {
    match m {
        Message::Malformed { type_, error, .. } => panic!("{at}: malformed {type_}: {error}"),
        Message::Other { type_, .. } => {
            assert!(
                OTHER_TYPES.contains(&type_.as_str()),
                "{at}: unexpected Other type {type_:?}"
            );
        }
        Message::System(System::Other { subtype, .. }) => {
            assert!(
                OTHER_SYSTEM.contains(&subtype.as_str()),
                "{at}: unexpected Other system subtype {subtype:?}"
            );
        }
        Message::Assistant(a) => check_blocks(at, a.content(), kinds),
        Message::User(u) => {
            if let MessageContent::Blocks(b) = u.content() {
                check_blocks(at, b, kinds);
            }
        }
        _ => {}
    }
}

/// The fields typed since they were extras, that the frame has, each with
/// whether it parsed into its typed field (not `extra`).
fn typed_fields(i: &Inbound) -> Vec<(&'static str, bool)> {
    let mut out = Vec::new();
    match i {
        Inbound::StreamEvent(e) if !e.api_message_id.is_absent() => {
            out.push(("api_message_id", !e.extra.contains_key("api_message_id")));
        }
        Inbound::System(System::Init(init)) if !init.terminal_slash_commands.is_absent() => {
            out.push((
                "terminal_slash_commands",
                !init.extra.contains_key("terminal_slash_commands"),
            ));
        }
        Inbound::System(System::TaskStarted(t)) if !t.owned_by_subagent.is_absent() => {
            out.push((
                "owned_by_subagent",
                !t.extra.contains_key("owned_by_subagent"),
            ));
        }
        Inbound::System(System::TaskNotification(t)) if !t.owned_by_subagent.is_absent() => {
            out.push((
                "owned_by_subagent",
                !t.extra.contains_key("owned_by_subagent"),
            ));
        }
        _ => {}
    }
    out
}

#[test]
fn corpus_parses_typed_and_round_trips() {
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut n = 0;
    for (at, raw) in lines() {
        n += 1;
        let inbound: Inbound = serde_json::from_value(raw.clone())
            .unwrap_or_else(|e| panic!("{at}: an object must always parse: {e}"));
        let kind = match &inbound {
            Inbound::System(s) => format!("system/{}", s.tag()),
            Inbound::ControlRequest(r) => format!("control_request/{}", r.request.tag()),
            Inbound::ControlResponse(r) => format!("control_response/{}", r.response.tag()),
            Inbound::Result(r) => {
                *kinds.entry("result".to_owned()).or_default() += 1;
                let sub = r.subtype.get().map_or("", |s| s.as_str());
                if r.is_success() {
                    *kinds.entry("result/success".to_owned()).or_default() += 1;
                } else if r.failed() {
                    *kinds.entry("result/error".to_owned()).or_default() += 1;
                }
                assert!(inbound.ends_turn(), "{at}: a result ends the turn");
                format!("result/{sub}")
            }
            i => i.type_name().to_owned(),
        };
        *kinds.entry(kind).or_default() += 1;

        for (field, typed) in typed_fields(&inbound) {
            assert!(typed, "{at}: {field} isn't typed");
            *kinds.entry(format!("field/{field}")).or_default() += 1;
        }

        let back = serde_json::to_value(&inbound).expect("serializes");
        assert_eq!(back, raw, "{at}: round trip changed the frame");

        match inbound.split() {
            Frame::Message(m) => check_message(&at, &m, &mut kinds),
            Frame::Control(Control::Request(r)) => {
                assert!(
                    !matches!(
                        r.request,
                        ccwire::protocol::ControlRequestBody::Other { .. }
                    ),
                    "{at}: unexpected Other control request"
                );
            }
            Frame::Control(_) => {}
        }
    }
    assert!(n > 300, "corpus has {n} lines");

    // Every core type and subtype is in the corpus at least once.
    for k in [
        "user",
        "assistant",
        "system/init",
        "system/compact_boundary",
        "system/task_started",
        "system/task_progress",
        "system/task_updated",
        "system/task_notification",
        "system/status",
        "system/background_tasks_changed",
        "command_lifecycle",
        "result",
        "result/success",
        "result/error",
        "result/error_during_execution",
        "result/error_max_turns",
        "result/error_max_budget_usd",
        "stream_event",
        "rate_limit_event",
        "control_request/interrupt",
        "control_request/initialize",
        "control_request/set_permission_mode",
        "control_request/set_model",
        "control_request/stop_task",
        "control_request/can_use_tool",
        "control_request/hook_callback",
        "control_request/mcp_message",
        "control_response/success",
        "control_response/error",
        "control_cancel_request",
        "block/text",
        "block/thinking",
        "block/redacted_thinking",
        "block/tool_use",
        "block/tool_result",
        "block/server_tool_use",
        "block/image",
        "field/api_message_id",
        "field/owned_by_subagent",
        "field/terminal_slash_commands",
    ] {
        assert!(kinds.contains_key(k), "corpus lacks {k}: {kinds:?}");
    }
}

#[test]
fn corpus_is_of_the_tested_cli_version() {
    let mut seen = 0;
    for (at, raw) in lines() {
        if let Some(v) = raw.get("claude_code_version").and_then(Value::as_str) {
            seen += 1;
            assert_eq!(v, ccwire::TESTED_CLI_VERSION, "{at}");
        }
    }
    assert!(seen > 0, "no init frame carries claude_code_version");
}
