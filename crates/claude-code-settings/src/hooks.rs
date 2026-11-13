//! `hooks`: event → matcher groups → hooks.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Extra;
use crate::enums::{HookEvent, HookOnFailure, Shell};
use crate::walk::{Unknown, join, walk};

/// `hooks`: for each [`HookEvent`], its matcher groups.
///
/// Claude Code runs every group whose `matcher` matches the event, and the
/// hooks of all matching groups in parallel. Across settings layers the
/// groups of an event add up (see [`merge_values`](crate::merge_values));
/// an identical handler defined in two files runs once. An event name
/// Claude Code doesn't know is skipped with a warning; this crate keeps it as
/// [`HookEvent::Other`].
///
/// - **Default**: unset, so no hooks run.
/// - **Scope**: any settings file. With `allowManagedHooksOnly` set in managed
///   settings, only managed hooks run; `disableAllHooks` turns all of them off.
///
/// Docs: [`hooks`](https://code.claude.com/docs/en/settings-reference#hooks),
/// [hooks reference](https://code.claude.com/docs/en/hooks).
pub type Hooks = BTreeMap<HookEvent, Vec<HookMatcher>>;

/// A matcher group: the hooks to run when `matcher` matches the event.
///
/// Docs: [matcher patterns](https://code.claude.com/docs/en/hooks#matcher-patterns).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HookMatcher {
    /// What the group matches, case-sensitive. Each event matches on its own
    /// field: the tool name for tool events, the agent type for subagent
    /// events, the source for `SessionStart`, and so on.
    ///
    /// - **Values**: `"*"`, `""` or unset matches everything; a string of only
    ///   letters, digits, `_`, `-`, spaces, `,` and `|` is an exact name or a
    ///   list of names (`Edit|Write`); anything else is an unanchored
    ///   JavaScript regular expression (`mcp__memory__.*`).
    /// - **Default**: unset: every occurrence of the event.
    ///
    /// Docs: [`matcher`](https://code.claude.com/docs/en/hooks#matcher-patterns).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// The handlers to run, in parallel, when the group matches.
    ///
    /// - **Values**: [`Hook`]s: `command`, `http`, `mcp_tool`, `prompt` or
    ///   `agent`.
    /// - **Default**: none: Claude Code requires the key, so this crate always
    ///   writes it (an empty list when unset).
    ///
    /// Docs: [hook handler fields](https://code.claude.com/docs/en/hooks#hook-handler-fields).
    #[serde(default)]
    pub hooks: Vec<Hook>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
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
///
/// Docs: [settings reference](https://code.claude.com/docs/en/hooks#command-hook-fields).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CommandHook {
    /// Shell command to execute. With `args`, the executable to spawn directly. See [Exec form
    /// and shell form](https://code.claude.com/docs/en/hooks#exec-form-and-shell-form).
    ///
    /// - **Values**: string
    ///
    /// Docs: [`command`](https://code.claude.com/docs/en/hooks#command-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Argument list. When present, `command` is resolved as an executable and spawned directly
    /// with `args` as the argument vector, with no shell involved. See [Exec form and shell
    /// form](https://code.claude.com/docs/en/hooks#exec-form-and-shell-form).
    ///
    /// - **Values**: array of strings
    ///
    /// Docs: [`args`](https://code.claude.com/docs/en/hooks#command-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Seconds before canceling. Claude Code doesn't enforce it on a command hook you run with
    /// [`async: true`](https://code.claude.com/docs/en/hooks#run-hooks-in-the-background).
    /// Defaults: 600 for `command`, `http`, and `mcp_tool`; 30 for `prompt`; 60 for `agent`.
    ///
    /// - **Values**: a number
    ///
    /// Docs: [`timeout`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// If `true`, runs in the background without blocking. See [Run hooks in the
    /// background](https://code.claude.com/docs/en/hooks#run-hooks-in-the-background).
    ///
    /// - **Values**: Boolean
    ///
    /// Docs: [`async`](https://code.claude.com/docs/en/hooks#command-hook-fields).
    #[serde(rename = "async", skip_serializing_if = "Option::is_none")]
    pub run_async: Option<bool>,
    /// If `true`, runs in the background and wakes Claude on exit code 2. The hook's stderr, or
    /// stdout if stderr is empty, is shown to Claude as a [system
    /// reminder](https://code.claude.com/docs/en/glossary#system-reminder) so it can react to a
    /// long-running background failure.
    ///
    /// - **Values**: Boolean
    ///
    /// Docs: [`asyncRewake`](https://code.claude.com/docs/en/hooks#command-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub async_rewake: Option<bool>,
    /// Shell to use for this hook. Accepts `"bash"` or `"powershell"`. Defaults to `"bash"`, or
    /// to `"powershell"` on Windows when Git Bash isn't installed.
    ///
    /// - **Values**: `"bash"`, `"powershell"`
    ///
    /// Docs: [`shell`](https://code.claude.com/docs/en/hooks#command-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<Shell>,
    /// [Permission rule
    /// syntax](https://code.claude.com/docs/en/permissions#permission-rule-syntax) to filter
    /// when this hook runs, such as `"Bash(git *)"` or `"Edit(*.ts)"`. The hook command only
    /// runs if the tool call matches the pattern. See the [Bash matching
    /// table](https://code.claude.com/docs/en/hooks#bash-if-matching) for how Bash patterns
    /// evaluate against subcommands, `$()`, and backticks.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`if`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Custom spinner message displayed while the hook runs.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`statusMessage`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Whether the action that triggered the hook goes ahead when the hook fails: `"continue"`
    /// or `"block"`. See [Block the action when a hook
    /// fails](https://code.claude.com/docs/en/hooks#block-the-action-when-a-hook-fails) for
    /// `"block"` and for leaving the field unset. Requires Claude Code v2.1.295 or later.
    ///
    /// - **Values**: `"continue"` or `"block"`. Requires Claude Code v2.1.295 or later
    /// - **Default**: unset: the action goes ahead on most events
    ///
    /// Docs: [`onFailure`](https://code.claude.com/docs/en/hooks#command-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<HookOnFailure>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
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
///
/// Docs: [settings reference](https://code.claude.com/docs/en/hooks#prompt-and-agent-hook-fields).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PromptHook {
    /// Prompt text to send to the model. Use `$ARGUMENTS` as a placeholder for the hook input
    /// JSON. Escape with a backslash to include literal text: `\$1.00` renders as `$1.00`.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`prompt`](https://code.claude.com/docs/en/hooks#prompt-and-agent-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Model to use for evaluation. Defaults to the model Claude Code uses for [background
    /// functionality](https://code.claude.com/docs/en/model-config#environment-variables).
    ///
    /// - **Values**: string
    ///
    /// Docs: [`model`](https://code.claude.com/docs/en/hooks#prompt-and-agent-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Seconds before canceling. Claude Code doesn't enforce it on a command hook you run with
    /// [`async: true`](https://code.claude.com/docs/en/hooks#run-hooks-in-the-background).
    /// Defaults: 600 for `command`, `http`, and `mcp_tool`; 30 for `prompt`; 60 for `agent`.
    ///
    /// - **Values**: a number
    ///
    /// Docs: [`timeout`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// [Permission rule
    /// syntax](https://code.claude.com/docs/en/permissions#permission-rule-syntax) to filter
    /// when this hook runs, such as `"Bash(git *)"` or `"Edit(*.ts)"`. The hook command only
    /// runs if the tool call matches the pattern. See the [Bash matching
    /// table](https://code.claude.com/docs/en/hooks#bash-if-matching) for how Bash patterns
    /// evaluate against subcommands, `$()`, and backticks.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`if`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Custom spinner message displayed while the hook runs.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`statusMessage`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// When the prompt returns ok: false, feed the reason back to Claude and continue the turn
    /// instead of stopping. Implemented as continue: true on the resulting decision: "block".
    ///
    /// - **Values**: Boolean
    /// - **Default**: `false`
    ///
    /// Docs: not in the settings reference; listed in the
    /// [JSON schema](https://json.schemastore.org/claude-code-settings.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continue_on_block: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An `agent` hook.
