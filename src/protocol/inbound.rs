//! [`Inbound`]: one frame the CLI writes, by `type`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use super::{
    AssistantMessage, ControlCancelRequest, ControlRequest, ControlResponse, Message,
    RateLimitEvent, ResultMessage, StreamEvent, System, UserMessage, from_fields, str_of, tagged,
};

/// One line of stream-json, by `type`.
///
/// Parsing never fails on a JSON object: an unknown `type` is
/// [`Inbound::Other`], a known one that doesn't parse is
/// [`Inbound::Malformed`]. Both keep the object as it came.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Inbound {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(System),
    Result(ResultMessage),
    StreamEvent(StreamEvent),
    RateLimitEvent(RateLimitEvent),
    ControlRequest(ControlRequest),
    ControlResponse(ControlResponse),
    ControlCancelRequest(ControlCancelRequest),
    /// A `type` this version doesn't model (or no string `type`).
    Other {
        /// Its `type` (empty if it had none).
        type_: String,
        /// The whole object.
        raw: Value,
    },
    /// A known `type` that didn't parse: the protocol changed.
    Malformed {
        type_: String,
        /// Why it didn't parse.
        error: String,
        /// The whole object.
        raw: Value,
    },
}

/// A line that isn't a JSON object.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not a JSON object")]
    NotAnObject,
}

/// A frame of the control channel.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[expect(
    clippy::large_enum_variant,
    reason = "one per line read, moved once: boxing would only add an allocation"
)]
pub enum Control {
    Request(ControlRequest),
    Response(ControlResponse),
    CancelRequest(ControlCancelRequest),
}

/// An [`Inbound`] split by who consumes it.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Message(Message),
    Control(Control),
}

impl Inbound {
    /// Parse one line (surrounding whitespace allowed).
    ///
    /// # Errors
    /// The line isn't JSON, or isn't an object.
    pub fn from_line(line: &str) -> Result<Self, ParseError> {
        Self::from_value(serde_json::from_str(line.trim())?)
    }

    /// Parse a JSON value.
    ///
    /// # Errors
    /// It isn't an object.
    pub fn from_value(v: Value) -> Result<Self, ParseError> {
        match v {
            Value::Object(obj) => Ok(Self::from_object(obj)),
            _ => Err(ParseError::NotAnObject),
        }
    }

    /// Parse a JSON object. Never fails.
    pub fn from_object(obj: Map<String, Value>) -> Self {
        let Some(type_) = str_of(&obj, "type") else {
            return Self::Other {
                type_: String::new(),
                raw: Value::Object(obj),
            };
        };
        let skip = &["type"];
        let parsed = match type_ {
            "user" => from_fields(&obj, skip).map(Self::User),
            "assistant" => from_fields(&obj, skip).map(Self::Assistant),
            "system" => System::from_object(&obj).map(Self::System),
            "result" => from_fields(&obj, skip).map(Self::Result),
            "stream_event" => from_fields(&obj, skip).map(Self::StreamEvent),
            "rate_limit_event" => from_fields(&obj, skip).map(Self::RateLimitEvent),
            "control_request" => from_fields(&obj, skip).map(Self::ControlRequest),
            "control_response" => from_fields(&obj, skip).map(Self::ControlResponse),
            "control_cancel_request" => from_fields(&obj, skip).map(Self::ControlCancelRequest),
            _ => {
                return Self::Other {
                    type_: type_.to_owned(),
                    raw: Value::Object(obj),
                };
            }
        };
        match parsed {
            Ok(v) => v,
            Err(e) => Self::Malformed {
                type_: type_.to_owned(),
                error: e.to_string(),
                raw: Value::Object(obj),
            },
        }
    }

    /// Its `type`.
    pub fn type_name(&self) -> &str {
        match self {
            Self::User(_) => "user",
            Self::Assistant(_) => "assistant",
            Self::System(_) => "system",
            Self::Result(_) => "result",
            Self::StreamEvent(_) => "stream_event",
            Self::RateLimitEvent(_) => "rate_limit_event",
            Self::ControlRequest(_) => "control_request",
            Self::ControlResponse(_) => "control_response",
            Self::ControlCancelRequest(_) => "control_cancel_request",
            Self::Other { type_, .. } | Self::Malformed { type_, .. } => type_,
        }
    }

    /// Whether this frame ends a turn: a `result`, parsed or
    /// [`Inbound::Malformed`] (a result whose shape drifted still ends the
    /// turn).
    pub fn ends_turn(&self) -> bool {
        match self {
            Self::Result(_) => true,
            Self::Malformed { type_, .. } => type_ == "result",
            _ => false,
        }
    }

