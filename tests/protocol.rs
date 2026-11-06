//! Unit serde round trips and shapes, compared as JSON values.

use ccwire::protocol::{
    ContentBlock, Control, ControlRequest, ControlRequestBody, ControlResponse, Field, Frame,
    Inbound, InterruptRequest, Message, PermissionResult, ResultSubtype, SetModelRequest, System,
    UserInput,
};
use serde_json::{Value, json};

fn parse(v: &Value) -> Inbound {
    serde_json::from_value(v.clone()).expect("an object always parses")
}

fn round_trip(v: &Value) -> Inbound {
    let i = parse(v);
    assert_eq!(&serde_json::to_value(&i).expect("serializes"), v);
    i
}

#[test]
fn result_success() {
    let v = json!({"type": "result", "subtype": "success", "is_error": false,
        "result": "hi", "duration_ms": 12, "num_turns": 1, "total_cost_usd": 0.005_310_880_000_000_000_5,
        "usage": {"input_tokens": 2, "output_tokens": 3, "server_tool_use": {"web_search_requests": 0}},
        "session_id": "s", "permission_denials": []});
    let Inbound::Result(r) = round_trip(&v) else {
        panic!("not a result")
    };
    assert!(r.is_success() && !r.failed());
    assert_eq!(r.result_text(), Some("hi"));
    assert_eq!(r.cost_usd(), Some(0.005_310_880_000_000_000_5));
    assert_eq!(
        r.usage.get().map(ccwire::protocol::Usage::context_tokens),
        Some(2)
    );
    assert!(r.extra.contains_key("permission_denials"));
}

#[test]
fn result_error_keeps_null_and_unknown_subtype() {
    let v = json!({"type": "result", "subtype": "error_during_execution", "is_error": true,
        "result": null, "errors": ["interrupted"]});
    let Inbound::Result(r) = round_trip(&v) else {
        panic!("not a result")
    };
    assert!(r.result.is_null() && r.failed() && !r.is_success());
    assert_eq!(r.subtype.get(), Some(&ResultSubtype::ErrorDuringExecution));
    assert_eq!(r.error_messages(), ["interrupted"]);

    let v = json!({"type": "result", "subtype": "error_brand_new"});
    let Inbound::Result(r) = round_trip(&v) else {
        panic!("not a result")
    };
    assert_eq!(
        r.subtype.get(),
        Some(&ResultSubtype::Other("error_brand_new".into()))
    );
    assert!(r.failed());
}

#[test]
fn drifted_result_stays_a_result() {
    // No subtype, a non-string `result`, `errors` of objects, a numeric
    // subtype: still a Result, and it still ends the turn.
    for v in [
        json!({"type": "result", "is_error": false, "num_turns": 1}),
        json!({"type": "result", "subtype": "success", "result": {"text": "hi"}}),
        json!({"type": "result", "subtype": "error_during_execution",
            "errors": [{"message": "boom"}, "plain"]}),
        json!({"type": "result", "subtype": 3, "errors": "one"}),
        json!({"type": "result", "subtype": null, "result": null}),
    ] {
        let i = round_trip(&v);
        assert!(i.ends_turn(), "{v}");
        let Inbound::Result(r) = i else {
            panic!("not a result: {v}")
        };
        assert!(!r.is_success() || r.result_text().is_none());
    }
    let Inbound::Result(r) = parse(
        &json!({"type": "result", "subtype": "error_during_execution",
        "errors": [{"message": "boom"}, "plain"]}),
    ) else {
        panic!("not a result")
    };
    assert_eq!(r.error_messages(), ["plain"]);
    let Inbound::Result(r) = parse(&json!({"type": "result", "subtype": 3})) else {
        panic!("not a result")
    };
    assert!(matches!(
        r.subtype.get(),
        Some(ResultSubtype::NotAString(_))
    ));
    assert!(!r.is_success() && !r.failed());
}

#[test]
fn malformed_result_ends_the_turn() {
    let v = json!({"type": "result", "subtype": "success", "num_turns": "one"});
    let i = round_trip(&v);
    assert!(matches!(&i, Inbound::Malformed { type_, .. } if type_ == "result"));
    assert!(i.ends_turn());
    let m = Message::try_from(i).expect("a message");
    assert!(m.ends_turn());

    let other = parse(&json!({"type": "assistant", "message": {"content": "not blocks"}}));
    assert!(!other.ends_turn());
    assert!(!parse(&json!({"type": "user", "message": {"content": "x"}})).ends_turn());
}

