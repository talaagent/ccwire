//! [`UserInput`]: a user message written to the CLI's stdin.

use serde::{Serialize, Serializer};
use serde_json::Value;

use super::{ContentBlock, Extra, MessageContent};

/// A user message for `--input-format stream-json`:
/// `{"type": "user", "message": {"role": "user", "content": ...}, "uuid"?: ...}`.
///
/// Optional keys are left out when `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserInput {
    pub content: MessageContent,
    /// Names the message: the CLI echoes it (replays, `command_lifecycle`).
    pub uuid: Option<String>,
    /// Answers under a subagent's tool call.
    pub parent_tool_use_id: Option<String>,
    pub session_id: Option<String>,
    /// More top-level keys (`priority`, ...).
    pub extra: Extra,
}

impl UserInput {
    /// A text message.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: MessageContent::Text(text.into()),
            ..Self::default()
        }
    }

    /// A message of content blocks (text, images).
    pub fn blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            content: MessageContent::Blocks(blocks),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_uuid(mut self, uuid: impl Into<String>) -> Self {
        self.uuid = Some(uuid.into());
        self
    }

    #[must_use]
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    #[must_use]
    pub fn with_parent_tool_use_id(mut self, id: impl Into<String>) -> Self {
        self.parent_tool_use_id = Some(id.into());
        self
    }

    /// The JSON object.
    ///
    /// # Errors
    /// A value in it doesn't serialize (a hand-built `Other` block whose
    /// `raw` isn't an object, say).
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

impl From<&str> for UserInput {
    fn from(s: &str) -> Self {
        Self::text(s)
    }
}

impl From<String> for UserInput {
    fn from(s: String) -> Self {
        Self::text(s)
    }
}

impl From<Vec<ContentBlock>> for UserInput {
    fn from(b: Vec<ContentBlock>) -> Self {
        Self::blocks(b)
    }
}

#[derive(Serialize)]
struct Wire<'a> {
    #[serde(rename = "type")]
    type_: &'static str,
    message: WireMessage<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uuid: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_tool_use_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<&'a str>,
    #[serde(flatten)]
    extra: &'a Extra,
}

#[derive(Serialize)]
struct WireMessage<'a> {
    role: &'static str,
    content: &'a MessageContent,
}

impl Serialize for UserInput {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Wire {
            type_: "user",
            message: WireMessage {
                role: "user",
                content: &self.content,
            },
            uuid: self.uuid.as_deref(),
            parent_tool_use_id: self.parent_tool_use_id.as_deref(),
            session_id: self.session_id.as_deref(),
            extra: &self.extra,
        }
        .serialize(s)
    }
}
