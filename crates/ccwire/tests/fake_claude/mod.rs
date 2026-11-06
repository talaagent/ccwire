//! A fake `claude` CLI over in-memory pipes ([`tokio::io::duplex`]), for
//! testing a [`Session`] with no real CLI.
//!
//! A test scripts the CLI side step by step: read what the session wrote
//! ([`FakeClaude::recv`] and friends) and write frames back
//! ([`FakeClaude::emit`]). [`FakeClaude::serve`] is a canned CLI that answers
//! every user message with a whole turn and every control request with
//! `success`.

#![allow(dead_code, reason = "each test file uses part of the harness")]

use ccwire::{
    framing::AsyncLineReader,
    session::{Session, SessionBuilder},
};
use serde_json::{Value, json};
use tokio::io::{AsyncWriteExt as _, DuplexStream, duplex};

/// `frame` as compact JSON with its `type` key first.
pub fn encode(frame: &Value) -> Vec<u8> {
    let Value::Object(obj) = frame else {
        return serde_json::to_vec(frame).expect("serializes");
    };
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort_by_key(|k| *k != "type");
    let fields: Vec<String> = keys
        .iter()
        .map(|k| format!("{}:{}", Value::from(k.as_str()), obj[k.as_str()]))
        .collect();
    format!("{{{}}}", fields.join(",")).into_bytes()
}

/// The pipes' buffer size.
const PIPE: usize = 64 * 1024;

/// The CLI side of a [`Session`].
pub struct FakeClaude {
    /// What the session writes (the CLI's stdin).
    input: AsyncLineReader<DuplexStream>,
    /// What the CLI writes (its stdout); `None` once it exited.
    output: Option<DuplexStream>,
}

/// A session started by `builder`, and the fake CLI at its other end.
pub fn pair(builder: SessionBuilder) -> (Session, FakeClaude) {
    let (session_out, cli_stdout) = duplex(PIPE);
    let (cli_stdin, session_in) = duplex(PIPE);
    let session = builder.start(session_out, session_in);
    let cli = FakeClaude {
        input: AsyncLineReader::new(cli_stdin),
        output: Some(cli_stdout),
    };
    (session, cli)
}

impl FakeClaude {
    /// The next frame the session wrote; `None` once it closed the input.
    pub async fn recv(&mut self) -> Option<Value> {
        let line = self.input.next_line().await.expect("a line")?;
        Some(serde_json::from_slice(&line.bytes).expect("the session writes JSON"))
    }

    /// The next frame, which must be there.
    pub async fn recv_some(&mut self) -> Value {
        self.recv().await.expect("a frame, not EOF")
    }

    /// The next frame, a user message: its text.
    pub async fn recv_user(&mut self) -> String {
        let v = self.recv_some().await;
        assert_eq!(v["type"], "user", "{v}");
        assert_eq!(v["message"]["role"], "user", "{v}");
        v["message"]["content"]
            .as_str()
            .expect("text content")
            .to_owned()
    }

    /// The next frame, a control request: its id and `request`.
    pub async fn recv_control(&mut self) -> (String, Value) {
        let v = self.recv_some().await;
        assert_eq!(v["type"], "control_request", "{v}");
        let id = v["request_id"].as_str().expect("a request_id").to_owned();
        (id, v["request"].clone())
    }

    /// The next frame, a control response: its `response`.
    pub async fn recv_response(&mut self) -> Value {
        let v = self.recv_some().await;
        assert_eq!(v["type"], "control_response", "{v}");
        v["response"].clone()
    }

    /// The session closed the input: EOF.
    pub async fn expect_eof(&mut self) {
        if let Some(v) = self.recv().await {
            panic!("expected EOF, got {v}");
        }
    }

    /// Write raw bytes to the session.
    pub async fn emit_raw(&mut self, bytes: &[u8]) {
        self.try_emit_raw(bytes).await.expect("the session reads");
    }

    /// Write raw bytes; an error once the session's reader is gone.
    pub async fn try_emit_raw(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        let out = self.output.as_mut().expect("the CLI hasn't exited");
        out.write_all(bytes).await?;
        out.flush().await
    }

    /// Write one frame, `type` first, as the CLI does.
    pub async fn emit(&mut self, frame: Value) {
        let mut line = encode(&frame);
        line.push(b'\n');
        self.emit_raw(&line).await;
    }

    /// Exit: the session reads EOF.
    pub fn exit(&mut self) {
        self.output = None;
    }

    pub async fn init(&mut self, session_id: &str) {
        self.emit(
            json!({"type": "system", "subtype": "init", "session_id": session_id,
            "model": "claude-test", "tools": ["Bash"], "cwd": "/tmp"}),
        )
        .await;
    }

    pub async fn assistant(&mut self, text: &str) {
        self.emit(
            json!({"type": "assistant", "message": {"id": "msg_1", "role": "assistant",
            "model": "claude-test", "content": [{"type": "text", "text": text}]},
            "parent_tool_use_id": null, "session_id": "s"}),
        )
        .await;
    }

    pub async fn result(&mut self, text: &str) {
        self.emit(
            json!({"type": "result", "subtype": "success", "is_error": false,
            "result": text, "num_turns": 1, "total_cost_usd": 0.01, "session_id": "s"}),
        )
        .await;
    }

    /// The result of an interrupted turn.
    pub async fn result_interrupted(&mut self) {
        self.emit(
            json!({"type": "result", "subtype": "error_during_execution",
            "is_error": true, "errors": ["interrupted"], "session_id": "s"}),
        )
        .await;
    }

    /// Answer a control request with `success`.
    pub async fn reply_ok(&mut self, request_id: &str, response: Option<Value>) {
        let mut body = json!({"subtype": "success", "request_id": request_id});
        if let Some(r) = response {
            body["response"] = r;
        }
        self.emit(json!({"type": "control_response", "response": body}))
            .await;
    }

    /// Answer a control request with `error`.
    pub async fn reply_err(&mut self, request_id: &str, error: &str) {
        self.emit(json!({"type": "control_response", "response":
            {"subtype": "error", "request_id": request_id, "error": error}}))
            .await;
    }

    /// Ask the session whether a tool may run.
    pub async fn can_use_tool(&mut self, request_id: &str, tool: &str, input: Value) {
        self.emit(json!({"type": "control_request", "request_id": request_id,
            "request": {"subtype": "can_use_tool", "tool_name": tool, "input": input,
                "tool_use_id": "toolu_1"}}))
            .await;
    }

    /// Take back a control request.
    pub async fn cancel(&mut self, request_id: &str) {
        self.emit(json!({"type": "control_cancel_request", "request_id": request_id}))
            .await;
    }

    /// A canned CLI, until the session closes the input: a user message
    /// gets `system/init`, an assistant message echoing it and a `result`;
    /// `initialize` gets `{"commands": []}`, any other control request a
    /// bare `success`. Then it exits.
    pub async fn serve(mut self) {
        while let Some(v) = self.recv().await {
            match v["type"].as_str() {
                Some("user") => {
                    let text = v["message"]["content"].as_str().unwrap_or_default();
                    let echo = format!("echo: {text}");
                    self.init("s1").await;
                    self.assistant(&echo).await;
                    self.result(&echo).await;
                }
                Some("control_request") => {
                    let id = v["request_id"].as_str().expect("a request_id").to_owned();
                    let payload =
                        (v["request"]["subtype"] == "initialize").then(|| json!({"commands": []}));
                    self.reply_ok(&id, payload).await;
                }
                _ => {}
            }
        }
        self.exit();
    }
}
