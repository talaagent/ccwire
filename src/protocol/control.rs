//! The control channel: `control_request`, `control_response` and
//! `control_cancel_request`, in both directions.
//!
//! The caller sends `interrupt`, `initialize`, `set_permission_mode`,
//! `set_model` and `stop_task`; the CLI sends `can_use_tool`,
//! `hook_callback` and `mcp_message`. Each is answered with a
//! `control_response` carrying the same `request_id`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{Extra, Field, tagged_enum};

/// `{"type": "control_request", "request_id": ..., "request": {"subtype": ...}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlRequest {
    pub request_id: String,
    pub request: ControlRequestBody,
    #[serde(flatten)]
    pub extra: Extra,
}

impl ControlRequest {
    pub fn new(request_id: impl Into<String>, request: impl Into<ControlRequestBody>) -> Self {
        Self {
            request_id: request_id.into(),
            request: request.into(),
            extra: Extra::new(),
        }
    }
}

/// A control request's `request`, by `subtype`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ControlRequestBody {
    /// Stop the current turn.
    Interrupt(InterruptRequest),
    /// Register hooks, agents and the like (sent first, when there are any).
    Initialize(InitializeRequest),
    SetPermissionMode(SetPermissionModeRequest),
    SetModel(SetModelRequest),
    /// Stop a background task.
    StopTask(StopTaskRequest),
    /// From the CLI: may this tool call run? Answered with a
    /// [`PermissionResult`].
    CanUseTool(CanUseToolRequest),
    /// From the CLI: run a hook registered by `initialize`.
    HookCallback(HookCallbackRequest),
    /// From the CLI: a JSON-RPC message for an in-process MCP server.
    McpMessage(McpMessageRequest),
    /// Any other subtype (`rewind_files`, `mcp_reconnect`, ...), as it came.
    Other {
        /// Its `subtype` (empty if it had none).
        subtype: String,
        /// The whole `request` object.
        raw: Value,
    },
}

tagged_enum!(ControlRequestBody, key = "subtype", skip = [], other = subtype, {
    "interrupt" => Interrupt,
    "initialize" => Initialize,
    "set_permission_mode" => SetPermissionMode,
    "set_model" => SetModel,
    "stop_task" => StopTask,
    "can_use_tool" => CanUseTool,
    "hook_callback" => HookCallback,
    "mcp_message" => McpMessage,
});

macro_rules! into_body {
    ($($t:ident => $v:ident),* $(,)?) => {$(
        impl From<$t> for ControlRequestBody {
            fn from(r: $t) -> Self {
                Self::$v(r)
            }
        }
    )*};
}

into_body!(
    InterruptRequest => Interrupt,
    InitializeRequest => Initialize,
    SetPermissionModeRequest => SetPermissionMode,
    SetModelRequest => SetModel,
    StopTaskRequest => StopTask,
    CanUseToolRequest => CanUseTool,
    HookCallbackRequest => HookCallback,
    McpMessageRequest => McpMessage,
);

/// `interrupt`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InterruptRequest {
    #[serde(flatten)]
    pub extra: Extra,
}