///
/// Docs: [settings reference](https://code.claude.com/docs/en/hooks#prompt-and-agent-hook-fields).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AgentHook {
    /// Prompt text to send to the model. Use `$ARGUMENTS` as a placeholder for the hook input
    /// JSON. Escape with a backslash to include literal text: `\$1.00` renders as `$1.00`.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`prompt`](https://code.claude.com/docs/en/hooks#prompt-and-agent-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Model to use for evaluation. Defaults to the model Claude Code uses for [background
    /// functionality](https://code.claude.com/docs/en/model-config#environment-variables).
    ///
    /// - **Values**: string
    ///
    /// Docs: [`model`](https://code.claude.com/docs/en/hooks#prompt-and-agent-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Seconds before canceling. Claude Code doesn't enforce it on a command hook you run with
    /// [`async: true`](https://code.claude.com/docs/en/hooks#run-hooks-in-the-background).
    /// Defaults: 600 for `command`, `http`, and `mcp_tool`; 30 for `prompt`; 60 for `agent`.
    ///
    /// - **Values**: a number
    ///
    /// Docs: [`timeout`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// [Permission rule
    /// syntax](https://code.claude.com/docs/en/permissions#permission-rule-syntax) to filter
    /// when this hook runs, such as `"Bash(git *)"` or `"Edit(*.ts)"`. The hook command only
    /// runs if the tool call matches the pattern. See the [Bash matching
    /// table](https://code.claude.com/docs/en/hooks#bash-if-matching) for how Bash patterns
    /// evaluate against subcommands, `$()`, and backticks.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`if`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Custom spinner message displayed while the hook runs.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`statusMessage`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An `http` hook.