    /// The JSON object.
    ///
    /// # Errors
    /// A hand-built `Other` or `Malformed` whose `raw` isn't an object, or
    /// a value serde can't serialize.
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        let t = self.type_name();
        match self {
            Self::User(m) => tagged("type", t, m),
            Self::Assistant(m) => tagged("type", t, m),
            Self::System(m) => tagged("type", t, m),
            Self::Result(m) => tagged("type", t, m),
            Self::StreamEvent(m) => tagged("type", t, m),
            Self::RateLimitEvent(m) => tagged("type", t, m),
            Self::ControlRequest(m) => tagged("type", t, m),
            Self::ControlResponse(m) => tagged("type", t, m),
            Self::ControlCancelRequest(m) => tagged("type", t, m),
            Self::Other { raw, .. } | Self::Malformed { raw, .. } => Ok(raw.clone()),
        }
    }

    /// Split into a [`Message`] or a [`Control`] frame. `Other` and
    /// `Malformed` (of any `type`) are messages.
    pub fn split(self) -> Frame {
        match self {
            Self::User(m) => Frame::Message(Message::User(m)),
            Self::Assistant(m) => Frame::Message(Message::Assistant(m)),
            Self::System(m) => Frame::Message(Message::System(m)),
            Self::Result(m) => Frame::Message(Message::Result(m)),
            Self::StreamEvent(m) => Frame::Message(Message::StreamEvent(m)),
            Self::RateLimitEvent(m) => Frame::Message(Message::RateLimitEvent(m)),
            Self::Other { type_, raw } => Frame::Message(Message::Other { type_, raw }),
            Self::Malformed { type_, error, raw } => {
                Frame::Message(Message::Malformed { type_, error, raw })
            }
            Self::ControlRequest(c) => Frame::Control(Control::Request(c)),
            Self::ControlResponse(c) => Frame::Control(Control::Response(c)),
            Self::ControlCancelRequest(c) => Frame::Control(Control::CancelRequest(c)),
        }
    }
}

impl TryFrom<Inbound> for Message {
    /// The control frame it was instead.
    type Error = Control;

    fn try_from(i: Inbound) -> Result<Self, Control> {
        match i.split() {
            Frame::Message(m) => Ok(m),
            Frame::Control(c) => Err(c),
        }
    }
}

impl From<Message> for Inbound {
    fn from(m: Message) -> Self {
        match m {
            Message::User(m) => Self::User(m),
            Message::Assistant(m) => Self::Assistant(m),
            Message::System(m) => Self::System(m),
            Message::Result(m) => Self::Result(m),
            Message::StreamEvent(m) => Self::StreamEvent(m),
            Message::RateLimitEvent(m) => Self::RateLimitEvent(m),
            Message::Other { type_, raw } => Self::Other { type_, raw },
            Message::Malformed { type_, error, raw } => Self::Malformed { type_, error, raw },
        }
    }
}

impl From<Control> for Inbound {
    fn from(c: Control) -> Self {
        match c {
            Control::Request(c) => Self::ControlRequest(c),
            Control::Response(c) => Self::ControlResponse(c),
            Control::CancelRequest(c) => Self::ControlCancelRequest(c),
        }
    }
}

impl From<Frame> for Inbound {
    fn from(f: Frame) -> Self {
        match f {
            Frame::Message(m) => m.into(),
            Frame::Control(c) => c.into(),
        }
    }
}

impl Serialize for Inbound {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.to_value()
            .map_err(serde::ser::Error::custom)?
            .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Inbound {
    /// Fails only on a value that isn't a JSON object.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let obj = super::object(d, "Inbound")?;
        Ok(Self::from_object(obj))
    }
}

impl Message {
    /// Whether this message ends a turn (see [`Inbound::ends_turn`]).
    pub fn ends_turn(&self) -> bool {
        match self {
            Self::Result(_) => true,
            Self::Malformed { type_, .. } => type_ == "result",
            _ => false,
        }
    }

    /// Its `type`.
    pub fn type_name(&self) -> &str {
        match self {
            Self::User(_) => "user",
            Self::Assistant(_) => "assistant",
            Self::System(_) => "system",
            Self::Result(_) => "result",
            Self::StreamEvent(_) => "stream_event",
            Self::RateLimitEvent(_) => "rate_limit_event",
            Self::Other { type_, .. } | Self::Malformed { type_, .. } => type_,
        }
    }
}
