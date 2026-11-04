//! The session core against a scripted fake CLI ([`fake_claude`]): no real
//! `claude` needed.

#![cfg(feature = "runtime")]
#![allow(
    clippy::disallowed_methods,
    reason = "#[tokio::test] expands to the runtime's block_on"
)]
#![allow(
    clippy::unwrap_used,
    clippy::cognitive_complexity,
    reason = "test bodies, also those on_both_runtimes! runs from plain fns"
)]

extern crate alloc;

mod fake_claude;

use core::{future::Future, time::Duration};

use ccwire::{
    protocol::{
        ControlRequest, ControlRequestBody, ControlResponse, Inbound, InitializeRequest,
        PermissionResult, System,
    },
    session::{ControlFuture, Error, Session, SessionState},
};
use fake_claude::pair;
use serde_json::{Map, json};
use tokio::sync::oneshot;

/// Run a test body, failing it if it hangs.
async fn within<T>(fut: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), fut)
        .await
        .expect("the test hung")
}

/// The next frame, which must be there and not an error.
async fn next(s: &mut Session) -> Inbound {
    match s.next().await {
        Some(Ok(frame)) => frame,
        other => panic!("expected a frame, got {other:?}"),
    }
}

/// A test body run on both runtimes: `#[tokio::test]` (current-thread)
/// and a multi-thread one, where the reader, the writer and the handlers
/// really run in parallel.
macro_rules! on_both_runtimes {
    ($name:ident, $body:ident) => {
        mod $name {
            #[tokio::test]
            async fn current_thread() {
                super::within(super::$body()).await;
            }

            #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
            async fn multi_thread() {
                super::within(super::$body()).await;
            }
        }
    };
}

fn result_text(frame: &Inbound) -> Option<&str> {
    match frame {
        Inbound::Result(r) => r.result_text(),
        _ => None,
    }
}

#[tokio::test]
async fn a_normal_turn() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder());
        assert_eq!(s.state(), SessionState::Idle);
        s.send("hi").await.unwrap();
        assert_eq!(s.state(), SessionState::InTurn);
        assert_eq!(cli.recv_user().await, "hi");

        cli.init("sess-1").await;
        cli.assistant("hello").await;
        cli.result("hello").await;
        assert!(matches!(
            next(&mut s).await,
            Inbound::System(System::Init(_))
        ));
        let Inbound::Assistant(a) = next(&mut s).await else {
            panic!("not an assistant message")
        };
        assert_eq!(a.content()[0].as_text(), Some("hello"));
        let r = next(&mut s).await;
        assert!(r.ends_turn());
        assert_eq!(result_text(&r), Some("hello"));
        assert_eq!(s.state(), SessionState::Idle);
        assert_eq!(s.session_id().as_deref(), Some("sess-1"));

        // No callbacks: finish closes the input at once.
        s.finish();
        cli.expect_eof().await;
        cli.exit();
        assert!(s.next().await.is_none(), "a clean EOF between turns");
        assert_eq!(s.state(), SessionState::Closed);
    })
    .await;
}

#[tokio::test]
async fn a_multi_turn_session() {
    within(async {
        let (mut s, cli) = pair(Session::builder());
        let cli = tokio::spawn(cli.serve());
        let mut states = s.watch_state();
        for text in ["one", "two", "three"] {
            s.send(text).await.unwrap();
            let r = loop {
                let f = next(&mut s).await;
                if f.ends_turn() {
                    break f;
                }
            };
            assert_eq!(result_text(&r), Some(format!("echo: {text}").as_str()));
            assert!(s.is_idle());
        }
        states.changed().await.unwrap();
        assert_eq!(*states.borrow_and_update(), SessionState::Idle);
        s.close().await.unwrap();
        cli.await.unwrap();
    })
    .await;
}

on_both_runtimes!(a_control_round_trip, control_round_trip);

