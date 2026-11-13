//! The other objects: attribution, status line, spinner, worktree, SSH,
//! voice, footer links, policy helper, spellcheck, remote.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::Extra;
use crate::enums::{BgIsolation, SpellChecker, SpinnerVerbsMode, VoiceMode, WorktreeBaseRef};
use crate::walk::{Unknown, walk};

/// `attribution`: what Claude Code adds to commits and pull requests.
///
/// Customize the attribution Claude Code adds to git commits and pull requests. Commits get a
/// [git trailer](https://git-scm.com/docs/git-interpret-trailers) such as `Co-Authored-By` by
/// default; pull request descriptions get plain text.
///
/// Docs: [`attribution`](https://code.claude.com/docs/en/settings-reference#attribution).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Attribution {
    /// Set the attribution text Claude Code adds to git commits, including any trailers. Set it
    /// to an empty string to hide commit attribution.
    ///
    /// - **Values**: string
    /// - **Default**: unset, so Claude Code adds `Co-Authored-By: <name>
    ///   <noreply@anthropic.com>`. The name is the model in use when the commit is made, such
    ///   as `Claude Sonnet 5`. When a [subagent](https://code.claude.com/docs/en/sub-agents)
    ///   makes the commit, the trailer names the subagent's model.   * When Claude Code
    ///   recognizes the model as a Claude model but can't confirm its exact version, it writes
    ///   `Claude` alone.   * When it can't match the model ID to any Claude model, such as a
    ///   third-party model served through a custom
    ///   [`ANTHROPIC_BASE_URL`](https://code.claude.com/docs/en/env-vars), it writes `Claude
    ///   Code`.
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`commit`](https://code.claude.com/docs/en/settings-reference#attribution-commit).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// Set the attribution text Claude Code adds to pull request descriptions. Set it to an
    /// empty string to hide pull request attribution.
    ///
    /// - **Values**: string
    /// - **Default**: unset, so Claude Code adds `🤖 Generated with [Claude
    ///   Code](https://claude.com/claude-code)`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`pr`](https://code.claude.com/docs/en/settings-reference#attribution-pr).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr: Option<String>,
    /// Choose whether Claude Code appends the claude.ai session link when it commits or opens a
    /// pull request from a [cloud](https://code.claude.com/docs/en/claude-code-on-the-web) or
    /// [Remote Control](https://code.claude.com/docs/en/remote-control) session. Claude Code
    /// adds the link as a `Claude-Session` trailer on commits and as a link in pull request
    /// descriptions. Set it to `false` to omit the link.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code appends the claude.ai session link when it commits or opens a
    ///     pull request from a cloud or Remote Control session
    ///   - `false`: Claude Code omits the link
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`sessionUrl`](https://code.claude.com/docs/en/settings-reference#attribution-sessionurl).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_url: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `attribution`: the object, or `false` to hide all attribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum AttributionSetting {
    /// `false` hides all attribution.
    Flag(bool),
    /// What to add.
    Custom(Attribution),
}

impl Unknown for AttributionSetting {
    fn unknown(&self, path: &str, out: &mut Vec<String>) {
        if let Self::Custom(a) = self {
            a.unknown(path, out);
        }
    }
}

