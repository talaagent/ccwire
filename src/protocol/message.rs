//! The conversation: [`Message`], and the `user`, `assistant`,
//! `stream_event` and `rate_limit_event` frames.

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use super::{ContentBlock, Extra, Field, MessageContent, ResultMessage, System, Usage};

/// What a caller consumes: every frame but the control channel's.
///
/// Built from an [`Inbound`](super::Inbound) with `TryFrom` or
/// [`Inbound::split`](super::Inbound::split).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(System),
    Result(ResultMessage),
    StreamEvent(StreamEvent),
    RateLimitEvent(RateLimitEvent),
    /// A `type` this version doesn't model, as it came.
    Other {
        /// Its `type` (empty if it had none).
        type_: String,
        raw: Value,
    },
    /// A known `type` that didn't parse (the protocol changed), as it came.
    Malformed {
        type_: String,
        /// Why it didn't parse.
        error: String,
        raw: Value,
    },
}

/// `type: assistant`: one API message (or a part of one: the CLI writes a
/// message once per content block).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub message: ApiMessage,
    /// Set on a subagent's messages: the `Task` tool call it runs under.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub parent_tool_use_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    /// `authentication_failed`, `rate_limit`, ... on a synthetic error reply.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub error: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

impl AssistantMessage {
    pub fn content(&self) -> &[ContentBlock] {
        &self.message.content
    }

    /// Whether a subagent's.
    pub const fn is_subagent(&self) -> bool {
        matches!(self.parent_tool_use_id, Field::Value(_))
    }
}

/// An assistant API message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ApiMessage {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub model: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub role: Field<String>,
    pub content: Vec<ContentBlock>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub stop_reason: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub stop_sequence: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub usage: Field<Usage>,
    /// `type: message`, `container`, `context_management` and the rest.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `type: user`: tool results, a subagent's prompt, a replayed input.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UserMessage {
    pub message: ApiUserMessage,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub parent_tool_use_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    /// The uuid the input was sent with (or the CLI's).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    /// `tool_use_result`, `isReplay`, `isSynthetic` and the rest.
    #[serde(flatten)]
    pub extra: Extra,
}

impl UserMessage {
    pub fn content(&self) -> &MessageContent {
        &self.message.content
    }
}

/// A user API message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ApiUserMessage {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub role: Field<String>,
    pub content: MessageContent,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `type: stream_event` (`--include-partial-messages`): one event of the
/// API's message stream, untyped.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamEvent {
    /// `{type: message_start | content_block_delta | ..., ...}`.
    pub event: Value,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub parent_tool_use_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

impl StreamEvent {
    /// The event's `type`.
    pub fn event_type(&self) -> Option<&str> {
        self.event.get("type").and_then(Value::as_str)
    }
}

/// `type: rate_limit_event`: the rate limit status changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimitEvent {
    pub rate_limit_info: RateLimitInfo,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A rate limit event's `rate_limit_info`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// `allowed`, `allowed_warning` or `rejected`.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub status: Field<String>,
    /// Unix seconds.
    #[serde(default, rename = "resetsAt", skip_serializing_if = "Field::is_absent")]
    pub resets_at: Field<u64>,
    /// `five_hour`, `seven_day`, ...
    #[serde(
        default,
        rename = "rateLimitType",
        skip_serializing_if = "Field::is_absent"
    )]
    pub rate_limit_type: Field<String>,
    /// The fraction used (0 to 1), as the CLI wrote it.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub utilization: Field<Number>,
    /// `overageStatus`, `unifiedWindows` and the rest.
    #[serde(flatten)]
    pub extra: Extra,
}