async fn control_round_trip() {
    let (mut s, mut cli) = pair(Session::builder());

    // Success, with a payload.
    let (answer, ()) = tokio::join!(s.request(InitializeRequest::default()), async {
        let (id, req) = cli.recv_control().await;
        assert_eq!(req["subtype"], "initialize");
        cli.reply_ok(&id, Some(json!({"commands": []}))).await;
    });
    assert_eq!(answer.unwrap(), Some(json!({"commands": []})));

    // An error answer.
    let (answer, ()) = tokio::join!(s.set_model(Some("opus")), async {
        let (id, req) = cli.recv_control().await;
        assert_eq!(req, json!({"subtype": "set_model", "model": "opus"}));
        cli.reply_err(&id, "no such model").await;
    });
    assert!(
        matches!(answer, Err(Error::Control { ref message, .. }) if message == "no such model"),
        "{answer:?}"
    );

    // The escape hatch, and distinct ids.
    let mut fields = Map::new();
    fields.insert("x".into(), json!(1));
    let (answer, ids) = tokio::join!(s.request_raw("brand_new", fields), async {
        let (id, req) = cli.recv_control().await;
        assert_eq!(req, json!({"subtype": "brand_new", "x": 1}));
        cli.reply_ok(&id, None).await;
        id
    });
    assert_eq!(answer.unwrap(), None);
    // `req_{n}_{nonce}`, the nonce the session's own.
    let nonce = ids.strip_prefix("req_3_").expect("req_3_<nonce>");
    assert_eq!(nonce.len(), 8, "{ids}");
    assert!(nonce.bytes().all(|b| b.is_ascii_hexdigit()), "{ids}");

    // The answered responses never reached the stream; a stray one does.
    cli.reply_ok("nobody", None).await;
    let Inbound::ControlResponse(r) = next(&mut s).await else {
        panic!("not the stray response")
    };
    assert_eq!(r.request_id(), Some("nobody"));
    assert!(s.is_idle());

    // A later session numbers its requests from 1 again, with another
    // nonce: an answer to the first session's `req_1` doesn't resolve
    // its `req_1`.
    let (mut s2, mut cli2) = pair(Session::builder());
    let (answer, ()) = tokio::join!(s2.interrupt(), async {
        let (id, _) = cli2.recv_control().await;
        assert!(
            id.starts_with("req_1_") && id != format!("req_1_{nonce}"),
            "{id}"
        );
        cli2.reply_ok(&format!("req_1_{nonce}"), None).await;
        cli2.reply_ok(&id, None).await;
    });
    answer.unwrap();
    let Inbound::ControlResponse(r) = next(&mut s2).await else {
        panic!("not the other session's response")
    };
    assert_eq!(r.request_id(), Some(format!("req_1_{nonce}").as_str()));
}

on_both_runtimes!(
    a_control_request_answered_from_the_stream,
    control_request_answered_from_the_stream
);

async fn control_request_answered_from_the_stream() {
    let (mut s, mut cli) = pair(Session::builder().answers_control_requests(true));
    s.send("go").await.unwrap();
    cli.recv_user().await;
    cli.can_use_tool("c1", "Bash", json!({"command": "ls"}))
        .await;
    let Inbound::ControlRequest(req) = next(&mut s).await else {
        panic!("not the request")
    };
    let ControlRequestBody::CanUseTool(ask) = &req.request else {
        panic!("not can_use_tool")
    };
    assert_eq!(ask.tool_name, "Bash");

    // With callbacks, finish waits for idle.
    s.finish();
    let allow = PermissionResult::allow(ask.input.clone());
    let resp = ControlResponse::permission(&req.request_id, &allow).unwrap();
    assert!(s.respond(resp.clone()).await.unwrap());
    assert!(!s.respond(resp).await.unwrap(), "answered already");
    let answer = cli.recv_response().await;
    assert_eq!(
        answer,
        json!({"subtype": "success", "request_id": "c1",
            "response": {"behavior": "allow", "updatedInput": {"command": "ls"}}})
    );

    // Cancelled before the answer: nothing is written.
    cli.can_use_tool("c2", "Bash", json!({})).await;
    cli.cancel("c2").await;
    assert!(matches!(next(&mut s).await, Inbound::ControlRequest(_)));
    assert!(matches!(
        next(&mut s).await,
        Inbound::ControlCancelRequest(_)
    ));
    let deny = ControlResponse::permission("c2", &PermissionResult::deny("no")).unwrap();
    assert!(!s.respond(deny).await.unwrap());

    // The result makes it idle: the input closes.
    cli.result("done").await;
    assert!(next(&mut s).await.ends_turn());
    cli.expect_eof().await;
}

