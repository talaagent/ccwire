//! [`Options`]: the `claude` command line of a session, built and checked
//! without running anything.
//!
//! ```
//! use ccwire::options::{InputFormat, Options, PermissionMode};
//!
//! let argv = Options::new()
//!     .input_format(InputFormat::StreamJson)
//!     .model("sonnet")
//!     .permission_mode(PermissionMode::DontAsk)
//!     .allowed_tools(["Read", "Grep"])
//!     .resume("0b5f0d6e-5d3a-4c55-9a3f-1f2e3d4c5b6a")
//!     .to_argv();
//! assert_eq!(argv[..3], ["-p", "--input-format", "stream-json"]);
//! assert!(argv.contains(&"--resume=0b5f0d6e-5d3a-4c55-9a3f-1f2e3d4c5b6a".into()));
//! ```
//!
//! Each argument is one argv element, never shell-quoted: spaces and quotes
//! go through as they are. A value that starts with `-` is passed as
//! `--flag=value`, so the CLI can't read it as a flag; so are the session
//! ids of `--resume` and `--session-id`, always.

use core::fmt;
use std::ffi::OsString;

use serde_json::{Map, Value};

/// `--output-format`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutputFormat {
    /// The final answer as plain text.
    Text,
    /// The final `result` as one JSON object.
    Json,
    /// One JSON object per line: what [`crate::protocol`] reads.
    #[default]
    StreamJson,
}

impl OutputFormat {
    /// Its CLI value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::StreamJson => "stream-json",
        }
    }
}

/// `--input-format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputFormat {
    /// The prompt as plain text on stdin.
    Text,
    /// [`crate::protocol::UserInput`]s and control messages, one per line.
    StreamJson,
}

impl InputFormat {
    /// Its CLI value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::StreamJson => "stream-json",
        }
    }
}

/// `--permission-mode`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    /// A classifier decides which tool calls need a prompt.
    Auto,
    BypassPermissions,
    /// Every tool call asks.
    Manual,
    Plan,
    /// Only pre-approved tools run; anything else is denied without asking.
    DontAsk,
    /// A mode this version doesn't know, passed as is.
    Other(String),
}

impl PermissionMode {
    /// Its CLI value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::Auto => "auto",
            Self::BypassPermissions => "bypassPermissions",
            Self::Manual => "manual",
            Self::Plan => "plan",
            Self::DontAsk => "dontAsk",
            Self::Other(s) => s,
        }
    }
}

impl From<&str> for PermissionMode {
    /// The mode named `s` (its CLI value), else [`PermissionMode::Other`].
    fn from(s: &str) -> Self {
        match s {
            "default" => Self::Default,
            "acceptEdits" => Self::AcceptEdits,
            "auto" => Self::Auto,
            "bypassPermissions" => Self::BypassPermissions,
            "manual" => Self::Manual,
            "plan" => Self::Plan,
            "dontAsk" => Self::DontAsk,
            _ => Self::Other(s.to_owned()),
        }
    }
}

