//! `permissions`, `autoMode` and the MCP server allow and deny lists.

use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::Extra;
use crate::enums::{DisableFlag, PermissionMode};
use crate::walk::walk;

/// `permissions`: rules and the starting mode.
///
/// Rules use Claude Code's permission-rule syntax (`Bash(git diff:*)`,
/// `Read(./src/**)`, `WebFetch(domain:example.com)`, `mcp__server`).
/// Across layers the lists merge, and a `deny` beats an `allow`.
///
/// Control which tools Claude can use without asking, which ones prompt, and which ones are
/// blocked. Also set the default [permission
/// mode](https://code.claude.com/docs/en/permission-modes) for new sessions.
///
/// Docs: [`permissions`](https://code.claude.com/docs/en/settings-reference#permissions).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Permissions {
    /// List the tool uses Claude Code approves without asking you. In an MCP rule, `*` can
    /// appear only in the tool name after the `mcp__<server>__` prefix, such as
    /// `mcp__github__get_*`; it can't appear in the server name.
    ///
    /// - **Values**: array of permission rule strings
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`allow`](https://code.claude.com/docs/en/settings-reference#permissions-allow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// List the tool uses Claude Code blocks. Use it for files that hold API keys, secrets, or
    /// environment values: Claude Code excludes matching files from file discovery and search
    /// results, denies reads of them, and blocks the [Edit and Write
    /// tools](https://code.claude.com/docs/en/permissions#read-and-edit) on the matching paths.
    /// Read and Edit deny rules apply to Claude's built-in file tools, to file commands Claude
    /// Code recognizes in Bash, such as `cat`, `head`, `tail`, `sed`, and `tee`, and to the
    /// targets of Bash [redirections](https://code.claude.com/docs/en/permissions#redirections)
    /// such as `> file` and `< file`; they don't apply to a command that reads files without
    /// naming them, such as `grep -r pattern .`, or to arbitrary subprocesses, so for OS-level
    /// enforcement [enable the sandbox](https://code.claude.com/docs/en/sandboxing).
    ///
    /// - **Values**: array of permission rule strings
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`deny`](https://code.claude.com/docs/en/settings-reference#permissions-deny).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Vec<String>>,
    /// List the tool uses that prompt you for confirmation even in a permission mode that would
    /// otherwise approve them, such as `acceptEdits` or `bypassPermissions`. In `dontAsk` mode
    /// Claude Code denies a matching tool use instead of prompting.
    ///
    /// - **Values**: array of permission rule strings
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`ask`](https://code.claude.com/docs/en/settings-reference#permissions-ask).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<Vec<String>>,
    /// Set the default [permission mode](https://code.claude.com/docs/en/permission-modes) for
    /// new sessions. When you leave it unset, the [built-in
    /// default](https://code.claude.com/docs/en/permission-modes#which-mode-a-session-starts-in)
    /// for your surface applies.
    ///
    /// - **Values**: string, one of:
    ///   - `"default"`: Claude Code runs only reads without asking
    ///   - `"acceptEdits"`: Claude Code also runs file edits and common filesystem commands
    ///     such as `mkdir` and `mv` without asking
    ///   - `"plan"`: Claude Code reads and plans but blocks edits until you approve a plan
    ///   - `"auto"`: Claude Code runs without routine prompts; before actions such as shell
    ///     commands and network requests run, a background classifier checks that they align
    ///     with your request
    ///   - `"dontAsk"`: Claude Code auto-denies every call that would otherwise prompt, though
    ///     a [mod](https://code.claude.com/docs/en/plugins/mods/overview) you installed can
    ///     still approve [some of those
    ///     calls](https://code.claude.com/docs/en/permissions#precedence-among-rules-hooks-and-mods).
    ///     File reads inside your working directories, other actions that need no approval, and
    ///     pre-approved tools still run, apart from reads from [network
    ///     paths](https://code.claude.com/docs/en/permissions#network-paths)
    ///   - `"bypassPermissions"`: Claude Code runs everything without asking
    ///   - `"manual"`: an alias for `"default"`
    /// - **Default**: unset
    /// - **Scope**: any settings file. `auto` and `bypassPermissions` don't take effect from
    ///   project or local settings, so set them in `~/.claude/settings.json` instead.
    ///
    /// Docs: [`defaultMode`](https://code.claude.com/docs/en/settings-reference#permissions-defaultmode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<PermissionMode>,
    /// Prevent anyone from entering `bypassPermissions` mode. Claude Code then rejects the
    /// `--dangerously-skip-permissions` flag, and ignores an [agent
    /// definition's](https://code.claude.com/docs/en/sub-agents#permission-modes)
    /// `permissionMode: bypassPermissions`, so the subagent runs with the parent session's
    /// permission mode.
    ///
    /// - **Values**: the string `"disable"`
    /// - **Default**: unset
    /// - **Scope**: any settings file. Typically set in [managed
    ///   settings](https://code.claude.com/docs/en/managed-settings) to enforce organizational
    ///   policy.
    ///
    /// Docs: [`disableBypassPermissionsMode`](https://code.claude.com/docs/en/settings-reference#permissions-disablebypasspermissionsmode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_bypass_permissions_mode: Option<DisableFlag>,
    /// Set to "disable" to prevent auto mode from being activated. Removes `auto` from the
    /// Shift+Tab cycle and rejects `--permission-mode auto` at startup. Most useful in managed
    /// settings where users cannot override it.
    ///
    /// - **Values**: `"disable"`
    /// - **Default**: unset
    ///
    /// Docs: [`disableAutoMode`](https://code.claude.com/docs/en/settings-reference#permissions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_auto_mode: Option<DisableFlag>,
    /// Give Claude file access to directories outside the one you started in, as additional
    /// [working directories](https://code.claude.com/docs/en/permissions#working-directories).
    /// Most `.claude/` configuration is [not
    /// discovered](https://code.claude.com/docs/en/permissions#additional-directories-grant-file-access-not-configuration)
    /// from these directories.
    ///
    /// - **Values**: array of directory paths
    /// - **Default**: unset
    /// - **Scope**: any settings file, with [limits on the sandbox write
    ///   access](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox)
    ///   that project and local entries give.
    ///
    /// Docs: [`additionalDirectories`](https://code.claude.com/docs/en/settings-reference#permissions-additionaldirectories).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_directories: Option<Vec<String>>,
    /// Make Claude's file tools refuse reads outside your [working
    /// directories](https://code.claude.com/docs/en/permissions#working-directories) in every
    /// permission mode, including `bypassPermissions`. Claude Code denies `Read`, `Grep`,
    /// `Glob`, and `LSP` calls on those paths and tells Claude to ask you to add the directory
    /// with `/add-dir`. Files Claude Code itself needs stay readable, such as your skills,
    /// plugins, rules, agents, commands, and the `CLAUDE.md` memory file under `~/.claude/`.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude's file tools refuse reads outside the working directories
    ///   - `false`: the same as unset; the block still applies if another file sets `true`
    /// - **Default**: unset, so reads outside the working directories follow your [permission
    ///   mode](https://code.claude.com/docs/en/permission-modes)
    /// - **Scope**: any settings file. A `true` in any file applies, so a repository can turn
    ///   the block on for itself but can't lift yours.
    ///
    /// Docs: [`blockReadsOutsideWorkingDirectories`](https://code.claude.com/docs/en/settings-reference#permissions-blockreadsoutsideworkingdirectories).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reads_outside_working_directories: Option<bool>,
    /// Make the `Write`, `Edit`, and `NotebookEdit` tools refuse paths outside your [working
    /// directories](https://code.claude.com/docs/en/permissions#working-directories) in every
    /// permission mode, including `bypassPermissions`, apart from [some of Claude Code's own
    /// files](https://code.claude.com/docs/en/settings-reference#what-the-write-block-refuses-and-what-stays-writable).
    /// The key doesn't limit shell commands, MCP tools, or the programs hooks run. To limit
    /// where shell commands write, turn on
    /// [sandboxing](https://code.claude.com/docs/en/sandboxing).
    ///
    /// - **Values**: Boolean
    ///   - `true`: turns the block on
    ///   - `false`: the same as unset; the block still applies if another file sets `true`
    /// - **Default**: unset, so writes outside the working directories follow your [permission
    ///   mode](https://code.claude.com/docs/en/permission-modes). The block is also on while
    ///   Claude Code can't read the `permissions` object in your [managed
    ///   settings](https://code.claude.com/docs/en/managed-settings#keys-that-fail-closed).
    /// - **Scope**: any settings file. A `true` in any file the session loads applies, so a
    ///   repository can turn the block on for itself but can't lift yours.
    ///
    /// Docs: [`blockWritesOutsideWorkingDirectories`](https://code.claude.com/docs/en/settings-reference#permissions-blockwritesoutsideworkingdirectories).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_writes_outside_working_directories: Option<bool>,
    /// Set this to `true` to keep a session running when nobody is there to answer a permission
    /// prompt. Claude Code denies a tool call that would otherwise wait on a prompt, and the
    /// turn continues. Requires Claude Code v2.1.319 or later.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code denies calls that would otherwise prompt
    ///   - `false`: the same as unset; calls are still denied if another of these sources sets
    ///     `true`
    /// - **Default**: unset, so a call that needs approval prompts you
    /// - **Scope**: user or managed, and `--settings`. A repository can't set it for you.
    ///
    /// Docs: [`denyInsteadOfPrompting`](https://code.claude.com/docs/en/settings-reference#permissions-denyinsteadofprompting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_instead_of_prompting: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `autoMode`: the auto-mode classifier's sections. Each list replaces
/// the built-in one unless it holds the literal `$defaults`.
///
/// Add your own rules to what the [auto
/// mode](https://code.claude.com/docs/en/permission-modes#eliminate-prompts-with-auto-mode)
/// classifier blocks and allows. Use it to tell the classifier which repos, buckets, and
/// domains your organization trusts, so it stops blocking routine internal operations.
///
/// Docs: [`autoMode`](https://code.claude.com/docs/en/settings-reference#automode).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AutoMode {
    /// Rules for the auto mode classifier allow section. Replaces the built-in allow rules
    /// entirely unless the literal string "$defaults" is included as an entry, which splices
    /// the built-in defaults in at that position.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`allow`](https://code.claude.com/docs/en/settings-reference#automode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// Rules for the auto mode classifier soft-deny section. Replaces the built-in soft-deny
    /// rules entirely unless the literal string "$defaults" is included as an entry, which
    /// splices the built-in defaults in at that position.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`soft_deny`](https://code.claude.com/docs/en/settings-reference#automode).
    #[serde(rename = "soft_deny", skip_serializing_if = "Option::is_none")]
    pub soft_deny: Option<Vec<String>>,
    /// Entries for the auto mode classifier environment section. Replaces the built-in
    /// environment context entirely unless the literal string "$defaults" is included as an
    /// entry, which splices the built-in defaults in at that position.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`environment`](https://code.claude.com/docs/en/settings-reference#automode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<String>>,
    /// Rules for the auto mode classifier hard-deny section. Hard-deny rules block
    /// unconditionally regardless of user intent. Replaces the built-in hard-deny rules
    /// entirely unless the literal string "$defaults" is included as an entry, which splices
    /// the built-in defaults in at that position.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`hard_deny`](https://code.claude.com/docs/en/settings-reference#automode).
    #[serde(rename = "hard_deny", skip_serializing_if = "Option::is_none")]
    pub hard_deny: Option<Vec<String>>,
    /// Send every Bash and PowerShell command through the auto mode classifier while auto mode
    /// is active. By default, auto mode suspends only allow rules that could run arbitrary
    /// code: tool-wide and wildcard rules such as `Bash(*)`, and interpreter or shell-wrapper
    /// prefixes such as `Bash(python *)`. A command that any other allow rule matches, such as
    /// `Bash(npm test)`, skips the classifier unless it carries [per-command allowed
    /// domains](https://code.claude.com/docs/en/sandboxing#per-command-allowed-domains-in-auto-mode).
    ///
    /// - **Values**: Boolean
    ///   - `true`: while auto mode is active, Claude Code sends every Bash and PowerShell
    ///     command through the classifier and suspends your shell allow rules; outside auto
    ///     mode the rules still apply
    ///   - `false`: auto mode suspends only allow rules that could run arbitrary code, such as
    ///     `Bash(*)` and `Bash(python *)`; a command that any other allow rule matches skips
    ///     the classifier unless it carries [per-command allowed
    ///     domains](https://code.claude.com/docs/en/sandboxing#per-command-allowed-domains-in-auto-mode),
    ///     and every other shell command goes through it
    /// - **Default**: `false`
    /// - **Scope**: user or managed. Read wherever automode is read.
    ///
    /// Docs: [`classifyAllShell`](https://code.claude.com/docs/en/settings-reference#automode-classifyallshell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classify_all_shell: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `allowedMcpServers` or `deniedMcpServers`: one of the
/// three fields.
///
/// Allowlist the MCP servers people can add. Claude Code blocks any server that doesn't match
/// an entry wherever it's defined, including plugin servers, servers a user passes with
/// `--mcp-config`, and servers from claude.ai.
///
/// Docs: [`allowedMcpServers`](https://code.claude.com/docs/en/settings-reference#allowedmcpservers).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct McpServerRule {
    /// Name of the MCP server that users are allowed to configure.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`serverName`](https://code.claude.com/docs/en/settings-reference#allowedmcpservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    /// Exact command and arguments used to start stdio servers.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`serverCommand`](https://code.claude.com/docs/en/settings-reference#allowedmcpservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_command: Option<Vec<String>>,
    /// URL pattern for remote servers, supports wildcards (e.g., <https://*.example.com/*>).
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`serverUrl`](https://code.claude.com/docs/en/settings-reference#allowedmcpservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

walk!(Permissions {});
walk!(AutoMode {});
walk!(McpServerRule {});
