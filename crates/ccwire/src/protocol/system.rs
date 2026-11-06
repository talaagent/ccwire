//! `type: system`, by `subtype`.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{Extra, Field, tagged_enum};

/// `type: system`, by `subtype`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum System {
    /// `init`: the session started (or resumed).
    Init(SystemInit),
    /// `compact_boundary`: the conversation was compacted.
    CompactBoundary(CompactBoundary),
    /// `task_started`: a background task (subagent, shell) started.
    TaskStarted(TaskStarted),
    /// `task_progress`.
    TaskProgress(TaskProgress),
    /// `task_updated`: a task's state changed (`patch`).
    TaskUpdated(TaskUpdated),
    /// `task_notification`: a task completed, failed or was stopped.
    TaskNotification(TaskNotification),
    /// `status`: what the CLI is doing (`requesting`, `compacting`, ...).
    Status(SystemStatus),
    /// `background_tasks_changed`: the full list of running background
    /// tasks.
    BackgroundTasksChanged(BackgroundTasksChanged),
    /// Any other subtype (`hook_response`, `thinking_tokens`, ...), as it
    /// came.
    Other {
        /// Its `subtype` (empty if it had none).
        subtype: String,
        /// The whole object.
        raw: Value,
    },
}

tagged_enum!(System, key = "subtype", skip = ["type"], other = subtype, {
    "init" => Init,
    "compact_boundary" => CompactBoundary,
    "task_started" => TaskStarted,
    "task_progress" => TaskProgress,
    "task_updated" => TaskUpdated,
    "task_notification" => TaskNotification,
    "status" => Status,
    "background_tasks_changed" => BackgroundTasksChanged,
});

/// `system/init`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemInit {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub model: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub cwd: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub tools: Field<Vec<String>>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub mcp_servers: Field<Vec<McpServerStatus>>,
    /// `default`, `acceptEdits`, `plan`, `bypassPermissions`, ...
    #[serde(
        default,
        rename = "permissionMode",
        skip_serializing_if = "Field::is_absent"
    )]
    pub permission_mode: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub claude_code_version: Field<String>,
    #[serde(
        default,
        rename = "apiKeySource",
        skip_serializing_if = "Field::is_absent"
    )]
    pub api_key_source: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub slash_commands: Field<Vec<String>>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub agents: Field<Vec<String>>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub skills: Field<Vec<String>>,
    /// Slash commands that only work in the interactive terminal UI.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub terminal_slash_commands: Field<Vec<String>>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub output_style: Field<String>,
    /// Protocol capabilities (`interrupt_receipt_v1`, ...): an open set.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub capabilities: Field<Vec<String>>,
    /// `plugins`, `betas`, `fast_mode_state` and the rest.
    #[serde(flatten)]
    pub extra: Extra,
}

impl SystemInit {
    /// The MCP server `name`'s status (`connected`, `pending`, `failed`,
    /// `needs-auth`, ...).
    pub fn mcp_status(&self, name: &str) -> Option<&str> {
        self.mcp_servers
            .get()?
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.status.as_str())
    }
}

/// One of `system/init`'s `mcp_servers`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpServerStatus {
    pub name: String,
    pub status: String,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `system/compact_boundary`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CompactBoundary {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub compact_metadata: Field<CompactMetadata>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `compact_boundary.compact_metadata`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CompactMetadata {
    /// `auto` or `manual`.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub trigger: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub pre_tokens: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub post_tokens: Field<u64>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// A task's `usage`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskUsage {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub total_tokens: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub tool_uses: Field<u64>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub duration_ms: Field<u64>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `system/task_started`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskStarted {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub task_id: Field<String>,
    /// The tool call that started it (`Task`, `Bash` in the background).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub tool_use_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub description: Field<String>,
    /// `local_agent`, `local_bash`, ...
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub task_type: Field<String>,
    /// Set on a task a subagent started (its background shell, ...).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub owned_by_subagent: Field<bool>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `system/task_progress`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskProgress {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub task_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub tool_use_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub description: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub last_tool_name: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub usage: Field<TaskUsage>,
    /// Set on a task a subagent started (its background shell, ...).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub owned_by_subagent: Field<bool>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `system/task_updated`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskUpdated {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub task_id: Field<String>,
    /// The changed fields (`status`, `end_time`, ...).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub patch: Field<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub status: Field<String>,
    /// Set on a task a subagent started (its background shell, ...).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub owned_by_subagent: Field<bool>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

impl TaskUpdated {
    /// `status`, else `patch.status`.
    pub fn any_status(&self) -> Option<&str> {
        self.status.as_deref().or_else(|| {
            self.patch
                .get()
                .and_then(|p| p.get("status"))
                .and_then(Value::as_str)
        })
    }
}

/// `system/task_notification`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskNotification {
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub task_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub tool_use_id: Field<String>,
    /// `completed`, `failed` or `stopped`.
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub status: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub output_file: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub summary: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub usage: Field<TaskUsage>,
    /// Set on a task a subagent started (its background shell, ...).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub owned_by_subagent: Field<bool>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `owned_by_subagent` of the task frames.
macro_rules! subagent_owned {
    ($($t:ident),*) => {$(
        impl $t {
            /// Whether a subagent started the task (`owned_by_subagent:
            /// true`), not the main thread.
            pub fn subagent_owned(&self) -> bool {
                self.owned_by_subagent.get() == Some(&true)
            }
        }
    )*};
}
subagent_owned!(TaskStarted, TaskProgress, TaskUpdated, TaskNotification);

/// `system/status`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemStatus {
    /// `requesting`, `compacting`, ...: an open set (`null` when it is
    /// back to nothing in particular).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub status: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

/// `system/background_tasks_changed`: every background task running now
/// (the list replaces the last one).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BackgroundTasksChanged {
    /// The tasks as they came: objects with `task_id`, `task_type`,
    /// `description`, `subagent_type`, `parent_task_id`, ...
    /// ([`Self::task_objects`] skips anything else in the list).
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub tasks: Field<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub session_id: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_absent")]
    pub uuid: Field<String>,
    #[serde(flatten)]
    pub extra: Extra,
}

impl BackgroundTasksChanged {
    /// The tasks that are JSON objects.
    pub fn task_objects(&self) -> impl Iterator<Item = &Map<String, Value>> {
        self.tasks
            .get()
            .into_iter()
            .flatten()
            .filter_map(Value::as_object)
    }
}