impl From<String> for PermissionMode {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which session to run: a new one, or one to carry on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionStart {
    /// A new session with an id of the CLI's choosing.
    #[default]
    New,
    /// A new session with this id (`--session-id`).
    WithId(String),
    /// The session with this id (`--resume`).
    Resume(String),
    /// The cwd's most recent session (`--continue`).
    Continue,
}

/// The `claude` command line of a print-mode session (`claude -p`).
///
/// A builder: each method sets one flag and returns the options. Nothing is
/// checked against the CLI; [`Options::to_argv`] renders what was set, in a
/// fixed order, and [`Options::extra_args`] covers any flag it lacks.
///
/// The defaults are `-p --output-format stream-json --verbose`: what
/// [`crate::protocol`] reads. The prompt goes to stdin, or as the
/// positional [`Options::prompt_arg`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Options {
    input_format: Option<InputFormat>,
    output_format: OutputFormat,
    verbose: Option<bool>,
    include_partial_messages: bool,
    include_hook_events: bool,
    model: Option<String>,
    fallback_model: Option<String>,
    effort: Option<String>,
    restricted: bool,
    permission_mode: Option<PermissionMode>,
    permission_prompts: Option<String>,
    permission_prompt_tool: Option<String>,
    strict_mcp_config: bool,
    mcp_config: Vec<OsString>,
    tools: Option<Vec<String>>,
    allowed_tools: Vec<String>,
    disallowed_tools: Vec<String>,
    disable_slash_commands: bool,
    add_dirs: Vec<OsString>,
    system_prompt: Option<String>,
    append_system_prompt: Option<String>,
    name: Option<String>,
    setting_sources: Option<Vec<String>>,
    settings: Map<String, Value>,
    settings_file: Option<OsString>,
    session: SessionStart,
    fork_session: bool,
    fork_session_id: Option<String>,
    no_session_persistence: bool,
    max_turns: Option<u32>,
    max_budget_usd: Option<f64>,
    extra_args: Vec<OsString>,
    prompt_arg: Option<OsString>,
}

impl Options {
    /// The defaults: `-p --output-format stream-json --verbose`.
    pub fn new() -> Self {
        Self::default()
    }

    // ------------------------------------------------------------ formats

    /// `--input-format` (default: none, the CLI's own: text).
    #[must_use]
    pub fn input_format(mut self, format: InputFormat) -> Self {
        self.input_format = Some(format);
        self
    }

    /// `--output-format` (default: stream-json).
    #[must_use]
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// `--verbose` (default: on with stream-json output, which needs it in
    /// print mode, else off).
    #[must_use]
    pub fn verbose(mut self, on: bool) -> Self {
        self.verbose = Some(on);
        self
    }

    /// `--include-partial-messages`: `stream_event` frames as the reply
    /// streams.
    #[must_use]
    pub fn include_partial_messages(mut self, on: bool) -> Self {
        self.include_partial_messages = on;
        self
    }

    /// `--include-hook-events`: hook lifecycle events in the stream.
    #[must_use]
    pub fn include_hook_events(mut self, on: bool) -> Self {
        self.include_hook_events = on;
        self
    }

    // ------------------------------------------------------------ model

    /// `--model`: an alias (`sonnet`) or a full model name.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// `--fallback-model`: used when the main model is overloaded.
    #[must_use]
    pub fn fallback_model(mut self, model: impl Into<String>) -> Self {
        self.fallback_model = Some(model.into());
        self
    }

    /// `--effort` (`low`, `medium`, `high`, ...).
    #[must_use]
    pub fn effort(mut self, level: impl Into<String>) -> Self {
        self.effort = Some(level.into());
        self
    }

    // ------------------------------------------------------------ permissions

    /// `--restricted`: no code-running built-in tools, file tools only in
    /// the working directories.
    #[must_use]
    pub fn restricted(mut self, on: bool) -> Self {
        self.restricted = on;
        self
    }

    /// `--permission-mode`.
    #[must_use]
    pub fn permission_mode(mut self, mode: impl Into<PermissionMode>) -> Self {
        self.permission_mode = Some(mode.into());
        self
    }

    /// `--permission-prompts`: who answers permission prompts (`host`: the
    /// SDK host or the permission prompt tool; `none`: nobody, they are
    /// denied).
    #[must_use]
    pub fn permission_prompts(mut self, target: impl Into<String>) -> Self {
        self.permission_prompts = Some(target.into());
        self
    }

    /// `--permission-prompt-tool`: the MCP tool that answers permission
    /// prompts, or `stdio` for `can_use_tool` control requests on the
    /// stream-json channel.
    #[must_use]
    pub fn permission_prompt_tool(mut self, tool: impl Into<String>) -> Self {
        self.permission_prompt_tool = Some(tool.into());
        self
    }