/// Sends `()` when dropped.
struct DropFlag(Option<oneshot::Sender<()>>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

on_both_runtimes!(
    a_control_handler_answers_and_holds_the_run_open,
    control_handler_answers_and_holds_the_run_open
);

async fn control_handler_answers_and_holds_the_run_open() {
    let handler = |req: ControlRequest| -> ControlFuture {
        Box::pin(async move {
            let ControlRequestBody::CanUseTool(ask) = req.request else {
                return Err("unsupported".to_owned());
            };
            let r = if ask.tool_name == "Bash" {
                PermissionResult::allow(ask.input)
            } else {
                PermissionResult::deny("not that one")
            };
            Ok(Some(serde_json::to_value(r).map_err(|e| e.to_string())?))
        })
    };
    let (mut s, mut cli) = pair(Session::builder().control_handler(handler));
    s.send("go").await.unwrap();
    // The run-end rule: with a handler, the input stays open until idle.
    s.finish();
    assert_eq!(cli.recv_user().await, "go");

    cli.can_use_tool("c1", "Bash", json!({"command": "ls"}))
        .await;
    let r = cli.recv_response().await;
    assert_eq!(r["request_id"], "c1");
    assert_eq!(r["response"]["behavior"], "allow");
    cli.can_use_tool("c2", "Write", json!({})).await;
    let r = cli.recv_response().await;
    assert_eq!(r["response"]["behavior"], "deny");
    assert_eq!(r["response"]["message"], "not that one");
    cli.emit(json!({"type": "control_request", "request_id": "c3",
        "request": {"subtype": "hook_callback", "callback_id": "h", "input": {}}}))
        .await;
    let r = cli.recv_response().await;
    assert_eq!(
        r,
        json!({"subtype": "error", "request_id": "c3", "error": "unsupported"})
    );

    cli.result("done").await;
    // The handled requests never reached the stream.
    assert_eq!(result_text(&next(&mut s).await), Some("done"));
    cli.expect_eof().await;
}

on_both_runtimes!(a_cancel_aborts_the_handler, cancel_aborts_the_handler);

async fn cancel_aborts_the_handler() {
    let (started_tx, started) = oneshot::channel();
    let (dropped_tx, dropped) = oneshot::channel();
    let slot = std::sync::Mutex::new(Some((started_tx, dropped_tx)));
    let handler = move |_req: ControlRequest| -> ControlFuture {
        let (started_tx, dropped_tx) = slot.lock().unwrap().take().expect("one request");
        Box::pin(async move {
            let _flag = DropFlag(Some(dropped_tx));
            let _ = started_tx.send(());
            core::future::pending::<()>().await;
            Ok(None)
        })
    };
    let (mut s, mut cli) = pair(Session::builder().control_handler(handler));
    cli.can_use_tool("c1", "Bash", json!({})).await;
    started.await.unwrap();
    assert!(!s.is_idle());
    cli.cancel("c1").await;
    dropped.await.expect("the handler's task was aborted");
    cli.assistant("after").await;
    assert!(matches!(next(&mut s).await, Inbound::Assistant(_)));
    assert!(s.is_idle());
    s.finish();
    // No answer was written for the cancelled request.
    cli.expect_eof().await;
}

#[tokio::test]
async fn an_interrupt() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder());
        s.send("long task").await.unwrap();
        cli.recv_user().await;
        cli.assistant("working").await;
        let (r, ()) = tokio::join!(s.interrupt(), async {
            let (id, req) = cli.recv_control().await;
            assert_eq!(req, json!({"subtype": "interrupt"}));
            cli.reply_ok(&id, None).await;
            cli.result_interrupted().await;
        });
        r.unwrap();
        assert!(matches!(next(&mut s).await, Inbound::Assistant(_)));
        let Inbound::Result(r) = next(&mut s).await else {
            panic!("not the result")
        };
        assert!(r.failed());
        assert_eq!(s.state(), SessionState::Idle);
    })
    .await;
}