///
/// Docs: [settings reference](https://code.claude.com/docs/en/hooks#http-hook-fields).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HttpHook {
    /// URL to send the POST request to.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`url`](https://code.claude.com/docs/en/hooks#http-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Additional HTTP headers as key-value pairs. Values support environment variable
    /// interpolation using `$VAR_NAME` or `${VAR_NAME}` syntax. Only variables listed in
    /// `allowedEnvVars` are resolved.
    ///
    /// - **Values**: object
    ///
    /// Docs: [`headers`](https://code.claude.com/docs/en/hooks#http-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    /// List of environment variable names that may be interpolated into header values.
    /// References to unlisted variables are replaced with empty strings. Required for any env
    /// var interpolation to work.
    ///
    /// - **Values**: array of strings
    ///
    /// Docs: [`allowedEnvVars`](https://code.claude.com/docs/en/hooks#http-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_env_vars: Option<Vec<String>>,
    /// Seconds before canceling. Claude Code doesn't enforce it on a command hook you run with
    /// [`async: true`](https://code.claude.com/docs/en/hooks#run-hooks-in-the-background).
    /// Defaults: 600 for `command`, `http`, and `mcp_tool`; 30 for `prompt`; 60 for `agent`.
    ///
    /// - **Values**: a number
    ///
    /// Docs: [`timeout`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// [Permission rule
    /// syntax](https://code.claude.com/docs/en/permissions#permission-rule-syntax) to filter
    /// when this hook runs, such as `"Bash(git *)"` or `"Edit(*.ts)"`. The hook command only
    /// runs if the tool call matches the pattern. See the [Bash matching
    /// table](https://code.claude.com/docs/en/hooks#bash-if-matching) for how Bash patterns
    /// evaluate against subcommands, `$()`, and backticks.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`if`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Custom spinner message displayed while the hook runs.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`statusMessage`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// What happens to the action when the hook fails: `"continue"`, the default, or `"block"`.
    /// See [Block the action when a hook
    /// fails](https://code.claude.com/docs/en/hooks#block-the-action-when-a-hook-fails).
    /// Requires Claude Code v2.1.295 or later.
    ///
    /// - **Values**: `"continue"` or `"block"`. Requires Claude Code v2.1.295 or later
    /// - **Default**: `"continue"`
    ///
    /// Docs: [`onFailure`](https://code.claude.com/docs/en/hooks#http-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_failure: Option<HookOnFailure>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An `mcp_tool` hook.
///
/// Docs: [settings reference](https://code.claude.com/docs/en/hooks#mcp-tool-hook-fields).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct McpToolHook {
    /// Name of a configured MCP server. For a [plugin-bundled
    /// server](https://code.claude.com/docs/en/mcp#plugin-provided-mcp-servers), this is the
    /// scoped name `plugin:<plugin-name>:<server-name>`, such as `plugin:my-plugin:db`, not the
    /// bare server key.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`server`](https://code.claude.com/docs/en/hooks#mcp-tool-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// Name of the tool to call on that server.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`tool`](https://code.claude.com/docs/en/hooks#mcp-tool-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// Arguments passed to the tool. String values support `${path}` substitution from the
    /// hook's [JSON input](https://code.claude.com/docs/en/hooks#hook-input-and-output), such
    /// as `"${tool_input.file_path}"`.
    ///
    /// - **Values**: object
    ///
    /// Docs: [`input`](https://code.claude.com/docs/en/hooks#mcp-tool-hook-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Map<String, Value>>,
    /// Seconds before canceling. Claude Code doesn't enforce it on a command hook you run with
    /// [`async: true`](https://code.claude.com/docs/en/hooks#run-hooks-in-the-background).
    /// Defaults: 600 for `command`, `http`, and `mcp_tool`; 30 for `prompt`; 60 for `agent`.
    ///
    /// - **Values**: a number
    ///
    /// Docs: [`timeout`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    /// [Permission rule
    /// syntax](https://code.claude.com/docs/en/permissions#permission-rule-syntax) to filter
    /// when this hook runs, such as `"Bash(git *)"` or `"Edit(*.ts)"`. The hook command only
    /// runs if the tool call matches the pattern. See the [Bash matching
    /// table](https://code.claude.com/docs/en/hooks#bash-if-matching) for how Bash patterns
    /// evaluate against subcommands, `$()`, and backticks.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`if`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Custom spinner message displayed while the hook runs.
    ///
    /// - **Values**: string
    ///
    /// Docs: [`statusMessage`](https://code.claude.com/docs/en/hooks#common-fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

walk!(CommandHook {});
walk!(PromptHook {});
walk!(AgentHook {});
walk!(HttpHook {});
walk!(McpToolHook {});