/// `statusLine`: a command that prints the status line from the session
/// JSON on its stdin.
///
/// Run your own command to render a [status line](https://code.claude.com/docs/en/statusline)
/// below the prompt with context such as the model, cost, or git branch. Optional fields adjust
/// spacing, add periodic re-runs, and hide the built-in vim mode indicator when your script
/// renders `vim.mode` itself.
///
/// Docs: [`statusLine`](https://code.claude.com/docs/en/settings-reference#statusline).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct StatusLine {
    /// The type of status line handler; must be set to "command" to run a custom shell script
    /// that receives JSON session data via stdin.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`type`](https://code.claude.com/docs/en/settings-reference#statusline).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// A shell command or path to a script that displays session information (context usage,
    /// costs, git status, etc.) by reading JSON data from stdin and writing output to stdout.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`command`](https://code.claude.com/docs/en/settings-reference#statusline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Optional number of extra horizontal spacing characters added to the status line content;
    /// defaults to 0.
    ///
    /// - **Values**: a number
    /// - **Default**: unset
    ///
    /// Docs: [`padding`](https://code.claude.com/docs/en/settings-reference#statusline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    /// Re-run the status line command every N seconds in addition to event-driven updates.
    /// Leave unset to run only on events. Useful for time-based data like clocks, or when
    /// background subagents change git state while the main session is idle.
    ///
    /// - **Values**: a whole number, minimum `1`
    /// - **Default**: unset
    ///
    /// Docs: [`refreshInterval`](https://code.claude.com/docs/en/settings-reference#statusline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval: Option<u64>,
    /// Set to true when your status line script renders the vim mode indicator itself, to
    /// suppress the built-in vim mode display.
    ///
    /// - **Values**: Boolean
    /// - **Default**: unset
    ///
    /// Docs: [`hideVimModeIndicator`](https://code.claude.com/docs/en/settings-reference#statusline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_vim_mode_indicator: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

impl StatusLine {
    /// A `command` status line.
    pub fn command(command: impl Into<String>) -> Self {
        Self {
            kind: Some("command".into()),
            command: Some(command.into()),
            ..Self::default()
        }
    }
}

/// `fileSuggestion` and `subagentStatusLine`: a `command` handler.
///
/// Run your own command to supply `@` file path autocomplete instead of the built-in file
/// suggestion. The built-in suggestion uses fast filesystem traversal; a large monorepo may do
/// better with project-specific indexing such as a pre-built file index.
///
/// Docs: [`fileSuggestion`](https://code.claude.com/docs/en/settings-reference#filesuggestion).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CommandSetting {
    /// The type of file suggestion handler; must be set to "command" to execute a custom shell
    /// script that generates file suggestions for the @ file picker.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`type`](https://code.claude.com/docs/en/settings-reference#filesuggestion).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Shell command to execute for file suggestions.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`command`](https://code.claude.com/docs/en/settings-reference#filesuggestion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

impl CommandSetting {
    /// A `command` handler.
    pub fn command(command: impl Into<String>) -> Self {
        Self {
            kind: Some("command".into()),
            command: Some(command.into()),
            ..Self::default()
        }
    }
}

/// `spinnerVerbs`.
///
/// While a turn is in progress, the spinner shows a rotating verb such as "Accomplishing",
/// "Architecting", or "Baking". Use this key to add your own verbs to that rotation or replace
/// the built-in list with yours.
///
/// Docs: [`spinnerVerbs`](https://code.claude.com/docs/en/settings-reference#spinnerverbs).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SpinnerVerbs {
    /// How to combine custom verbs with default spinner verbs: 'append' adds custom verbs to
    /// the default list, 'replace' uses only custom verbs.
    ///
    /// - **Values**: `"append"`, `"replace"`
    /// - **Default**: unset
    ///
    /// Docs: [`mode`](https://code.claude.com/docs/en/settings-reference#spinnerverbs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<SpinnerVerbsMode>,
    /// Custom verbs used in spinner progress text.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`verbs`](https://code.claude.com/docs/en/settings-reference#spinnerverbs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbs: Option<Vec<String>>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `spinnerTipsOverride`.
///
/// Add your own tips to the [spinner
/// tips](https://code.claude.com/docs/en/settings-reference#spinnertipsenabled) that Claude
/// Code shows while Claude works, or replace the built-in tips with yours. Claude Code puts
/// your tips in the same rotation as the built-in ones.
///
/// Docs: [`spinnerTipsOverride`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SpinnerTips {
    /// If true, only show custom tips. If false or absent, custom tips merge with built-in
    /// tips.
    ///
    /// - **Values**: Boolean
    /// - **Default**: `false`
    ///
    /// Docs: [`excludeDefault`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_default: Option<bool>,
    /// Custom tip strings to display in the spinner.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`tips`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tips: Option<Vec<SpinnerTip>>,
    /// A local JSON file holding an array of tips (or an object with a `tips` array), up to 256
    /// KB. Claude Code reads it once per process. Not settable through server-managed settings.
    ///
    /// - **Values**: string, an absolute or `~/` path
    /// - **Default**: unset
    ///
    /// Docs: [`tipsFile`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tips_file: Option<String>,
    /// The prefix shown before tips from user, `--settings`, and managed settings; tips from
    /// project and local settings always use `Tip`.
    ///
    /// - **Values**: string, up to 40 characters
    /// - **Default**: `"Tip"`
    ///
    /// Docs: [`label`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `spinnerTipsOverride.tips`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum SpinnerTip {
    /// The tip's text.
    Text(String),
    /// A tip with an id, a cooldown and a priority.
    Entry(TipEntry),
}