on_both_runtimes!(a_dropped_request_is_cancelled, dropped_request_is_cancelled);

async fn dropped_request_is_cancelled() {
    let (mut s, mut cli) = pair(Session::builder());
    let r = tokio::time::timeout(Duration::from_millis(50), s.interrupt()).await;
    assert!(r.is_err(), "nobody answered");
    assert!(s.is_idle(), "the pending request went with its future");
    let (id, _) = cli.recv_control().await;
    // The CLI is told.
    assert_eq!(
        cli.recv_some().await,
        json!({"type": "control_cancel_request", "request_id": id})
    );
    cli.reply_ok(&id, None).await;
    assert!(matches!(next(&mut s).await, Inbound::ControlResponse(_)));

    // An answered request sends no cancel.
    let (r, ()) = tokio::join!(s.interrupt(), async {
        let (id, _) = cli.recv_control().await;
        cli.reply_ok(&id, None).await;
    });
    r.unwrap();
    s.finish();
    cli.expect_eof().await;
}

#[tokio::test]
async fn garbage_lines_do_not_end_the_stream() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder());
        cli.emit_raw(b"not json at all\n").await;
        cli.emit_raw(b"[1, 2]\n\n   \n").await;
        cli.emit_raw(b"\xff\xfe\n").await;
        cli.emit(json!({"type": "assistant", "message": {"content": 7}}))
            .await;
        cli.emit(json!({"type": "brand_new", "x": 1})).await;
        cli.assistant("still here").await;

        let Inbound::Malformed { type_, error, raw } = next(&mut s).await else {
            panic!("not malformed")
        };
        assert_eq!(type_, "");
        assert!(error.starts_with("invalid JSON"), "{error}");
        assert_eq!(raw, json!("not json at all"));
        let Inbound::Malformed { error, .. } = next(&mut s).await else {
            panic!("not malformed")
        };
        assert_eq!(error, "not a JSON object");
        assert!(matches!(next(&mut s).await, Inbound::Malformed { .. }));
        assert!(
            matches!(next(&mut s).await, Inbound::Malformed { type_, .. } if type_ == "assistant")
        );
        assert!(matches!(next(&mut s).await, Inbound::Other { type_, .. } if type_ == "brand_new"));
        assert!(matches!(next(&mut s).await, Inbound::Assistant(_)));

        s.send("still works").await.unwrap();
        assert_eq!(cli.recv_user().await, "still works");
    })
    .await;
}

#[tokio::test]
async fn an_oversized_line_is_skipped_and_reported() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder().max_line_len(256));
        s.send("big").await.unwrap();
        cli.recv_user().await;
        let big = "x".repeat(10_000);
        cli.assistant(&big).await;
        cli.assistant("small").await;
        let Inbound::Malformed { type_, error, raw } = next(&mut s).await else {
            panic!("not malformed")
        };
        assert_eq!(type_, "assistant");
        assert!(
            error.contains("too long") && error.contains("256"),
            "{error}"
        );
        assert!(raw.as_str().is_some_and(|h| h.len() <= 1024));
        assert!(matches!(next(&mut s).await, Inbound::Assistant(_)));

        // A giant result still ends the turn.
        cli.result(&big).await;
        let r = next(&mut s).await;
        assert!(matches!(&r, Inbound::Malformed { .. }) && r.ends_turn());
        assert_eq!(s.state(), SessionState::Idle);
    })
    .await;
}