/// `initialize`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InitializeRequest {
    /// `{HookEvent: [{matcher, hookCallbackIds, timeout?}]}`.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub hooks: Field<Map<String, Value>>,
    /// Agent definitions by name.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub agents: Field<Map<String, Value>>,
    /// `skills`, `excludeDynamicSections` and the rest.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `set_permission_mode`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SetPermissionModeRequest {
    /// `default`, `acceptEdits`, `plan`, `bypassPermissions`, ...
    pub mode: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `set_model`: `null` goes back to the default model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SetModelRequest {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub model: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `stop_task`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StopTaskRequest {
    pub task_id: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `can_use_tool`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CanUseToolRequest {
    pub tool_name: String,
    pub input: Value,
    /// Permission updates the CLI suggests (`addRules`, `setMode`, ...).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub permission_suggestions: Field<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub blocked_path: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub decision_reason: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub title: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub display_name: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub description: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub tool_use_id: Field<String>,
    /// Set when a subagent asks.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub agent_id: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `hook_callback`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HookCallbackRequest {
    pub callback_id: String,
    /// The hook's input (`hook_event_name`, `tool_name`, ...).
    pub input: Value,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub tool_use_id: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `mcp_message`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpMessageRequest {
    pub server_name: String,
    /// A JSON-RPC message.
    pub message: Value,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"type": "control_response", "response": {"subtype": "success" | "error", ...}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlResponse {
    pub response: ControlResponseBody,
    #[serde(flatten)]
    pub extra: Extra,
}

impl ControlResponse {
    /// A success answer to `request_id`.
    pub fn success(request_id: impl Into<String>, response: Option<Value>) -> Self {
        Self::of(ControlResponseBody::Success(ControlSuccess {
            request_id: request_id.into(),
            response: response.into(),
            extra: Extra::new(),
        }))
    }

    /// An error answer to `request_id`.
    pub fn error(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::of(ControlResponseBody::Error(ControlError {
            request_id: request_id.into(),
            error: error.into(),
            extra: Extra::new(),
        }))
    }

    /// The answer to a `can_use_tool` request.
    ///
    /// # Errors
    /// `result` doesn't serialize.
    pub fn permission(
        request_id: impl Into<String>,
        result: &PermissionResult,
    ) -> Result<Self, serde_json::Error> {
        let v = serde_json::to_value(result)?;
        Ok(Self::success(request_id, Some(v)))
    }

    fn of(response: ControlResponseBody) -> Self {
        Self {
            response,
            extra: Map::new(),
        }
    }

    /// The `request_id` it answers.
    pub fn request_id(&self) -> Option<&str> {
        match &self.response {
            ControlResponseBody::Success(s) => Some(&s.request_id),
            ControlResponseBody::Error(e) => Some(&e.request_id),
            ControlResponseBody::Other { raw, .. } => raw.get("request_id")?.as_str(),
        }
    }
}

/// A control response's `response`, by `subtype`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ControlResponseBody {
    Success(ControlSuccess),
    Error(ControlError),
    /// Any other subtype, as it came.
    Other {
        subtype: String,
        /// The whole `response` object.
        raw: Value,
    },
}

tagged_enum!(ControlResponseBody, key = "subtype", skip = [], other = subtype, {
    "success" => Success,
    "error" => Error,
});

/// `{"subtype": "success", "request_id": ..., "response": {...}}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlSuccess {
    pub request_id: String,
    /// The answer's payload (a [`PermissionResult`], a hook's output, ...).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub response: Field<Value>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"subtype": "error", "request_id": ..., "error": ...}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlError {
    pub request_id: String,
    pub error: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"type": "control_cancel_request", "request_id": ...}`: the CLI no
/// longer wants the answer to a request it sent.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlCancelRequest {
    pub request_id: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// The answer to `can_use_tool`, by `behavior`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "behavior", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PermissionResult {
    Allow(PermissionAllow),
    Deny(PermissionDeny),
}

impl PermissionResult {
    /// Allow the call with `input` (the request's, or a changed one).
    pub fn allow(input: Value) -> Self {
        Self::Allow(PermissionAllow {
            updated_input: input,
            updated_permissions: None,
            extra: Extra::new(),
        })
    }

    /// Deny the call; the model sees `message`.
    pub fn deny(message: impl Into<String>) -> Self {
        Self::Deny(PermissionDeny {
            message: message.into(),
            interrupt: None,
            extra: Extra::new(),
        })
    }
}

/// `{"behavior": "allow", "updatedInput": {...}, "updatedPermissions"?: [...]}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PermissionAllow {
    /// The input to run the tool with (the CLI requires it: send the
    /// request's `input` when unchanged).
    #[serde(rename = "updatedInput")]
    pub updated_input: Value,
    /// Permission updates to apply (`addRules`, `setMode`, ...).
    #[serde(
        default,
        rename = "updatedPermissions",
        skip_serializing_if = "Option::is_none"
    )]
    pub updated_permissions: Option<Vec<Value>>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `{"behavior": "deny", "message": ..., "interrupt"?: true}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PermissionDeny {
    pub message: String,
    /// Also stop the turn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interrupt: Option<bool>,
    #[serde(flatten)]
    pub extra: Extra,
}