impl Unknown for SpinnerTip {
    fn unknown(&self, path: &str, out: &mut Vec<String>) {
        if let Self::Entry(t) = self {
            t.unknown(path, out);
        }
    }
}

/// A tip object.
///
/// Docs: [`spinnerTipsOverride.tips`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TipEntry {
    /// Up to 64 letters, digits, `.`, `_`, or `-`. Claude Code keys the tip's show history on
    /// it, so the tip's cooldown survives reordering the list. Of two entries with the same id,
    /// Claude Code uses the first.
    ///
    /// - **Values**: string, up to 64 letters, digits, `.`, `_`, or `-` (required)
    /// - **Default**: a position-based id
    ///
    /// Docs: [`id`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The tip, one line of up to 500 characters. Claude Code strips ANSI escapes and control
    /// characters and collapses whitespace.
    ///
    /// - **Values**: string, one line of up to 500 characters (required)
    /// - **Default**: none: required
    ///
    /// Docs: [`text`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Sessions Claude Code waits before showing the tip again, `0` to `1000`, default `0`.
    ///
    /// - **Values**: a whole number, `0` to `1000`
    /// - **Default**: `0`
    ///
    /// Docs: [`cooldownSessions`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cooldown_sessions: Option<u64>,
    /// Order among tips that have gone unshown equally long, higher first, `-10` to `10`,
    /// default `0`.
    ///
    /// - **Values**: a whole number, `-10` to `10`
    /// - **Default**: `0`
    ///
    /// Docs: [`priority`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `managedMcpServers`: MCP server configurations, by name or as a list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum ManagedMcpServers {
    /// Server name → its `.mcp.json` configuration.
    ByName(BTreeMap<String, Extra>),
    /// A list of configurations.
    List(Vec<Extra>),
}