#[tokio::test]
async fn eof_mid_turn() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder());
        let h = s.handle();
        s.send("go").await.unwrap();
        cli.recv_user().await;
        cli.assistant("partial").await;
        let (r, ()) = tokio::join!(h.interrupt(), async {
            cli.recv_control().await;
            // Killed mid-line, before answering.
            cli.emit_raw(br#"{"type":"assistant","mess"#).await;
            cli.exit();
        });
        assert!(matches!(r, Err(Error::Closed)), "{r:?}");

        assert!(matches!(next(&mut s).await, Inbound::Assistant(_)));
        let cut = next(&mut s).await;
        assert!(matches!(&cut, Inbound::Malformed { type_, .. } if type_ == "assistant"));
        assert!(matches!(s.next().await, Some(Err(Error::EndedMidTurn))));
        assert!(s.next().await.is_none());
        assert_eq!(s.state(), SessionState::Closed);
        assert!(matches!(s.send("again").await, Err(Error::InputClosed)));
        cli.expect_eof().await;
    })
    .await;
}

#[tokio::test]
async fn a_drifted_result_still_ends_the_turn() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder().answers_control_requests(true));
        s.send("go").await.unwrap();
        s.finish();
        cli.recv_user().await;
        cli.emit(json!({"type": "result", "subtype": "success", "num_turns": "three"}))
            .await;
        let r = next(&mut s).await;
        assert!(
            matches!(&r, Inbound::Malformed { type_, .. } if type_ == "result"),
            "{r:?}"
        );
        assert_eq!(s.state(), SessionState::Idle);
        // Idle, so the finishing run closes the input.
        cli.expect_eof().await;
        cli.exit();
        assert!(s.next().await.is_none(), "not mid-turn");
    })
    .await;
}

#[tokio::test]
async fn offsets_from_the_start_offset() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder().start_offset(1000));
        cli.emit_raw(b"\n").await;
        cli.assistant("a").await;
        let Some(Ok((_, end))) = s.next_at().await else {
            panic!("no frame")
        };
        let mut line = fake_claude::encode(&json!({"type": "assistant", "message": {"id": "msg_1",
            "role": "assistant", "model": "claude-test", "content": [{"type": "text", "text": "a"}]},
            "parent_tool_use_id": null, "session_id": "s"}));
        line.push(b'\n');
        assert_eq!(end, 1000 + 1 + line.len() as u64);
    })
    .await;
}

#[tokio::test]
async fn close_ends_gracefully() {
    within(async {
        let (s, cli) = pair(Session::builder());
        let cli = tokio::spawn(cli.serve());
        let (_, answer) = tokio::join!(cli, async {
            let r = s.request(InitializeRequest::default()).await;
            s.close().await.unwrap();
            r
        });
        assert_eq!(answer.unwrap(), Some(json!({"commands": []})));
    })
    .await;
}

#[tokio::test]
async fn dropping_the_session_cleans_up() {
    within(async {
        let (started_tx, started) = oneshot::channel();
        let (dropped_tx, dropped) = oneshot::channel();
        let slot = std::sync::Mutex::new(Some((started_tx, dropped_tx)));
        let handler = move |_req: ControlRequest| -> ControlFuture {
            let (started_tx, dropped_tx) = slot.lock().unwrap().take().expect("one request");
            Box::pin(async move {
                let _flag = DropFlag(Some(dropped_tx));
                let _ = started_tx.send(());
                core::future::pending::<()>().await;
                Ok(None)
            })
        };
        let (s, mut cli) = pair(Session::builder().control_handler(handler));
        let h = s.handle();
        cli.can_use_tool("c1", "Bash", json!({})).await;
        started.await.unwrap();
        drop(s);

        dropped.await.expect("the handler's task was aborted");
        // The writer task is gone: the CLI's input ends, with no answer.
        cli.expect_eof().await;
        // The reader task is gone: writing to it fails.
        while cli.try_emit_raw(b"{}\n").await.is_ok() {
            tokio::task::yield_now().await;
        }
        assert_eq!(h.state(), SessionState::Closed);
        assert!(h.send("late").await.is_err());
        assert!(h.interrupt().await.is_err());
    })
    .await;
}