#[test]
fn assistant_blocks() {
    let v = json!({"type": "assistant", "parent_tool_use_id": null, "session_id": "s",
        "message": {"id": "m", "type": "message", "role": "assistant", "content": [
            {"type": "text", "text": "a"},
            {"type": "tool_use", "id": "t", "name": "Bash", "input": {"command": "ls"}},
            {"type": "mystery_block", "x": 1}
        ], "usage": {"input_tokens": 1, "cache_read_input_tokens": 10}}});
    let Inbound::Assistant(a) = round_trip(&v) else {
        panic!("not assistant")
    };
    assert!(!a.is_subagent());
    assert!(a.parent_tool_use_id.is_null());
    let c = a.content();
    assert_eq!(c[0].as_text(), Some("a"));
    assert!(matches!(&c[1], ContentBlock::ToolUse(t) if t.name == "Bash"));
    assert!(matches!(&c[2], ContentBlock::Other { type_, .. } if type_ == "mystery_block"));
}

#[test]
fn system_subtypes() {
    let v = json!({"type": "system", "subtype": "init", "session_id": "s", "model": "m",
        "mcp_servers": [{"name": "a", "status": "failed"}], "permissionMode": "plan",
        "plugins": []});
    let Inbound::System(System::Init(i)) = round_trip(&v) else {
        panic!("not init")
    };
    assert_eq!(i.mcp_status("a"), Some("failed"));
    assert_eq!(i.permission_mode.as_deref(), Some("plan"));

    let v = json!({"type": "system", "subtype": "task_updated", "task_id": "t",
        "patch": {"status": "killed"}});
    let Inbound::System(System::TaskUpdated(t)) = round_trip(&v) else {
        panic!("not task_updated")
    };
    assert_eq!(t.any_status(), Some("killed"));

    let v = json!({"type": "system", "subtype": "hook_started", "hook_name": "x"});
    let Inbound::System(System::Other { subtype, raw }) = round_trip(&v) else {
        panic!("not other")
    };
    assert_eq!(subtype, "hook_started");
    assert_eq!(raw, v);
}

#[test]
fn unknown_type_is_other() {
    let v = json!({"type": "brand_new", "a": [1, 2]});
    assert!(
        matches!(round_trip(&v), Inbound::Other { type_, raw } if type_ == "brand_new" && raw == v)
    );
    let v = json!({"no_type": true});
    assert!(matches!(round_trip(&v), Inbound::Other { type_, .. } if type_.is_empty()));
    let v = json!({"type": 7});
    assert!(matches!(round_trip(&v), Inbound::Other { type_, .. } if type_.is_empty()));
}

#[test]
fn known_type_of_a_new_shape_is_malformed() {
    let v = json!({"type": "assistant", "message": {"content": "not blocks"}});
    let Inbound::Malformed { type_, error, raw } = round_trip(&v) else {
        panic!("not malformed")
    };
    assert_eq!(type_, "assistant");
    assert_ne!(error, "");
    assert_eq!(raw, v);

    let v = json!({"type": "system", "subtype": "init", "model": 5});
    assert!(matches!(round_trip(&v), Inbound::Malformed { type_, .. } if type_ == "system"));
}

#[test]
fn malformed_content_keeps_the_block_error() {
    // A tool_use block without `id`: the error names it, not serde's
    // untagged "did not match any variant".
    let v = json!({"type": "user", "message": {"role": "user", "content": [
        {"type": "text", "text": "a"},
        {"type": "tool_use", "name": "Bash", "input": {}}
    ]}});
    let Inbound::Malformed { error, .. } = round_trip(&v) else {
        panic!("not malformed")
    };
    assert!(
        error.contains("content block 1") && error.contains("`id`"),
        "{error}"
    );

    let v = json!({"type": "user", "message": {"role": "user", "content": [
        {"type": "tool_result", "tool_use_id": "t", "content": [{"type": "text", "text": 1}]}
    ]}});
    let Inbound::Malformed { error, .. } = round_trip(&v) else {
        panic!("not malformed")
    };
    assert!(!error.contains("did not match any variant"), "{error}");
    assert!(error.contains("content block 0"), "{error}");

    let v = json!({"type": "user", "message": {"role": "user", "content": 5}});
    let Inbound::Malformed { error, .. } = round_trip(&v) else {
        panic!("not malformed")
    };
    assert!(error.contains("a string or an array"), "{error}");
}