/// `worktree`.
///
/// Configure how Claude Code creates and manages [git
/// worktrees](https://code.claude.com/docs/en/worktrees) for `--worktree`, the `EnterWorktree`
/// tool, and isolated subagents and background sessions.
///
/// Docs: [`worktree`](https://code.claude.com/docs/en/settings-reference#worktree).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Worktree {
    /// Check out only the listed directories in each worktree through git sparse-checkout.
    /// Claude Code writes only those directories plus root-level files to disk, which is faster
    /// in large monorepos; see [Check out only the directories you
    /// need](https://code.claude.com/docs/en/large-codebases#check-out-only-the-directories-you-need).
    ///
    /// - **Values**: array of strings, directory paths relative to the repository root
    /// - **Default**: unset, so each worktree checks out the whole tree
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`sparsePaths`](https://code.claude.com/docs/en/settings-reference#worktree-sparsepaths).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_paths: Option<Vec<String>>,
    /// Choose which ref new worktrees branch from. `"fresh"` branches from
    /// `origin/<default-branch>` for a clean tree matching the remote; `"head"` branches from
    /// your current local `HEAD`, so unpushed commits and feature-branch state are present in
    /// the worktree.
    ///
    /// - **Values**: string, one of:
    ///   - `"fresh"`: new worktrees branch from `origin/<default-branch>`
    ///   - `"head"`: new worktrees branch from your current local `HEAD`, including unpushed
    ///     commits
    /// - **Default**: `"fresh"`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`baseRef`](https://code.claude.com/docs/en/settings-reference#worktree-baseref).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<WorktreeBaseRef>,
    /// Choose how [background
    /// sessions](https://code.claude.com/docs/en/agent-view#how-file-edits-are-isolated)
    /// isolate their file edits. If you moved a session to the background with `←` or
    /// `/background`, that session edits files in place whatever this key says. With
    /// `"worktree"`, Claude Code blocks `Edit` and `Write` in the main checkout until the
    /// session calls `EnterWorktree`; with `"none"`, background jobs edit the working copy
    /// directly.
    ///
    /// - **Values**: string, one of:
    ///   - `"worktree"`: Claude Code blocks `Edit` and `Write` in the main checkout until the
    ///     session calls `EnterWorktree`
    ///   - `"none"`: background jobs edit the working copy directly
    /// - **Default**: `"worktree"`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`bgIsolation`](https://code.claude.com/docs/en/settings-reference#worktree-bgisolation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_isolation: Option<BgIsolation>,
    /// Symlink directories from the main repository into each worktree so you don't duplicate
    /// large directories on disk.
    ///
    /// - **Values**: array of strings, directory paths relative to the repository root
    /// - **Default**: unset, so Claude Code symlinks no directories
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`symlinkDirectories`](https://code.claude.com/docs/en/settings-reference#worktree-symlinkdirectories).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symlink_directories: Option<Vec<String>>,
    /// Choose the folder on a remote machine where [Desktop SSH
    /// sessions](https://code.claude.com/docs/en/desktop#choose-where-ssh-session-worktrees-go)
    /// create their worktrees, instead of `<project-root>/.claude/worktrees/`. Only the desktop
    /// app reads this key: `--worktree`, the `EnterWorktree` tool, isolated subagents, and
    /// background sessions ignore it. Requires Claude Desktop v1.44121.0 or later.
    ///
    /// - **Values**: string, an absolute path or one that starts with `~/`
    /// - **Default**: unset, so worktrees go inside the project
    /// - **Scope**: user, in `~/.claude/settings.json` on the remote machine.
    ///
    /// Docs: [`location`](https://code.claude.com/docs/en/settings-reference#worktree-location).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sshConfigs`.
///
/// Add SSH connections to the
/// [Desktop](https://code.claude.com/docs/en/desktop#pre-configure-ssh-connections-for-your-team)
/// environment selector. Administrators use `sshConfigs` to distribute shared connections to a
/// team.
///
/// Docs: [`sshConfigs`](https://code.claude.com/docs/en/settings-reference#sshconfigs).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SshConfig {
    /// Unique identifier for this SSH config, used to match configs across settings sources.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`id`](https://code.claude.com/docs/en/settings-reference#sshconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Display name for the SSH connection.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`name`](https://code.claude.com/docs/en/settings-reference#sshconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// SSH host in the form "user@hostname" or "hostname", or a host alias from ~/.ssh/config.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`sshHost`](https://code.claude.com/docs/en/settings-reference#sshconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host: Option<String>,
    /// SSH port (default 22).
    ///
    /// - **Values**: a whole number
    /// - **Default**: unset
    ///
    /// Docs: [`sshPort`](https://code.claude.com/docs/en/settings-reference#sshconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_port: Option<u16>,
    /// Path to the SSH identity file (private key).
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`sshIdentityFile`](https://code.claude.com/docs/en/settings-reference#sshconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_identity_file: Option<String>,
    /// Default working directory on the remote host; supports tilde expansion. Defaults to the
    /// remote user home directory.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`startDirectory`](https://code.claude.com/docs/en/settings-reference#sshconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_directory: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sshHostCommands`: a desktop SSH connection opened by a
/// command.
///
/// Add SSH connections whose host comes from a command to the
/// [Desktop](https://code.claude.com/docs/en/desktop#create-an-ssh-connection-from-a-command)
/// environment selector. When you create a connection or create another machine for it, Desktop
/// runs the entry's command on your computer and connects to the host that the command prints.
///
/// Docs: [`sshHostCommands`](https://code.claude.com/docs/en/settings-reference#sshhostcommands).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SshHostCommand {
    /// The connection's identifier.
    ///
    /// - **Values**: string (required)
    /// - **Default**: unset
    ///
    /// Docs: [`id`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The connection's display name.
    ///
    /// - **Values**: string (required)
    /// - **Default**: unset
    ///
    /// Docs: [`name`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The program and its arguments that open the connection. The desktop app does not read it
    /// as shell syntax: no pipes or variables.
    ///
    /// - **Values**: string (required)
    /// - **Default**: unset
    ///
    /// Docs: [`command`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// The private key to use.
    ///
    /// - **Values**: string, a path
    /// - **Default**: unset
    ///
    /// Docs: [`sshIdentityFile`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_identity_file: Option<String>,
    /// The working directory on the remote host.
    ///
    /// - **Values**: string, a path
    /// - **Default**: unset
    ///
    /// Docs: [`startDirectory`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_directory: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `voice`.
///
/// Turn on [voice dictation](https://code.claude.com/docs/en/voice-dictation) and choose how
/// the dictation key behaves. Claude Code writes this object for you when you run `/voice`.
///
/// Docs: [`voice`](https://code.claude.com/docs/en/settings-reference#voice).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Voice {
    /// Enable push-to-talk voice dictation input.
    ///
    /// - **Values**: Boolean
    /// - **Default**: unset
    ///
    /// Docs: [`enabled`](https://code.claude.com/docs/en/settings-reference#voice).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Recording mode: "hold" (default) is push-to-talk; "tap" taps once to start and again to
    /// stop and submit.
    ///
    /// - **Values**: `"hold"`, `"tap"`
    /// - **Default**: unset
    ///
    /// Docs: [`mode`](https://code.claude.com/docs/en/settings-reference#voice).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<VoiceMode>,
    /// In hold mode, auto-submit the prompt on key release when the transcript is at least
    /// three words long.
    ///
    /// - **Values**: Boolean
    /// - **Default**: unset
    ///
    /// Docs: [`autoSubmit`](https://code.claude.com/docs/en/settings-reference#voice).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_submit: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `footerLinksRegexes`.
///
/// Render extra clickable badges in the footer below the input box when a regex matches turn
/// output: tool results, including file contents and fetched pages, and Claude's own responses.
/// Use it to turn IDs printed by project CLIs, such as review tools and issue trackers, into
/// session links.
///
/// Docs: [`footerLinksRegexes`](https://code.claude.com/docs/en/settings-reference#footerlinksregexes).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FooterLink {
    /// Config variant; "regex" matches turn output and builds a URL from named capture groups.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`type`](https://code.claude.com/docs/en/settings-reference#footerlinksregexes).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Regex matched against turn output.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`pattern`](https://code.claude.com/docs/en/settings-reference#footerlinksregexes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Link target; {name} placeholders are filled from named regex capture groups.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`url`](https://code.claude.com/docs/en/settings-reference#footerlinksregexes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Badge text; {name} placeholders filled from named capture groups, defaults to the full
    /// match.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`label`](https://code.claude.com/docs/en/settings-reference#footerlinksregexes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `policyHelper`: a program that computes managed settings.
///
/// Run an executable you deploy that computes managed settings at startup, so you can derive
/// policy from device posture, identity, or a remote service instead of a static file. Claude
/// Code runs the helper before it accepts the first prompt and treats the settings it emits as
/// the managed settings for the session.
///
/// Docs: [`policyHelper`](https://code.claude.com/docs/en/settings-reference#policyhelper).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PolicyHelper {
    /// Name the helper executable Claude Code runs. For what happens when the path breaks the
    /// rules below, see [Helper
    /// failures](https://code.claude.com/docs/en/settings-reference#helper-failures).
    ///
    /// - **Values**: string, an absolute path in normalized form, without `.` or `..` segments;
    ///   on Windows, a drive-letter or UNC path that ends in `.exe`
    /// - **Default**: none; required when `policyHelper` is set
    /// - **Scope**: managed. Read from the macOS plist, the Windows HKLM registry, or the
    ///   managed settings file, wherever policyhelper is read.
    ///
    /// Docs: [`path`](https://code.claude.com/docs/en/settings-reference#policyhelper-path).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Set how long Claude Code waits for the helper before treating the run as failed. A
    /// timed-out run fails the same way as a non-zero exit, so at startup Claude Code refuses
    /// to start.
    ///
    /// - **Values**: integer, milliseconds, minimum `1000`
    /// - **Default**: `10000`
    /// - **Scope**: managed. Read from the macOS plist, the Windows HKLM registry, or the
    ///   managed settings file, wherever policyhelper is read.
    ///
    /// Docs: [`timeoutMs`](https://code.claude.com/docs/en/settings-reference#policyhelper-timeoutms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// Have Claude Code re-run the helper in the background on an interval so policy changes
    /// reach a running session. When a refresh succeeds, its output replaces the previous
    /// managed settings without a restart; when a refresh fails, Claude Code keeps the policy
    /// it already has.
    ///
    /// - **Values**: integer, milliseconds: `0` to disable refresh, otherwise at least `60000`
    /// - **Default**: unset, so Claude Code runs the helper once at startup
    /// - **Scope**: managed. Read from the macOS plist, the Windows HKLM registry, or the
    ///   managed settings file, wherever policyhelper is read.
    ///
    /// Docs: [`refreshIntervalMs`](https://code.claude.com/docs/en/settings-reference#policyhelper-refreshintervalms).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval_ms: Option<u64>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `spellcheck`.
///
/// Underline misspelled words in the prompt input as you type, using a spell checker you
/// install. Claude Code checks only the text in the input box.
///
/// Docs: [`spellcheck`](https://code.claude.com/docs/en/settings-reference#spellcheck).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Spellcheck {
    /// Turn spellchecking of the prompt on.
    ///
    /// - **Values**: Boolean
    /// - **Default**: unset
    ///
    /// Docs: [`enabled`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The spellchecker to run.
    ///
    /// - **Values**: `"aspell"`, `"hunspell"`, `"ispell"`, or `"auto"` for the first of those
    ///   found on `PATH`
    /// - **Default**: `"auto"`
    ///
    /// Docs: [`checker`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checker: Option<SpellChecker>,
    /// The dictionary, passed to the checker as its dictionary name, such as `en_GB`.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`language`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// The color that marks a misspelled word.
    ///
    /// - **Values**: string: a terminal color name, `#rrggbb`, `rgb(r,g,b)`, `ansi256(n)`, or
    ///   `ansi:<name>`
    /// - **Default**: unset
    ///
    /// Docs: [`color`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `remote`: cloud sessions.
///
/// Defaults for cloud sessions.
///
/// Docs: [`remote`](https://code.claude.com/docs/en/settings-reference#remote-defaultenvironmentid).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Remote {
    /// Pick the default [cloud environment](https://code.claude.com/docs/en/cloud-environments)
    /// for cloud sessions you create from the CLI, such as with `claude --cloud`. Claude Code
    /// writes this key to your user settings when you pick an environment with
    /// [`/remote-env`](https://code.claude.com/docs/en/cloud-environments#select-an-environment-from-the-cli).
    ///
    /// - **Values**: string, an environment ID such as `env_...` or `ccpool_...`
    /// - **Default**: unset, so Claude Code uses the Anthropic-hosted environment when your
    ///   list has one, and otherwise the first environment in your list that isn't a [Remote
    ///   Control bridge
    ///   environment](https://code.claude.com/docs/en/cloud-environments#the-default-environment),
    ///   or the first environment when every one is a bridge environment
    /// - **Scope**: any settings file. For a self-hosted environment ID, user or managed
    ///   settings, or the `--settings` flag only.
    ///
    /// Docs: [`defaultEnvironmentId`](https://code.claude.com/docs/en/settings-reference#remote-defaultenvironmentid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_environment_id: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `forceLoginOrgUUID`: one organization or several.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum OneOrMany {
    /// One.
    One(String),
    /// Several.
    Many(Vec<String>),
}

walk!(Attribution {});
walk!(StatusLine {});
walk!(CommandSetting {});
walk!(SpinnerVerbs {});
walk!(SpinnerTips { tips: "tips" });
walk!(TipEntry {});
walk!(Worktree {});
walk!(SshConfig {});
walk!(SshHostCommand {});
walk!(Voice {});
walk!(FooterLink {});
walk!(PolicyHelper {});
walk!(Spellcheck {});
walk!(Remote {});