/// `frames` as the CLI writes them, and their length in bytes.
fn transcript(frames: &[serde_json::Value]) -> (Vec<u8>, u64) {
    let mut out = Vec::new();
    for f in frames {
        out.extend(fake_claude::encode(f));
        out.push(b'\n');
    }
    let len = out.len() as u64;
    (out, len)
}

fn init(id: &str) -> serde_json::Value {
    json!({"type": "system", "subtype": "init", "session_id": id})
}

fn can_use_tool(id: &str) -> serde_json::Value {
    json!({"type": "control_request", "request_id": id,
        "request": {"subtype": "can_use_tool", "tool_name": "Bash", "input": {},
            "tool_use_id": "toolu_1"}})
}

fn lifecycle(uuid: &str, state: &str) -> serde_json::Value {
    json!({"type": "command_lifecycle", "command_uuid": uuid, "state": state,
        "session_id": "s"})
}

#[tokio::test]
async fn a_reattached_turn_replays_without_side_effects() {
    within(async {
        let calls = alloc::sync::Arc::new(core::sync::atomic::AtomicUsize::new(0));
        let in_handler = alloc::sync::Arc::clone(&calls);
        let handler = move |_req: ControlRequest| -> ControlFuture {
            in_handler.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
            Box::pin(async { Ok(Some(json!({"behavior": "allow", "updatedInput": {}}))) })
        };
        // What the CLI wrote while an earlier reader followed it: a turn
        // started, a permission asked (and answered by that reader), one
        // asked and taken back.
        let (earlier, upto) = transcript(&[
            init("sess-1"),
            json!({"type": "assistant", "message": {"content": []}}),
            can_use_tool("old_1"),
            can_use_tool("old_2"),
            json!({"type": "control_cancel_request", "request_id": "old_2"}),
        ]);
        let (mut s, mut cli) = pair(
            Session::builder()
                .control_handler(handler)
                .replay_until(upto),
        );
        cli.emit_raw(&earlier).await;
        // Replayed: out of the stream, none answered, none open.
        let mut replayed = 0;
        while replayed < 5 {
            let Some(Ok((_, at))) = s.next_at().await else {
                panic!("a replayed frame")
            };
            assert!(at <= upto);
            replayed += 1;
        }
        assert_eq!(s.state(), SessionState::InTurn, "the replayed init");
        assert_eq!(s.session_id().as_deref(), Some("sess-1"));
        assert_eq!(calls.load(core::sync::atomic::Ordering::SeqCst), 0);

        // Live: answered.
        cli.can_use_tool("new_1", "Bash", json!({})).await;
        let r = cli.recv_response().await;
        assert_eq!(r["request_id"], "new_1", "no answer to a replayed request");
        assert_eq!(calls.load(core::sync::atomic::Ordering::SeqCst), 1);

        // The CLI dies before the turn's result: the replayed turn counts.
        cli.exit();
        assert!(matches!(s.next().await, Some(Err(Error::EndedMidTurn))));
        assert!(s.next().await.is_none());
    })
    .await;
}

#[tokio::test]
async fn a_replayed_request_does_not_hold_the_run_open() {
    within(async {
        let (earlier, upto) = transcript(&[
            init("sess-1"),
            can_use_tool("old_1"),
            json!({"type": "result", "subtype": "success", "is_error": false}),
            init("sess-1"),
        ]);
        let (mut s, mut cli) = pair(
            Session::builder()
                .answers_control_requests(true)
                .replay_until(upto),
        );
        cli.emit_raw(&earlier).await;
        for _ in 0..4 {
            let Some(Ok((_, at))) = s.next_at().await else {
                panic!("a replayed frame")
            };
            assert!(at <= upto);
        }
        // Not open: answering it writes nothing.
        let resp = ControlResponse::permission("old_1", &PermissionResult::deny("late")).unwrap();
        assert!(!s.respond(resp).await.unwrap());
        assert_eq!(s.state(), SessionState::InTurn);
        s.finish();

        // The live result makes it idle: the run ends.
        cli.result("done").await;
        assert!(next(&mut s).await.ends_turn());
        assert!(s.is_idle());
        cli.expect_eof().await;
    })
    .await;
}

