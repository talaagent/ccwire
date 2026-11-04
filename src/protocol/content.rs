//! Content blocks of a message (the Messages API's, re-modelled).

use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;

use super::{Extra, Field, tagged_enum};

/// One content block, by `type`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ContentBlock {
    /// `text`.
    Text(TextBlock),
    /// `thinking`.
    Thinking(ThinkingBlock),
    /// `redacted_thinking`.
    RedactedThinking(RedactedThinkingBlock),
    /// `tool_use`: the model calls a tool the client runs.
    ToolUse(ToolUseBlock),
    /// `tool_result`: a tool's answer (in a user message).
    ToolResult(ToolResultBlock),
    /// `server_tool_use`: a tool the API runs (web search, ...).
    ServerToolUse(ServerToolUseBlock),
    /// `image`.
    Image(ImageBlock),
    /// Any other block type, as it came (`raw` has its `type`).
    Other {
        /// Its `type` (empty if it had none).
        type_: String,
        /// The block.
        raw: Value,
    },
}

tagged_enum!(ContentBlock, key = "type", skip = [], other = type_, {
    "text" => Text,
    "thinking" => Thinking,
    "redacted_thinking" => RedactedThinking,
    "tool_use" => ToolUse,
    "tool_result" => ToolResult,
    "server_tool_use" => ServerToolUse,
    "image" => Image,
});

impl ContentBlock {
    /// A text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(TextBlock {
            text: text.into(),
            extra: Extra::new(),
        })
    }

    /// Its text, if a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(t) => Some(&t.text),
            _ => None,
        }
    }
}

/// `{"type": "text", "text": ...}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
    /// `citations` and the rest.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"type": "thinking", "thinking": ..., "signature": ...}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ThinkingBlock {
    pub thinking: String,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub signature: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"type": "redacted_thinking", "data": ...}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RedactedThinkingBlock {
    pub data: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"type": "tool_use", "id": ..., "name": ..., "input": {...}}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"type": "server_tool_use", "id": ..., "name": ..., "input": {...}}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ServerToolUseBlock {
    pub id: String,
    pub name: String,
    pub input: Value,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"type": "tool_result", "tool_use_id": ..., "content": ..., "is_error": ...}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub content: Field<ToolResultContent>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub is_error: Field<bool>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A tool result's `content`: a string or blocks (text, image).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl<'de> Deserialize<'de> for ToolResultContent {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match d.deserialize_any(TextOrBlocks)? {
            Ok(s) => Self::Text(s),
            Err(b) => Self::Blocks(b),
        })
    }
}

/// `{"type": "image", "source": {...}}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImageBlock {
    /// `{type: base64, media_type, data}` or `{type: url, url}`.
    pub source: Value,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A message's `content`: a string or blocks.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl<'de> Deserialize<'de> for MessageContent {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match d.deserialize_any(TextOrBlocks)? {
            Ok(s) => Self::Text(s),
            Err(b) => Self::Blocks(b),
        })
    }
}

/// Reads a string (`Ok`) or an array of blocks (`Err`), keeping a bad
/// block's own error (an untagged enum would only say "did not match any
/// variant").
struct TextOrBlocks;

impl<'de> de::Visitor<'de> for TextOrBlocks {
    type Value = Result<String, Vec<ContentBlock>>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a string or an array of content blocks")
    }

    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        Ok(Ok(s.to_owned()))
    }

    fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
        Ok(Ok(s))
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut blocks = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        loop {
            let i = blocks.len();
            match seq.next_element::<ContentBlock>() {
                Ok(Some(b)) => blocks.push(b),
                Ok(None) => return Ok(Err(blocks)),
                Err(e) => {
                    return Err(de::Error::custom(format_args!("content block {i}: {e}")));
                }
            }
        }
    }
}

impl Default for MessageContent {
    fn default() -> Self {
        Self::Blocks(Vec::new())
    }
}

impl MessageContent {
    /// The blocks (none for a plain string).
    pub fn blocks(&self) -> &[ContentBlock] {
        match self {
            Self::Text(_) => &[],
            Self::Blocks(b) => b,
        }
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}

impl From<Vec<ContentBlock>> for MessageContent {
    fn from(b: Vec<ContentBlock>) -> Self {
        Self::Blocks(b)
    }
}