    // ------------------------------------------------------------ tools

    /// `--strict-mcp-config`: only the servers of [`Options::mcp_config`].
    #[must_use]
    pub fn strict_mcp_config(mut self, on: bool) -> Self {
        self.strict_mcp_config = on;
        self
    }

    /// One `--mcp-config`: a JSON file's path or a JSON string
    /// (`{"mcpServers": {...}}`). Each call adds one.
    #[must_use]
    pub fn mcp_config(mut self, config: impl Into<OsString>) -> Self {
        self.mcp_config.push(config.into());
        self
    }

    /// `--tools`: the built-in tools available, comma-joined. An empty list
    /// is passed (`--tools ""`): no built-in tools at all.
    #[must_use]
    pub fn tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tools = Some(tools.into_iter().map(Into::into).collect());
        self
    }

    /// `--allowedTools`: tools (or rules, `Bash(git:*)`) that run without
    /// asking, comma-joined. Adds to earlier calls; left out when empty.
    #[must_use]
    pub fn allowed_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_tools.extend(tools.into_iter().map(Into::into));
        self
    }

    /// `--disallowedTools`: tools (or rules) that never run, comma-joined.
    /// Adds to earlier calls; left out when empty.
    #[must_use]
    pub fn disallowed_tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.disallowed_tools
            .extend(tools.into_iter().map(Into::into));
        self
    }

    /// `--disable-slash-commands`: no skills or slash commands.
    #[must_use]
    pub fn disable_slash_commands(mut self, on: bool) -> Self {
        self.disable_slash_commands = on;
        self
    }

    /// One `--add-dir`: another directory the tools may use. Each call adds
    /// one.
    #[must_use]
    pub fn add_dir(mut self, dir: impl Into<OsString>) -> Self {
        self.add_dirs.push(dir.into());
        self
    }

    // ------------------------------------------------------------ prompt

    /// `--system-prompt`: replaces Claude Code's system prompt.
    #[must_use]
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// `--append-system-prompt`: added after the system prompt.
    #[must_use]
    pub fn append_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.append_system_prompt = Some(prompt.into());
        self
    }

    /// `--name`: the session's display name.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    // ------------------------------------------------------------ settings

    /// `--setting-sources`: which settings files load (`user`, `project`,
    /// `local`), comma-joined. An empty list is passed
    /// (`--setting-sources ""`): none of them.
    #[must_use]
    pub fn setting_sources<I, S>(mut self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.setting_sources = Some(sources.into_iter().map(Into::into).collect());
        self
    }

    /// Top-level `--settings` keys, merged into those set before (a key set
    /// again is replaced whole).
    #[must_use]
    pub fn settings(mut self, settings: Map<String, Value>) -> Self {
        self.settings.extend(settings);
        self
    }

    /// One top-level `--settings` key (replaced whole if set before).
    #[must_use]
    pub fn setting(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.settings.insert(key.into(), value.into());
        self
    }

    /// One variable of the `--settings` `env` object (Claude Code sets it
    /// for every command it runs). Merged into the `env` set before; an
    /// `env` that isn't an object is replaced.
    #[must_use]
    pub fn setting_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let env = self
            .settings
            .entry("env")
            .or_insert_with(|| Value::Object(Map::new()));
        if !env.is_object() {
            *env = Value::Object(Map::new());
        }
        if let Value::Object(env) = env {
            env.insert(key.into(), Value::String(value.into()));
        }
        self
    }

    /// `--settings` from a JSON file. The CLI takes one `--settings`: keys
    /// set with [`Options::settings`], [`Options::setting`] or
    /// [`Options::setting_env`] win, and the file is then left out.
    #[must_use]
    pub fn settings_file(mut self, path: impl Into<OsString>) -> Self {
        self.settings_file = Some(path.into());
        self
    }

    // ------------------------------------------------------------ session

    /// `--resume=<id>`: carry on that session.
    #[must_use]
    pub fn resume(mut self, session_id: impl Into<String>) -> Self {
        self.session = SessionStart::Resume(session_id.into());
        self
    }

    /// `--continue`: carry on the cwd's most recent session.
    #[must_use]
    pub fn continue_last(mut self) -> Self {
        self.session = SessionStart::Continue;
        self
    }

    /// `--session-id=<id>`: a new session with this id (a UUID).
    #[must_use]
    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session = SessionStart::WithId(session_id.into());
        self
    }

    /// Which session to run, as a value (replaces [`Options::resume`],
    /// [`Options::continue_last`] and [`Options::session_id`]).
    #[must_use]
    pub fn session(mut self, start: SessionStart) -> Self {
        self.session = start;
        self
    }

    /// `--fork-session`: a resumed or continued session goes on under a new
    /// id. Left out on a new session. Turning it off also drops a
    /// [`Options::fork_session_id`].
    #[must_use]
    pub fn fork_session(mut self, on: bool) -> Self {
        self.fork_session = on;
        if !on {
            self.fork_session_id = None;
        }
        self
    }

    /// `--fork-session --session-id=<id>`: a resumed or continued session
    /// goes on under this new id (a UUID). Left out on a new session.
    #[must_use]
    pub fn fork_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.fork_session = true;
        self.fork_session_id = Some(session_id.into());
        self
    }

    /// `--no-session-persistence`: nothing saved to disk; it can't be
    /// resumed.
    #[must_use]
    pub fn no_session_persistence(mut self, on: bool) -> Self {
        self.no_session_persistence = on;
        self
    }

    // ------------------------------------------------------------ limits

    /// `--max-turns`.
    #[must_use]
    pub fn max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }

    /// `--max-budget-usd`, rendered in cents (`{:.2}`: `1.5` gives `1.50`).
    ///
    /// # Panics
    ///
    /// If `usd` is negative, NaN or infinite.
    #[must_use]
    pub fn max_budget_usd(mut self, usd: f64) -> Self {
        assert!(
            usd.is_finite() && usd >= 0.0,
            "max_budget_usd: {usd} is not a finite, non-negative amount"
        );
        self.max_budget_usd = Some(usd);
        self
    }

    // ------------------------------------------------------------ escape hatch

    /// Arguments added after the flags, as they are: a flag these options
    /// lack. Not for the prompt: a variadic flag before them (`--add-dir`,
    /// `--mcp-config`, `--tools`, `--allowedTools`, `--disallowedTools`)
    /// would take it as one more value; use [`Options::prompt_arg`].
    #[must_use]
    pub fn extra_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.extra_args.extend(args.into_iter().map(Into::into));
        self
    }

    /// The prompt as a positional argument, after `--` at the very end, so
    /// no variadic flag takes it and a prompt starting with `-` isn't read
    /// as a flag. For a one-shot `text`-input session; with stream-json
    /// input the prompt goes to stdin.
    #[must_use]
    pub fn prompt_arg(mut self, prompt: impl Into<OsString>) -> Self {
        self.prompt_arg = Some(prompt.into());
        self
    }

    // ------------------------------------------------------------ render

    /// The `--settings` JSON these options pass, if any (the keys set; a
    /// [`Options::settings_file`] is not read).
    pub fn settings_json(&self) -> Option<String> {
        (!self.settings.is_empty()).then(|| Value::Object(self.settings.clone()).to_string())
    }

    /// The arguments after the program name, in a fixed order: `-p`, the
    /// formats, the model, permissions, MCP and tools, directories, the
    /// prompts, settings, the session, limits, [`Options::extra_args`],
    /// then `--` and [`Options::prompt_arg`].
    pub fn to_argv(&self) -> Vec<OsString> {
        let mut a = Argv(Vec::new());
        a.flag("-p");
        if let Some(f) = self.input_format {
            a.value("--input-format", f.as_str());
        }
        a.value("--output-format", self.output_format.as_str());
        if self
            .verbose
            .unwrap_or(self.output_format == OutputFormat::StreamJson)
        {
            a.flag("--verbose");
        }
        a.switch("--include-partial-messages", self.include_partial_messages);
        a.switch("--include-hook-events", self.include_hook_events);
        a.opt("--model", self.model.as_deref());
        a.opt("--fallback-model", self.fallback_model.as_deref());
        a.opt("--effort", self.effort.as_deref());
        a.switch("--restricted", self.restricted);
        a.opt(
            "--permission-mode",
            self.permission_mode.as_ref().map(PermissionMode::as_str),
        );
        a.opt("--permission-prompts", self.permission_prompts.as_deref());
        a.opt(
            "--permission-prompt-tool",
            self.permission_prompt_tool.as_deref(),
        );
        a.switch("--strict-mcp-config", self.strict_mcp_config);
        for c in &self.mcp_config {
            a.value("--mcp-config", c);
        }
        if let Some(t) = &self.tools {
            a.value("--tools", t.join(","));
        }
        if !self.allowed_tools.is_empty() {
            a.value("--allowedTools", self.allowed_tools.join(","));
        }
        if !self.disallowed_tools.is_empty() {
            a.value("--disallowedTools", self.disallowed_tools.join(","));
        }
        a.switch("--disable-slash-commands", self.disable_slash_commands);
        for d in &self.add_dirs {
            a.value("--add-dir", d);
        }
        a.opt("--system-prompt", self.system_prompt.as_deref());
        a.opt(
            "--append-system-prompt",
            self.append_system_prompt.as_deref(),
        );
        a.opt("--name", self.name.as_deref());
        if let Some(s) = &self.setting_sources {
            a.value("--setting-sources", s.join(","));
        }
        if let Some(s) = self.settings_json() {
            a.value("--settings", s);
        } else if let Some(f) = &self.settings_file {
            a.value("--settings", f);
        }
        let resumed = match &self.session {
            SessionStart::New => false,
            SessionStart::WithId(id) => {
                a.joined("--session-id", id);
                false
            }
            SessionStart::Resume(id) => {
                a.joined("--resume", id);
                true
            }
            SessionStart::Continue => {
                a.flag("--continue");
                true
            }
        };
        if resumed && self.fork_session {
            a.flag("--fork-session");
            if let Some(id) = &self.fork_session_id {
                a.joined("--session-id", id);
            }
        }
        a.switch("--no-session-persistence", self.no_session_persistence);
        if let Some(n) = self.max_turns {
            a.value("--max-turns", n.to_string());
        }
        if let Some(usd) = self.max_budget_usd {
            a.value("--max-budget-usd", format!("{usd:.2}"));
        }
        a.0.extend(self.extra_args.iter().cloned());
        if let Some(p) = &self.prompt_arg {
            a.flag("--");
            a.0.push(p.clone());
        }
        a.0
    }
}

/// An argv being built.
struct Argv(Vec<OsString>);

impl Argv {
    fn flag(&mut self, flag: &str) {
        self.0.push(flag.into());
    }

    fn switch(&mut self, flag: &str, on: bool) {
        if on {
            self.flag(flag);
        }
    }

    /// `flag value`, or `flag=value` when the value starts with `-` (it
    /// would read as a flag).
    fn value(&mut self, flag: &str, value: impl Into<OsString>) {
        let value = value.into();
        if value.as_encoded_bytes().starts_with(b"-") {
            self.joined(flag, value);
        } else {
            self.0.extend([flag.into(), value]);
        }
    }

    fn opt(&mut self, flag: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.value(flag, v);
        }
    }

    /// `flag=value`, one argument.
    fn joined(&mut self, flag: &str, value: impl Into<OsString>) {
        let mut a = OsString::from(flag);
        a.push("=");
        a.push(value.into());
        self.0.push(a);
    }
}

#[cfg(test)]
mod tests;