/// The uuid of the user message the session wrote.
async fn recv_uuid(cli: &mut fake_claude::FakeClaude) -> String {
    let v = cli.recv_some().await;
    assert_eq!(v["type"], "user", "{v}");
    v["uuid"].as_str().expect("a uuid").to_owned()
}

#[tokio::test]
async fn a_queued_message_keeps_the_run_open() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder().answers_control_requests(true));
        s.send("one").await.unwrap();
        let one = recv_uuid(&mut cli).await;
        cli.init("s").await;
        cli.emit(lifecycle(&one, "started")).await;
        // Sent mid-turn; the CLI holds it for the next turn.
        s.send("two").await.unwrap();
        let two = recv_uuid(&mut cli).await;
        assert_ne!(one, two);
        cli.emit(lifecycle(&two, "queued")).await;
        s.finish();
        cli.result("one done").await;
        cli.emit(lifecycle(&one, "completed")).await;
        for _ in 0..4 {
            next(&mut s).await;
        }
        assert_eq!(s.state(), SessionState::InTurn, "`two` is still to run");

        // Its turn.
        cli.init("s").await;
        cli.emit(lifecycle(&two, "started")).await;
        cli.result("two done").await;
        cli.emit(lifecycle(&two, "completed")).await;
        for _ in 0..4 {
            next(&mut s).await;
        }
        assert!(s.is_idle());
        cli.expect_eof().await;
    })
    .await;
}

#[tokio::test]
async fn an_absorbed_message_ends_with_its_turn() {
    within(async {
        let (mut s, mut cli) = pair(Session::builder().answers_control_requests(true));
        s.send("one").await.unwrap();
        let one = recv_uuid(&mut cli).await;
        cli.init("s").await;
        cli.emit(lifecycle(&one, "started")).await;
        s.send("and two").await.unwrap();
        let two = recv_uuid(&mut cli).await;
        // Taken in at the next tool boundary, answered in this result.
        cli.emit(lifecycle(&two, "started")).await;
        s.finish();
        cli.result("both done").await;
        for _ in 0..4 {
            next(&mut s).await;
        }
        assert!(s.is_idle(), "no completed needed once the turn ended");
        cli.expect_eof().await;
    })
    .await;
}

#[tokio::test]
async fn a_failed_send_starts_no_turn() {
    within(async {
        let (session_out, _cli_stdout) = tokio::io::duplex(1024);
        let (cli_stdin, session_in) = tokio::io::duplex(1024);
        drop(cli_stdin);
        let s = Session::builder()
            .answers_control_requests(true)
            .start(session_out, session_in);
        let r = s.send("lost").await;
        assert!(matches!(r, Err(Error::Write(_))), "{r:?}");
        assert_eq!(s.state(), SessionState::Idle);
        assert!(s.is_idle());
        assert!(matches!(s.send("again").await, Err(Error::InputClosed)));
    })
    .await;
}

#[tokio::test]
async fn an_answer_the_input_cannot_take_is_reported() {
    within(async {
        let (release_tx, release) = oneshot::channel::<()>();
        let slot = std::sync::Mutex::new(Some(release));
        let handler = move |_req: ControlRequest| -> ControlFuture {
            let release = slot.lock().unwrap().take().expect("one request");
            Box::pin(async move {
                let _ = release.await;
                Ok(None)
            })
        };
        let (mut s, mut cli) = pair(Session::builder().control_handler(handler));
        cli.can_use_tool("c1", "Bash", json!({})).await;
        // Wait until the request is open, then close the input under it.
        while s.is_idle() {
            tokio::task::yield_now().await;
        }
        s.end_input();
        cli.expect_eof().await;
        release_tx.send(()).unwrap();
        assert!(matches!(s.next().await, Some(Err(Error::InputClosed))));
        assert!(s.is_idle());
    })
    .await;
}