#[test]
fn control_requests_and_responses() {
    let v = json!({"type": "control_request", "request_id": "r1", "request": {
        "subtype": "can_use_tool", "tool_name": "Bash", "input": {"command": "ls"},
        "permission_suggestions": null, "blocked_path": null, "tool_use_id": "t"}});
    let i = round_trip(&v);
    let Frame::Control(Control::Request(r)) = i.split() else {
        panic!("not a control request")
    };
    let ControlRequestBody::CanUseTool(c) = &r.request else {
        panic!("not can_use_tool")
    };
    assert_eq!(c.tool_name, "Bash");
    assert!(c.permission_suggestions.is_null());

    // Built by hand: what a caller writes.
    let out = Inbound::ControlRequest(ControlRequest::new("r2", InterruptRequest::default()));
    assert_eq!(
        serde_json::to_value(&out).unwrap(),
        json!({"type": "control_request", "request_id": "r2", "request": {"subtype": "interrupt"}})
    );
    let out = Inbound::ControlRequest(ControlRequest::new(
        "r3",
        SetModelRequest {
            model: Field::Null,
            ..SetModelRequest::default()
        },
    ));
    assert_eq!(
        serde_json::to_value(&out).unwrap(),
        json!({"type": "control_request", "request_id": "r3",
            "request": {"subtype": "set_model", "model": null}})
    );

    let v = json!({"type": "control_request", "request_id": "r4",
        "request": {"subtype": "rewind_files", "user_message_id": "u"}});
    let Inbound::ControlRequest(r) = round_trip(&v) else {
        panic!("not a control request")
    };
    assert_eq!(r.request.tag(), "rewind_files");

    let v = json!({"type": "control_cancel_request", "request_id": "r1"});
    assert!(matches!(round_trip(&v), Inbound::ControlCancelRequest(c) if c.request_id == "r1"));

    let v = json!({"type": "control_response", "response": {"subtype": "error",
        "request_id": "r5", "error": "nope"}});
    let Inbound::ControlResponse(r) = round_trip(&v) else {
        panic!("not a control response")
    };
    assert_eq!(r.request_id(), Some("r5"));
}

#[test]
fn control_response_builders() {
    let to = |r: ControlResponse| serde_json::to_value(Inbound::ControlResponse(r)).unwrap();
    assert_eq!(
        to(ControlResponse::success("r", None)),
        json!({"type": "control_response", "response": {"subtype": "success", "request_id": "r"}})
    );
    assert_eq!(
        to(ControlResponse::error("r", "bad")),
        json!({"type": "control_response",
            "response": {"subtype": "error", "request_id": "r", "error": "bad"}})
    );
    assert_eq!(
        to(
            ControlResponse::permission("r", &PermissionResult::allow(json!({"command": "ls"})))
                .unwrap()
        ),
        json!({"type": "control_response", "response": {"subtype": "success", "request_id": "r",
            "response": {"behavior": "allow", "updatedInput": {"command": "ls"}}}})
    );
}

#[test]
fn permission_results() {
    let deny = PermissionResult::deny("no");
    assert_eq!(
        serde_json::to_value(&deny).unwrap(),
        json!({"behavior": "deny", "message": "no"})
    );
    for v in [
        json!({"behavior": "allow", "updatedInput": {"a": 1},
            "updatedPermissions": [{"type": "setMode", "mode": "plan", "destination": "session"}]}),
        json!({"behavior": "deny", "message": "no", "interrupt": true, "future": 1}),
    ] {
        let p: PermissionResult = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(serde_json::to_value(&p).unwrap(), v);
    }
}

