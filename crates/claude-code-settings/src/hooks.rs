//! `hooks`: event → matcher groups → hooks.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Extra;
use crate::enums::{HookEvent, Shell};
use crate::walk::{Unknown, join, walk};

/// `hooks`: the matcher groups of each event, in the order they run.
pub type Hooks = BTreeMap<HookEvent, Vec<HookMatcher>>;

/// A matcher group: the hooks to run when `matcher` matches the event.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HookMatcher {
    /// The pattern to match the event's context (a tool name for tool
    /// events), case-sensitive; unset matches everything.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// The hooks to run.
    #[serde(default)]
    pub hooks: Vec<Hook>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

impl HookMatcher {
    /// A group that runs `hooks` on every occurrence of the event.
    pub fn all(hooks: impl IntoIterator<Item = Hook>) -> Self {
        Self {
            hooks: hooks.into_iter().collect(),
            ..Self::default()
        }
    }

    /// A group that runs `hooks` when `matcher` matches.
    pub fn matching(matcher: impl Into<String>, hooks: impl IntoIterator<Item = Hook>) -> Self {
        Self {
            matcher: Some(matcher.into()),
            ..Self::all(hooks)
        }
    }
}

walk!(HookMatcher { hooks: "hooks" });

/// A hook, by its `type`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Hook {
    /// `command`: run a shell command (or a program, with `args`).
    Command(CommandHook),
    /// `prompt`: ask a model about the event.
    Prompt(PromptHook),
    /// `agent`: have an agent verify something.
    Agent(AgentHook),
    /// `http`: `POST` the event to a URL.
    Http(HttpHook),
    /// `mcp_tool`: call a tool on a connected MCP server.
    McpTool(McpToolHook),
    /// A `type` this crate doesn't know (or none), the whole object as
    /// written.
    Other(Map<String, Value>),
}

impl Hook {
    /// A command hook running `command`.
    pub fn command(command: impl Into<String>) -> Self {
        Self::Command(CommandHook::new(command))
    }

    /// The hook's `type`.
    pub fn kind(&self) -> Option<&str> {
        match self {
            Self::Command(_) => Some("command"),
            Self::Prompt(_) => Some("prompt"),
            Self::Agent(_) => Some("agent"),
            Self::Http(_) => Some("http"),
            Self::McpTool(_) => Some("mcp_tool"),
            Self::Other(m) => m.get("type").and_then(Value::as_str),
        }
    }
}

impl Serialize for Hook {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let inner = match self {
            Self::Command(h) => serde_json::to_value(h),
            Self::Prompt(h) => serde_json::to_value(h),
            Self::Agent(h) => serde_json::to_value(h),
            Self::Http(h) => serde_json::to_value(h),
            Self::McpTool(h) => serde_json::to_value(h),
            Self::Other(m) => return m.serialize(s),
        };
        let Value::Object(mut obj) = inner.map_err(serde::ser::Error::custom)? else {
            return Err(serde::ser::Error::custom("a hook serializes as an object"));
        };
        if let Some(kind) = self.kind() {
            obj.insert("type".into(), kind.into());
        }
        obj.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Hook {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        fn parse<T: serde::de::DeserializeOwned, E: serde::de::Error>(
            mut obj: Map<String, Value>,
        ) -> Result<T, E> {
            obj.remove("type");
            serde_json::from_value(Value::Object(obj)).map_err(E::custom)
        }
        let obj = Map::<String, Value>::deserialize(d)?;
        Ok(match obj.get("type").and_then(Value::as_str) {
            Some("command") => Self::Command(parse(obj)?),
            Some("prompt") => Self::Prompt(parse(obj)?),
            Some("agent") => Self::Agent(parse(obj)?),
            Some("http") => Self::Http(parse(obj)?),
            Some("mcp_tool") => Self::McpTool(parse(obj)?),
            _ => Self::Other(obj),
        })
    }
}

impl Unknown for Hook {
    fn unknown(&self, path: &str, out: &mut Vec<String>) {
        match self {
            Self::Command(h) => h.unknown(path, out),
            Self::Prompt(h) => h.unknown(path, out),
            Self::Agent(h) => h.unknown(path, out),
            Self::Http(h) => h.unknown(path, out),
            Self::McpTool(h) => h.unknown(path, out),
            Self::Other(_) => out.push(join(path, "type")),
        }
    }
}

/// A `command` hook.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CommandHook {
    /// The shell command (or, with `args`, the program).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// The program's arguments: run without a shell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Timeout in seconds (Claude Code's default is 600, 30 on
    /// `UserPromptSubmit`, 10 on `MessageDisplay`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// Run in the background without blocking.
    #[serde(rename = "async", skip_serializing_if = "Option::is_none")]
    pub run_async: Option<bool>,
    /// Run in the background and wake the model when it exits with code 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub async_rewake: Option<bool>,
    /// The shell to run `command` in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<Shell>,
    /// A permission rule that filters tool events (`Bash(git *)`).
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// The spinner message while it runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

impl CommandHook {
    /// A command hook running `command` in a shell.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: Some(command.into()),
            ..Self::default()
        }
    }

    /// Sets `timeout`, in seconds.
    #[must_use]
    pub fn with_timeout(mut self, seconds: f64) -> Self {
        self.timeout = Some(seconds);
        self
    }
}

/// A `prompt` hook.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PromptHook {
    /// The prompt; `$ARGUMENTS` is the hook input JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The model (a fast one by default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Timeout in seconds (default 30).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// A permission rule that filters tool events.
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// The spinner message while it runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// On `ok: false`, feed the reason back and continue the turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_on_block: Option<bool>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An `agent` hook.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AgentHook {
    /// What to verify; `$ARGUMENTS` is the hook input JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The model (a fast one by default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Timeout in seconds (default 60).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// A permission rule that filters tool events.
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// The spinner message while it runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An `http` hook.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HttpHook {
    /// The URL to `POST` the hook input to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Request headers; values interpolate `$VAR` from `allowedEnvVars`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    /// The environment variables `headers` may interpolate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_env_vars: Option<Vec<String>>,
    /// Timeout in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// A permission rule that filters tool events.
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// The spinner message while it runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An `mcp_tool` hook.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct McpToolHook {
    /// The MCP server (already connected).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// The tool to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// The tool's arguments; strings substitute `${path}` from the hook
    /// input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Map<String, Value>>,
    /// Timeout in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// A permission rule that filters tool events.
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// The spinner message while it runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

walk!(CommandHook {});
walk!(PromptHook {});
walk!(AgentHook {});
walk!(HttpHook {});
walk!(McpToolHook {});
