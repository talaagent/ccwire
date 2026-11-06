//! `stream_event` deltas (`--include-partial-messages`): [`Partial`] reads
//! one event, [`PartialMessage`] assembles a message's text as it streams.

use alloc::collections::BTreeMap;

use serde_json::Value;

use super::StreamEvent;

/// What a reply streamer reads of a `stream_event`'s `event`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Partial {
    /// `message_start`: a new API message.
    MessageStart,
    /// `content_block_start`: block `index` starts, of `kind` (`text`,
    /// `tool_use`, `thinking`, `server_tool_use`, ...; empty if missing).
    BlockStart { index: u64, kind: String },
    /// `content_block_delta` with a `text_delta`: text appended to text
    /// block `index`.
    TextDelta { index: u64, text: String },
    /// `content_block_stop`.
    BlockStop { index: u64 },
    /// `message_delta`: the message's stop reason (`end_turn`, `tool_use`,
    /// `max_tokens`, ...), when it has one.
    MessageDelta { stop_reason: Option<String> },
    /// `message_stop`.
    MessageStop,
}

impl Partial {
    /// Read an API stream event; `None` for the ones a reply streamer
    /// doesn't need (deltas other than text, `ping`, unknown types). A
    /// missing `index` reads as 0.
    pub fn of_event(e: &Value) -> Option<Self> {
        let index = || e.get("index").and_then(Value::as_u64).unwrap_or(0);
        let s = |v: Option<&Value>| v.and_then(Value::as_str).map(str::to_owned);
        Some(match e.get("type").and_then(Value::as_str)? {
            "message_start" => Self::MessageStart,
            "content_block_start" => Self::BlockStart {
                index: index(),
                kind: s(e.pointer("/content_block/type")).unwrap_or_default(),
            },
            "content_block_delta" => {
                if e.pointer("/delta/type").and_then(Value::as_str) != Some("text_delta") {
                    return None;
                }
                Self::TextDelta {
                    index: index(),
                    text: s(e.pointer("/delta/text"))?,
                }
            }
            "content_block_stop" => Self::BlockStop { index: index() },
            "message_delta" => Self::MessageDelta {
                stop_reason: s(e.pointer("/delta/stop_reason")),
            },
            "message_stop" => Self::MessageStop,
            _ => return None,
        })
    }
}

/// One API message's text, assembled from its `stream_event`s.
///
/// Feed it every `stream_event` of one thread (the main one, or one
/// subagent's: [`StreamEvent::parent_tool_use_id`]); a `message_start`, or
/// an event of another `api_message_id`, starts it over.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartialMessage {
    /// The message's `api_message_id`, when the events carry it.
    pub api_message_id: Option<String>,
    /// Its text blocks so far, by index.
    pub texts: BTreeMap<u64, String>,
    /// The blocks that are done (`content_block_stop`).
    pub stopped: Vec<u64>,
    /// Its stop reason, once `message_delta` gave it.
    pub stop_reason: Option<String>,
    /// `message_stop` came: the whole `assistant` message follows.
    pub done: bool,
}

impl PartialMessage {
    /// Take in one `stream_event`. Returns what it read, if anything.
    pub fn push(&mut self, ev: &StreamEvent) -> Option<Partial> {
        let id = ev.api_message_id.get();
        if id.is_some() && id != self.api_message_id.as_ref() {
            *self = Self {
                api_message_id: id.cloned(),
                ..Self::default()
            };
        }
        let p = ev.partial()?;
        self.apply(&p);
        Some(p)
    }

    /// Take in one [`Partial`].
    pub fn apply(&mut self, p: &Partial) {
        match p {
            Partial::MessageStart => {
                let id = self.api_message_id.take();
                *self = Self {
                    api_message_id: id,
                    ..Self::default()
                };
            }
            Partial::BlockStart { index, kind } if kind == "text" => {
                self.texts.entry(*index).or_default();
            }
            Partial::BlockStart { .. } => {}
            Partial::TextDelta { index, text } => {
                self.texts.entry(*index).or_default().push_str(text);
            }
            Partial::BlockStop { index } => self.stopped.push(*index),
            Partial::MessageDelta { stop_reason } => {
                if stop_reason.is_some() {
                    self.stop_reason.clone_from(stop_reason);
                }
            }
            Partial::MessageStop => self.done = true,
        }
    }

    /// The text so far: the text blocks in order, joined by a blank line.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for t in self.texts.values().filter(|t| !t.is_empty()) {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(t);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn se(event: Value, msg: &str) -> StreamEvent {
        StreamEvent {
            event,
            api_message_id: msg.to_owned().into(),
            ..StreamEvent::default()
        }
    }

    #[test]
    fn reads_the_events_a_streamer_needs() {
        let of = |v: Value| Partial::of_event(&v);
        assert_eq!(
            of(json!({"type": "message_start", "message": {}})),
            Some(Partial::MessageStart)
        );
        assert_eq!(
            of(json!({"type": "content_block_start", "index": 1,
                      "content_block": {"type": "tool_use"}})),
            Some(Partial::BlockStart {
                index: 1,
                kind: "tool_use".into()
            })
        );
        assert_eq!(
            of(json!({"type": "content_block_delta", "index": 0,
                      "delta": {"type": "text_delta", "text": "Hi"}})),
            Some(Partial::TextDelta {
                index: 0,
                text: "Hi".into()
            })
        );
        assert_eq!(
            of(json!({"type": "content_block_delta", "index": 0,
                      "delta": {"type": "input_json_delta", "partial_json": "{"}})),
            None
        );
        assert_eq!(
            of(json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}})),
            Some(Partial::MessageDelta {
                stop_reason: Some("end_turn".into())
            })
        );
        assert_eq!(of(json!({"type": "ping"})), None);
        assert_eq!(of(json!({"no": "type"})), None);
    }

    #[test]
    fn assembles_a_message_and_starts_over_on_the_next() {
        let mut m = PartialMessage::default();
        for e in [
            json!({"type": "message_start", "message": {}}),
            json!({"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Hi"}}),
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": " there"}}),
            json!({"type": "content_block_stop", "index": 0}),
            json!({"type": "content_block_start", "index": 1, "content_block": {"type": "tool_use"}}),
            json!({"type": "content_block_start", "index": 2, "content_block": {"type": "text", "text": ""}}),
            json!({"type": "content_block_delta", "index": 2, "delta": {"type": "text_delta", "text": "Done."}}),
            json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}}),
            json!({"type": "message_stop"}),
        ] {
            m.push(&se(e, "msg_1"));
        }
        assert_eq!(m.text(), "Hi there\n\nDone.");
        assert_eq!(m.stopped, [0]);
        assert_eq!(m.stop_reason.as_deref(), Some("end_turn"));
        assert!(m.done);
        assert_eq!(m.api_message_id.as_deref(), Some("msg_1"));

        // Another message's delta (its message_start missed): a fresh start.
        m.push(&se(
            json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "Next"}}),
            "msg_2",
        ));
        assert_eq!((m.text().as_str(), m.done), ("Next", false));
    }
}