#[test]
fn user_input_matches_what_hosts_write() {
    // `{"type":"user","message":{"role":"user","content":...},"uuid":...}`.
    assert_eq!(
        UserInput::text("hi").with_uuid("u1").to_value().unwrap(),
        json!({"type": "user", "message": {"role": "user", "content": "hi"}, "uuid": "u1"})
    );
    assert_eq!(
        UserInput::blocks(vec![ContentBlock::text("a")])
            .with_session_id("s")
            .with_parent_tool_use_id("t")
            .to_value()
            .unwrap(),
        json!({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": "a"}]},
            "parent_tool_use_id": "t", "session_id": "s"})
    );
    // It reads back as a user message.
    let i = round_trip(&UserInput::from("x").to_value().unwrap());
    assert!(matches!(Message::try_from(i), Ok(Message::User(_))));
}

#[test]
fn split_and_back() {
    let v = json!({"type": "stream_event", "event": {"type": "message_stop"}});
    let i = round_trip(&v);
    let Frame::Message(Message::StreamEvent(e)) = i.clone().split() else {
        panic!("not a stream event")
    };
    assert_eq!(e.event_type(), Some("message_stop"));
    assert_eq!(Inbound::from(i.clone().split()), i);
    let c = parse(&json!({"type": "control_cancel_request", "request_id": "r"}));
    assert!(Message::try_from(c).is_err());
}

#[test]
fn other_and_malformed_frames_are_counted() {
    use ccwire::protocol::stats;
    let before = stats::counts();
    let other = Inbound::from_line(r#"{"type":"brand_new","x":1}"#).unwrap();
    assert!(matches!(other, Inbound::Other { .. }));
    let bad = Inbound::from_line(r#"{"type":"assistant","message":7}"#).unwrap();
    assert!(matches!(bad, Inbound::Malformed { .. }));
    let after = stats::counts();
    // Other tests parse in parallel: at least these.
    assert!(after.other > before.other);
    assert!(after.malformed > before.malformed);
}

#[test]
fn the_new_types_parse_typed() {
    use ccwire::protocol::{Message, System};
    let lc = Inbound::from_line(
        r#"{"type":"command_lifecycle","command_uuid":"u1","state":"queued","session_id":"s1"}"#,
    )
    .unwrap();
    let Inbound::CommandLifecycle(c) = &lc else {
        panic!("{lc:?}")
    };
    assert_eq!(
        (c.command_uuid.as_deref(), c.state.as_deref()),
        (Some("u1"), Some("queued"))
    );
    assert!(matches!(
        Message::try_from(lc).unwrap(),
        Message::CommandLifecycle(_)
    ));

    let st = Inbound::from_line(r#"{"type":"system","subtype":"status","status":"compacting"}"#)
        .unwrap();
    let Inbound::System(System::Status(st)) = st else {
        panic!("{st:?}")
    };
    assert_eq!(st.status.as_deref(), Some("compacting"));

    let bt = Inbound::from_line(
        r#"{"type":"system","subtype":"background_tasks_changed","tasks":[{"task_id":"t1","task_type":"local_agent","description":"Find bugs"},"junk"]}"#,
    )
    .unwrap();
    let Inbound::System(System::BackgroundTasksChanged(bt)) = bt else {
        panic!("{bt:?}")
    };
    let ids: Vec<_> = bt.task_objects().map(|t| t["task_id"].clone()).collect();
    assert_eq!(ids, ["t1"]);

    let ts = Inbound::from_line(
        r#"{"type":"system","subtype":"task_started","task_id":"bc","owned_by_subagent":true}"#,
    )
    .unwrap();
    let Inbound::System(System::TaskStarted(ts)) = ts else {
        panic!("{ts:?}")
    };
    assert!(ts.subagent_owned());

    let se = Inbound::from_line(
        r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}},"parent_tool_use_id":null,"api_message_id":"msg_5"}"#,
    )
    .unwrap();
    let Inbound::StreamEvent(se) = se else {
        panic!("{se:?}")
    };
    assert_eq!(se.api_message_id.as_deref(), Some("msg_5"));
    assert!(!se.is_subagent());
    assert_eq!(
        se.partial(),
        Some(ccwire::protocol::Partial::TextDelta {
            index: 0,
            text: "Hi".into()
        })
    );
}
