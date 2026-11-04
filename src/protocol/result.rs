//! `type: result`: the end of a turn (one per user message in streaming
//! input). It isn't the end of the process: with callbacks registered
//! (hooks, `can_use_tool`, SDK MCP servers) the CLI keeps running, and the
//! run is over only once it is idle.
//!
//! The result frame is how a caller knows a turn ended, so it parses
//! leniently: `subtype`, `result` and `errors` take any JSON, and
//! [`super::Inbound::ends_turn`] also matches a result that didn't parse.

use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Number, Value};

use super::{Extra, Field};

/// `type: result`.
///
/// No `Default`: a made-up result must not read as a success.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultMessage {
    /// `success` or an `error_*` (absent or `null` on a drifted frame).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub subtype: Field<ResultSubtype>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub is_error: Field<bool>,
    /// The final text (`success`), as it came: [`Self::result_text`] reads
    /// it as a string.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub result: Field<Value>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub duration_ms: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub duration_api_ms: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub num_turns: Field<u64>,
    /// Kept as the CLI wrote it; [`Self::cost_usd`] reads it as `f64`.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub total_cost_usd: Field<Number>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub usage: Field<Usage>,
    /// Per model: `{inputTokens, outputTokens, costUSD, ...}`.
    #[serde(
        default,
        rename = "modelUsage",
        skip_serializing_if = "Field::is_absent"
    )]
    pub model_usage: Field<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub stop_reason: Field<String>,
    /// The `error_*` subtypes' messages, as they came:
    /// [`Self::error_messages`] reads the strings.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub errors: Field<Value>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    /// `permission_denials`, `structured_output` and the rest.
    #[serde(flatten)]
    pub extra: Extra,
}

impl ResultMessage {
    /// `total_cost_usd` as a float.
    pub fn cost_usd(&self) -> Option<f64> {
        self.total_cost_usd.get().and_then(Number::as_f64)
    }

    /// Whether the turn succeeded: subtype `success` and not `is_error`.
    pub fn is_success(&self) -> bool {
        self.subtype.get().is_some_and(ResultSubtype::is_success) && !self.failed()
    }

    /// Whether the turn failed: `is_error`, or an `error_*` subtype.
    pub fn failed(&self) -> bool {
        self.is_error.get().copied().unwrap_or(false)
            || self.subtype.get().is_some_and(ResultSubtype::is_error)
    }

    /// `result` if it is a string.
    pub fn result_text(&self) -> Option<&str> {
        self.result.get().and_then(Value::as_str)
    }

    /// The strings of `errors` (a list of strings, or one string).
    pub fn error_messages(&self) -> Vec<&str> {
        match self.errors.get() {
            Some(Value::String(s)) => vec![s.as_str()],
            Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        }
    }
}

/// A result's `subtype`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResultSubtype {
    Success,
    ErrorMaxTurns,
    ErrorDuringExecution,
    ErrorMaxBudgetUsd,
    ErrorMaxStructuredOutputRetries,
    /// Another subtype, as it came.
    Other(String),
    /// A `subtype` that isn't a string, as it came.
    NotAString(Value),
}

impl ResultSubtype {
    /// The wire name.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Success => "success",
            Self::ErrorMaxTurns => "error_max_turns",
            Self::ErrorDuringExecution => "error_during_execution",
            Self::ErrorMaxBudgetUsd => "error_max_budget_usd",
            Self::ErrorMaxStructuredOutputRetries => "error_max_structured_output_retries",
            Self::Other(s) => s,
            Self::NotAString(_) => "",
        }
    }

    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Any `error_*` subtype, known or not.
    pub fn is_error(&self) -> bool {
        self.as_str().starts_with("error")
    }
}

impl From<&str> for ResultSubtype {
    fn from(s: &str) -> Self {
        match s {
            "success" => Self::Success,
            "error_max_turns" => Self::ErrorMaxTurns,
            "error_during_execution" => Self::ErrorDuringExecution,
            "error_max_budget_usd" => Self::ErrorMaxBudgetUsd,
            "error_max_structured_output_retries" => Self::ErrorMaxStructuredOutputRetries,
            s => Self::Other(s.to_owned()),
        }
    }
}

impl fmt::Display for ResultSubtype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ResultSubtype {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::NotAString(v) => v.serialize(s),
            _ => s.serialize_str(self.as_str()),
        }
    }
}

impl<'de> Deserialize<'de> for ResultSubtype {
    /// Never fails: a non-string is [`ResultSubtype::NotAString`].
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match Value::deserialize(d)? {
            Value::String(s) => Self::from(s.as_str()),
            v => Self::NotAString(v),
        })
    }
}

/// Token usage (an assistant message's `usage`, a result's).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub input_tokens: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub output_tokens: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub cache_creation_input_tokens: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub cache_read_input_tokens: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub service_tier: Field<String>,
    /// `cache_creation`, `server_tool_use`, `iterations` and the rest.
    #[serde(flatten)]
    pub extra: Extra,
}

impl Usage {
    /// The context the request used: input plus cache reads and writes.
    pub fn context_tokens(&self) -> u64 {
        [
            &self.input_tokens,
            &self.cache_read_input_tokens,
            &self.cache_creation_input_tokens,
        ]
        .into_iter()
        .filter_map(Field::get)
        .sum()
    }
}
