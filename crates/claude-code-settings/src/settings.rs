//! [`Settings`]: the settings.json document.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Deserializer, Serialize};

use crate::Extra;
use crate::enums::{
    CacheTtl, DialogTimeout, DiffTool, DisableFlag, DisabledFlag, EditorMode, EffortLevel,
    FeedbackDrafts, InboundMode, KeybindingFlavor, LoginMethod, ModelMatch, NotifChannel, Provider,
    Shell, SkillOverride, SourcesBehavior, TeammateMode, Theme, TimeFormat, Tui, UpdatesChannel,
    ViewMode, WorkflowSize,
};
use crate::hooks::Hooks;
use crate::models::{ModelPicker, ModelPricing, ModelSettings};
use crate::permissions::{AutoMode, McpServerRule, Permissions};
use crate::plugins::{
    ChannelPluginEntry, Marketplace, PluginConfig, PluginEnabled, Source, StrictCustomization,
};
use crate::sandbox::Sandbox;
use crate::types::{
    AttributionSetting, CommandSetting, FooterLink, ManagedMcpServers, OneOrMany, PolicyHelper,
    Remote, Spellcheck, SpinnerTips, SpinnerVerbs, SshConfig, SshHostCommand, StatusLine, Voice,
    Worktree,
};
use crate::walk::{Unknown, walk};

/// Claude Code's settings.json: one settings file, a `--settings`
/// document, or the merge of several.
///
/// Every key is an `Option`, written only when `Some`, so a document says
/// only what it sets and Claude Code's own default applies to the rest;
/// each field's docs give that default. Keys this crate doesn't model (a
/// newer Claude Code's, or undocumented ones) are kept in
/// [`extra`](Self::extra) and written back, so a document round-trips.
///
/// The struct is `#[non_exhaustive]`, so new keys can be added without a
/// breaking release: build one from [`Settings::default()`] and set its
/// fields.
///
/// ```
/// use claude_code_settings::{PermissionMode, Permissions, Settings};
///
/// let mut permissions = Permissions::default();
/// permissions.default_mode = Some(PermissionMode::AcceptEdits);
///
/// let mut settings = Settings::default();
/// settings.model = Some("sonnet".into());
/// settings.permissions = Some(permissions);
/// assert_eq!(
///     settings.to_json_string()?,
///     r#"{"permissions":{"defaultMode":"acceptEdits"},"model":"sonnet"}"#,
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// Docs: [settings reference](https://code.claude.com/docs/en/settings-reference).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Settings {
    /// JSON Schema reference for Claude Code settings.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: not in the settings reference; listed in the
    /// [JSON schema](https://json.schemastore.org/claude-code-settings.json).
    #[serde(skip_serializing_if = "Option::is_none", rename = "$schema")]
    pub schema: Option<String>,
    /// Run your own command to produce the credential Claude Code sends with model requests.
    /// Claude Code runs the command through the system shell, `/bin/sh` on macOS and Linux and
    /// `cmd` on Windows, and sends its output as both the `X-Api-Key` and `Authorization:
    /// Bearer` headers. Use it for dynamic or rotating credentials, such as short-lived tokens
    /// fetched from a vault.
    ///
    /// - **Values**: string, a shell command line
    /// - **Default**: unset, so Claude Code doesn't run a helper
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`apiKeyHelper`](https://code.claude.com/docs/en/settings-reference#apikeyhelper).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_helper: Option<String>,
    /// Turn [auto memory](https://code.claude.com/docs/en/memory#enable-or-disable-auto-memory)
    /// on or off. When `false`, Claude doesn't read from or write to the auto memory directory.
    /// You can also toggle it with `/memory` during a session, which writes this key to your
    /// user settings.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the same as unset; auto memory stays on unless something that outranks this
    ///     key turns it off for the session, such as `--bare`, safe mode, or
    ///     `CLAUDE_CODE_DISABLE_AUTO_MEMORY`
    ///   - `false`: Claude doesn't read from or write to the auto memory directory
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`autoMemoryEnabled`](https://code.claude.com/docs/en/settings-reference#automemoryenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_memory_enabled: Option<bool>,
    /// Choose which [release
    /// channel](https://code.claude.com/docs/en/setup#configure-release-channel) background
    /// auto-updates and `claude update` follow. Set `"stable"` for a version that is typically
    /// about one week old and skips releases with major regressions, or `"latest"` for the most
    /// recent release.
    ///
    /// - **Values**: string, one of:
    ///   - `"latest"`: updates follow the most recent release
    ///   - `"stable"`: updates follow a version that is typically about one week old and skips
    ///     releases with major regressions
    /// - **Default**: unset, so Claude Code follows `"latest"`
    /// - **Scope**: any settings file. Set it in managed settings to enforce one channel across
    ///   your organization.
    ///
    /// Docs: [`autoUpdatesChannel`](https://code.claude.com/docs/en/settings-reference#autoupdateschannel).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_updates_channel: Option<UpdatesChannel>,
    /// Run your own command that prints AWS credentials as JSON, so Claude Code can call
    /// [Amazon Bedrock](https://code.claude.com/docs/en/amazon-bedrock) with credentials that
    /// don't live in your `.aws` directory. Claude Code accepts the `aws sts` output shape and
    /// the flat `aws configure export-credentials` shape, and scopes the credentials to its own
    /// Bedrock client, so the shell commands Claude runs still see your ambient credentials.
    ///
    /// - **Values**: string, a shell command line
    /// - **Default**: unset, so Claude Code uses the ambient AWS credential chain
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`awsCredentialExport`](https://code.claude.com/docs/en/settings-reference#awscredentialexport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_credential_export: Option<String>,
    /// Run your own command, such as `aws sso login`, to refresh the credentials in your `.aws`
    /// directory when the ones Claude Code has for [Amazon
    /// Bedrock](https://code.claude.com/docs/en/amazon-bedrock) stop working. Claude Code
    /// checks the current credentials against STS first and runs the command only when that
    /// check fails, then reads the refreshed `.aws` directory. When the check fails at the same
    /// time in several Claude Code processes that use the same command and credentials, such as
    /// separate terminals or IDE windows, one process runs the command and the rest wait for
    /// that run instead of starting their own.
    ///
    /// - **Values**: string, a shell command line
    /// - **Default**: unset, so Claude Code doesn't refresh AWS credentials for you
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`awsAuthRefresh`](https://code.claude.com/docs/en/settings-reference#awsauthrefresh).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_auth_refresh: Option<String>,
    /// Skip specific `CLAUDE.md` files when Claude Code loads
    /// [memory](https://code.claude.com/docs/en/memory#exclude-specific-claude-md-files). In a
    /// large monorepo, use it to skip CLAUDE.md files from other teams that aren't relevant to
    /// your work; [Exclude irrelevant CLAUDE.md
    /// files](https://code.claude.com/docs/en/large-codebases#exclude-irrelevant-claude-md-files)
    /// in the large-codebases guide walks through that case. Patterns match against absolute
    /// file paths.
    ///
    /// - **Values**: array of strings, each a glob pattern or absolute path
    /// - **Default**: unset, so Claude Code loads every CLAUDE.md it finds
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`claudeMdExcludes`](https://code.claude.com/docs/en/settings-reference#claudemdexcludes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_md_excludes: Option<Vec<String>>,
    /// Set how many days Claude Code keeps [session transcripts and other application
    /// data](https://code.claude.com/docs/en/claude-directory#cleaned-up-automatically) before
    /// deleting them. Claude Code runs the deletion as a background sweep after a session
    /// starts, as long as it can safely determine the retention period. The sweep deletes
    /// transcripts without showing a message, so a session you haven't used for longer than the
    /// retention period no longer appears in the
    /// [`/resume`](https://code.claude.com/docs/en/sessions#resume-a-session) picker.
    ///
    /// - **Values**: number of days, a whole number, minimum `1`
    /// - **Default**: `30`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`cleanupPeriodDays`](https://code.claude.com/docs/en/settings-reference#cleanupperioddays).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_period_days: Option<u64>,
    /// Set environment variables for every session and for the subprocesses Claude Code starts
    /// from it. Most variables in the [environment variables
    /// reference](https://code.claude.com/docs/en/env-vars) can go here, which is how you apply
    /// one to every session or roll it out to your team. Project and local settings can't set
    /// [some of
    /// them](https://code.claude.com/docs/en/settings-reference#variables-claude-code-ignores-in-env).
    ///
    /// - **Values**: object mapping variable names to string values
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`env`](https://code.claude.com/docs/en/settings-reference#env).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<BTreeMap<String, String>>,
    /// Customize the attribution Claude Code adds to git commits and pull requests. Commits get
    /// a [git trailer](https://git-scm.com/docs/git-interpret-trailers) such as
    /// `Co-Authored-By` by default; pull request descriptions get plain text. Set each part
    /// separately with the sub-keys below.
    ///
    /// - **Values**: object with `commit` and `pr` strings and a `sessionUrl` Boolean, or
    ///   `false` to hide all attribution. The `false` value requires Claude Code v2.1.281 or
    ///   later; earlier versions reject it and [skip the whole user, project, or local settings
    ///   file](https://code.claude.com/docs/en/settings#fix-a-broken-settings-file) that holds
    ///   it
    /// - **Default**: unset, so Claude Code uses the standard attribution shown under each
    ///   sub-key
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`attribution`](https://code.claude.com/docs/en/settings-reference#attribution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<AttributionSetting>,
    /// Claude Code gives Claude two git-related pieces of context: its built-in instructions
    /// for how to write commits and pull requests, in the Bash tool's description, and a git
    /// status snapshot of your repository. The snapshot holds the current branch, the main
    /// branch, `git status` output, and recent commits. Claude Code reads it when a
    /// conversation starts.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code includes its built-in commit and pull request workflow
    ///     instructions and the git status snapshot. Cloud sessions never include the snapshot
    ///   - `false`: Claude Code leaves both out
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`includeGitInstructions`](https://code.claude.com/docs/en/settings-reference#includegitinstructions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_git_instructions: Option<bool>,
    /// **Warning:** Deprecated since v2.0.62, when
    /// [`attribution`](https://code.claude.com/docs/en/settings-reference#attribution) replaced
    /// it. Claude Code still reads it, but new configurations should set `attribution`. Use
    /// [`attribution`](https://code.claude.com/docs/en/settings-reference#attribution) instead,
    /// which replaces this key and lets you change or hide the commit trailer, the pull request
    /// text, and the session link separately.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the same as unset; Claude Code adds the commit trailer and the pull request
    ///     attribution text
    ///   - `false`: Claude Code omits both the commit trailer and the pull request attribution
    ///     text, unless `attribution` sets `commit` or `pr`, in which case the
    ///     [`attribution`](https://code.claude.com/docs/en/settings-reference#attribution)
    ///     rules apply
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`includeCoAuthoredBy`](https://code.claude.com/docs/en/settings-reference#includecoauthoredby).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_co_authored_by: Option<bool>,
    /// Choose where Claude Code stores the plan files it writes in [plan
    /// mode](https://code.claude.com/docs/en/permission-modes#analyze-before-you-edit-with-plan-mode).
    /// Claude Code resolves the path relative to the project root.
    ///
    /// - **Values**: string, a path relative to the project root
    /// - **Default**: unset, so Claude Code uses `~/.claude/plans`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`plansDirectory`](https://code.claude.com/docs/en/settings-reference#plansdirectory).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plans_directory: Option<String>,
    /// Control whether the `@` file picker leaves out files that match `.gitignore` patterns.
    /// Appears in `/config` as **Respect .gitignore in file picker**.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the `@` file picker leaves out files that match `.gitignore` patterns
    ///   - `false`: the `@` file picker includes files that match `.gitignore` patterns
    /// - **Default**: `true`
    /// - **Scope**: any settings file. When no settings file sets it, Claude Code falls back to
    ///   `respectGitignore` in `~/.claude.json`, which the `/config` toggle writes.
    ///
    /// Docs: [`respectGitignore`](https://code.claude.com/docs/en/settings-reference#respectgitignore).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respect_gitignore: Option<bool>,
    /// Control which tools Claude can use without asking, which ones prompt, and which ones are
    /// blocked. Also set the default [permission
    /// mode](https://code.claude.com/docs/en/permission-modes) for new sessions. Every
    /// `permissions.*` key below nests under this object.
    ///
    /// - **Values**: object with `allow`, `ask`, `deny`, `additionalDirectories`,
    ///   `blockReadsOutsideWorkingDirectories`, `blockWritesOutsideWorkingDirectories`,
    ///   `defaultMode`, `denyInsteadOfPrompting`, `disableBypassPermissionsMode`, and
    ///   `disableAutoMode`
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`permissions`](https://code.claude.com/docs/en/settings-reference#permissions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// Have Claude respond in a language other than English by default. There is no fixed list
    /// for responses: Claude Code passes the value verbatim to Claude as an instruction to
    /// always respond in that language, so any language name Claude can read works. Claude Code
    /// doesn't check the value, so a misspelled name reaches Claude as written rather than
    /// producing an error.
    ///
    /// - **Values**: string, any language name, such as `"japanese"`, `"spanish"`, or
    ///   `"french"`; Claude Code doesn't validate it
    /// - **Default**: unset; session titles then match the language of your conversation
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`language`](https://code.claude.com/docs/en/settings-reference#language).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Set the model every new session uses, so you don't have to pick one with `/model` each
    /// time. Setting it here doesn't stop you from switching mid-session. If your admin set an
    /// [organization default
    /// model](https://code.claude.com/docs/en/model-config#organization-default-model) to
    /// override user selection, you get that model even when you set this key in user, project,
    /// or local settings.
    ///
    /// - **Values**: string, a model alias or full model ID
    /// - **Default**: unset, so Claude Code uses your account's default model
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`model`](https://code.claude.com/docs/en/settings-reference#model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Restrict which models people can select for the main session,
    /// [subagents](https://code.claude.com/docs/en/sub-agents),
    /// [skills](https://code.claude.com/docs/en/skills), and the
    /// [advisor](https://code.claude.com/docs/en/advisor). A managed list constrains `/model`,
    /// `--model`, and the `model` key in a developer's own files; a model outside it can't be
    /// selected. With the default prefix matching, this doesn't touch the Default option on its
    /// own; pair it with
    /// [`enforceAvailableModels`](https://code.claude.com/docs/en/settings-reference#enforceavailablemodels)
    /// for that.
    ///
    /// - **Values**: array of model aliases or IDs
    /// - **Default**: unset, so every model is available
    /// - **Scope**: any settings file. Deploy it in managed settings to enforce it for an
    ///   organization.
    ///
    /// Docs: [`availableModels`](https://code.claude.com/docs/en/settings-reference#availablemodels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_models: Option<Vec<String>>,
    /// Map Anthropic model IDs to provider-specific model IDs, such as Amazon Bedrock inference
    /// profile ARNs. Each model picker entry then uses its mapped value when calling the
    /// provider API. Administrators use this on [Amazon Bedrock, Google Cloud's Agent Platform,
    /// and Microsoft
    /// Foundry](https://code.claude.com/docs/en/model-config#override-model-ids-per-version) to
    /// route each model version to a specific inference profile, version name, or deployment
    /// for governance, cost allocation, or regional routing.
    ///
    /// - **Values**: object mapping model ID to provider model ID
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`modelOverrides`](https://code.claude.com/docs/en/settings-reference#modeloverrides).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_overrides: Option<BTreeMap<String, String>>,
    /// Set a default [effort
    /// level](https://code.claude.com/docs/en/model-config#adjust-effort-level) for models you
    /// haven't saved a level for. Lower levels are faster and cheaper on straightforward tasks,
    /// and higher levels reason more deeply on complex problems. When you run `/effort low`,
    /// `medium`, `high`, or `xhigh` in an interactive session on your machine, Claude Code
    /// saves the level for the active model under
    /// [`modelSettings`](https://code.claude.com/docs/en/settings-reference#modelsettings)
    /// rather than writing this key.
    ///
    /// - **Values**: string, one of:
    ///   - `"low"`: the least reasoning, for short, scoped, latency-sensitive tasks that aren't
    ///     intelligence-sensitive
    ///   - `"medium"`: reduces token usage for cost-sensitive work that can trade off some
    ///     intelligence
    ///   - `"high"`: balances token usage and intelligence
    ///   - `"xhigh"`: deeper reasoning at higher token spend
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`effortLevel`](https://code.claude.com/docs/en/settings-reference#effortlevel).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_level: Option<EffortLevel>,
    /// Turn [fast mode](https://code.claude.com/docs/en/fast-mode) on for sessions where it's
    /// available, for interactive work such as rapid iteration or live debugging where you want
    /// speed at a higher cost per token. You don't usually edit this key yourself. Running
    /// `/fast` writes `fastMode: true` to `~/.claude/settings.json`, and running it again to
    /// turn fast mode off removes the key.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code turns fast mode on for sessions where it's available. On Fable
    ///     5.2, you must also [run `/fast` while using Fable
    ///     5.2](https://code.claude.com/docs/en/fast-mode#switch-models-while-fast-mode-is-on)
    ///   - `false`: fast mode stays off
    /// - **Default**: unset, so fast mode is off
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`fastMode`](https://code.claude.com/docs/en/settings-reference#fastmode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode: Option<bool>,
    /// Normally, running `/fast` saves
    /// [`fastMode`](https://code.claude.com/docs/en/settings-reference#fastmode) to a person's
    /// user settings, so new sessions on a supported model start with fast mode on. On Fable
    /// 5.2, that happens only if they [turned it on while using Fable
    /// 5.2](https://code.claude.com/docs/en/fast-mode#switch-models-while-fast-mode-is-on). To
    /// make each person run `/fast` in each session they want it, set this key to `true`.
    ///
    /// - **Values**: Boolean
    ///   - `true`: a saved `fastMode: true` no longer turns fast mode on at session start, so
    ///     each person runs `/fast` in each session they want it; a `fastMode: true` passed
    ///     with `--settings` still counts for that session unless managed settings set this key
    ///   - `false`: a saved `fastMode: true` works as
    ///     [`fastMode`](https://code.claude.com/docs/en/settings-reference#fastmode) describes
    /// - **Default**: `false`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`fastModePerSessionOptIn`](https://code.claude.com/docs/en/settings-reference#fastmodepersessionoptin).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode_per_session_opt_in: Option<bool>,
    /// Set the probability that the [session quality
    /// survey](https://code.claude.com/docs/en/data-usage#session-quality-surveys) appears when
    /// a session is eligible for it. Set `0` to keep the survey from appearing.
    ///
    /// - **Values**: number between `0` and `1`
    /// - **Default**: unset, so Claude Code uses the rate Anthropic sets remotely, or its
    ///   built-in rate of `0.005` on Amazon Bedrock, Google Cloud's Agent Platform, and
    ///   Microsoft Foundry, which don't receive remote configuration
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`feedbackSurveyRate`](https://code.claude.com/docs/en/settings-reference#feedbacksurveyrate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_survey_rate: Option<f64>,
    /// Approve every MCP server defined in project `.mcp.json` files without a prompt. Claude
    /// Code writes this key to `.claude/settings.local.json` when you choose to approve all
    /// servers in the approval dialog.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code approves every MCP server defined in project `.mcp.json` files
    ///     without a prompt
    ///   - `false`: Claude Code asks you to approve each server. In a trusted folder, a `false`
    ///     in a higher-precedence file overrides a `true` in a lower one; in a folder you
    ///     haven't trusted, a `true` in any honored file is enough
    /// - **Default**: unset, so Claude Code asks you to approve each server
    /// - **Scope**: any settings file. In a folder whose trust dialog you haven't accepted,
    ///   Claude Code honors it from user settings, managed settings, and `--settings` and
    ///   ignores it in the shared project file, both in the session and for `claude mcp list`
    ///   and `claude mcp get`; [Project server approvals and workspace
    ///   trust](https://code.claude.com/docs/en/mcp#project-server-approvals-and-workspace-trust)
    ///   says when an untracked `.claude/settings.local.json` counts too.
    ///
    /// Docs: [`enableAllProjectMcpServers`](https://code.claude.com/docs/en/settings-reference#enableallprojectmcpservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_all_project_mcp_servers: Option<bool>,
    /// Approve specific servers defined in project `.mcp.json` files so Claude Code connects
    /// them without asking. Claude Code writes this key to `.claude/settings.local.json` when
    /// you approve a server in the approval dialog.
    ///
    /// - **Values**: array of strings, the server names as they appear in `.mcp.json`
    /// - **Default**: unset
    /// - **Scope**: any settings file. In a folder whose trust dialog you haven't accepted,
    ///   Claude Code honors it from user settings, managed settings, and `--settings` and
    ///   ignores it in the shared project file, both in the session and for `claude mcp list`
    ///   and `claude mcp get`; [Project server approvals and workspace
    ///   trust](https://code.claude.com/docs/en/mcp#project-server-approvals-and-workspace-trust)
    ///   says when an untracked `.claude/settings.local.json` counts too.
    ///
    /// Docs: [`enabledMcpjsonServers`](https://code.claude.com/docs/en/settings-reference#enabledmcpjsonservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_mcpjson_servers: Option<Vec<String>>,
    /// Reject specific servers defined in a project's `.mcp.json` file so Claude Code never
    /// connects them or asks you to approve them. A rejection in any settings file applies,
    /// including a project `.claude/settings.json` checked into the repository.
    ///
    /// - **Values**: array of strings, the server names as they appear in `.mcp.json`
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`disabledMcpjsonServers`](https://code.claude.com/docs/en/settings-reference#disabledmcpjsonservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_mcpjson_servers: Option<Vec<String>>,
    /// Allowlist the MCP servers people can add. Claude Code blocks any server that doesn't
    /// match an entry wherever it's defined, including plugin servers, servers a user passes
    /// with `--mcp-config`, and servers from claude.ai. Built-in servers such as Claude in
    /// Chrome, the `ide` server Claude Code connects to in a running [VS
    /// Code](https://code.claude.com/docs/en/vs-code#the-built-in-ide-mcp-server) or
    /// [JetBrains](https://code.claude.com/docs/en/jetbrains#the-built-in-ide-mcp-server) IDE,
    /// and servers the CLI itself configures are exempt from the allowlist, and the denylist
    /// still applies to them.
    ///
    /// - **Values**: array of objects, each with exactly one key. Use `serverName`, a string
    ///   limited to letters, numbers, hyphens, and underscores; `serverCommand`, an array of
    ///   the command and its arguments matched exactly; or `serverUrl`, a URL pattern with `*`
    ///   wildcards
    /// - **Default**: unset, so every server is allowed; an empty array blocks every server
    ///   users add
    /// - **Scope**: any settings file. Entries from every file merge into one allowlist unless
    ///   allowmanagedmcpserversonly is set.
    ///
    /// Docs: [`allowedMcpServers`](https://code.claude.com/docs/en/settings-reference#allowedmcpservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mcp_servers: Option<Vec<McpServerRule>>,
    /// Block specific MCP servers. Claude Code refuses to load a matching server wherever it's
    /// defined, including plugin servers, servers passed with `--mcp-config`, servers from
    /// `managed-mcp.json`, servers from
    /// [`managedMcpServers`](https://code.claude.com/docs/en/settings-reference#managedmcpservers),
    /// and the claude.ai connectors [it fetches
    /// itself](https://code.claude.com/docs/en/mcp#how-connectors-reach-claude-code).
    /// In-process `type: "sdk"` servers are exempt; the app that started the session registers
    /// them.
    ///
    /// - **Values**: array of objects, each with exactly one key: `serverName`, a string, so a
    ///   claude.ai connector's display name such as `"claude.ai Slack"` works; `serverCommand`,
    ///   an array of the command and its arguments matched exactly; or `serverUrl`, a URL
    ///   pattern with `*` wildcards
    /// - **Default**: unset, so no server is blocked; an empty array also blocks nothing
    /// - **Scope**: any settings file. Entries from every file merge into one denylist, and
    ///   allowmanagedmcpserversonly doesn't change that.
    ///
    /// Docs: [`deniedMcpServers`](https://code.claude.com/docs/en/settings-reference#deniedmcpservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_mcp_servers: Option<Vec<McpServerRule>>,
    /// An [HTTP hook](https://code.claude.com/docs/en/hooks#http-hook-fields) can put the value
    /// of an environment variable into a request header, for example an `Authorization: Bearer
    /// $HOOK_TOKEN` header, but only for variables the hook lists in its own `allowedEnvVars`.
    /// This key sets an outer limit on that list for every HTTP hook: a hook can use a variable
    /// only if both its own `allowedEnvVars` and this key name it. Use it to stop a hook from
    /// reading a secret it shouldn't, even when the hook's definition asks for it.
    ///
    /// - **Values**: array of environment variable names
    /// - **Default**: unset, so each hook's own `allowedEnvVars` list applies
    /// - **Scope**: any settings file. Arrays merge across settings files.
    ///
    /// Docs: [`httpHookAllowedEnvVars`](https://code.claude.com/docs/en/settings-reference#httphookallowedenvvars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_hook_allowed_env_vars: Option<Vec<String>>,
    /// Run your own commands, prompts, agents, HTTP requests, or MCP tools as
    /// [hooks](https://code.claude.com/docs/en/hooks) at points in Claude Code's lifecycle,
    /// such as before a tool call or when a session starts; the [hooks
    /// reference](https://code.claude.com/docs/en/hooks#hook-events) lists every event, its
    /// payload, and its exit codes. Each event maps to a list of matcher groups, and each group
    /// lists the handlers to run when the matcher applies.
    ///
    /// - **Values**: object keyed by [hook
    ///   event](https://code.claude.com/docs/en/hooks#hook-events); each value is an array of
    ///   `{ "matcher", "hooks" }` groups whose `hooks` entries have a `type` of `"command"`,
    ///   `"prompt"`, `"agent"`, `"http"`, or `"mcp_tool"`
    /// - **Default**: unset, so no hooks run
    /// - **Scope**: any settings file. Hooks merge across files rather than replacing each
    ///   other, and hooks from managed settings can't be removed from other files.
    ///
    /// Docs: [`hooks`](https://code.claude.com/docs/en/settings-reference#hooks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Hooks>,
    /// Turn off [hooks](https://code.claude.com/docs/en/hooks#disable-or-remove-hooks), any
    /// custom [status line](https://code.claude.com/docs/en/statusline), and any custom [file
    /// suggestion](https://code.claude.com/docs/en/settings-reference#filesuggestion) command.
    /// Use it to turn all of these off temporarily without deleting them from your settings.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code turns off hooks, any custom status line, and any custom file
    ///     suggestion command
    ///   - `false`: hooks, the status line, and the file suggestion command run
    /// - **Default**: unset, so hooks run
    /// - **Scope**: any settings file. Only managed settings can disable managed hooks.
    ///
    /// Docs: [`disableAllHooks`](https://code.claude.com/docs/en/settings-reference#disableallhooks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_all_hooks: Option<bool>,
    /// Choose which [channel](https://code.claude.com/docs/en/channels) plugins can push
    /// messages into sessions in your organization. When you set it, Claude Code uses your list
    /// in place of the default Anthropic allowlist; each entry names a plugin and the
    /// marketplace it comes from.
    ///
    /// - **Values**: array of objects, each with `marketplace` and `plugin` strings. An entry
    ///   can instead be a `"plugin@marketplace"` string such as
    ///   `"telegram@claude-plugins-official"`, which Claude Code treats as the equivalent
    ///   object. The string form requires Claude Code v2.1.267 or later; earlier versions
    ///   reject the whole `allowedChannelPlugins` value when it contains one
    /// - **Default**: unset, so Claude Code uses the default Anthropic allowlist
    /// - **Scope**: managed.
    ///
    /// Docs: [`allowedChannelPlugins`](https://code.claude.com/docs/en/settings-reference#allowedchannelplugins).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_channel_plugins: Option<Vec<ChannelPluginEntry>>,
    /// Limit which URLs [HTTP hooks](https://code.claude.com/docs/en/hooks#http-hook-fields)
    /// can target. When you define this key, Claude Code runs an HTTP hook only if its URL
    /// matches one of the patterns and blocks the rest without running them; an empty array
    /// blocks every HTTP hook.
    ///
    /// - **Values**: array of URL patterns, with `*` as a wildcard
    /// - **Default**: unset, so any URL is allowed
    /// - **Scope**: any settings file. Arrays merge across settings files.
    ///
    /// Docs: [`allowedHttpHookUrls`](https://code.claude.com/docs/en/settings-reference#allowedhttphookurls).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_http_hook_urls: Option<Vec<String>>,
    /// Restrict hook execution to hooks your organization deploys.
    ///
    /// - **Values**: Boolean
    ///   - `true`: only managed hooks run, plus Agent SDK hooks and hooks from plugins your
    ///     managed settings force-enable. See [What runs under
    ///     `allowManagedHooksOnly`](https://code.claude.com/docs/en/settings-reference#what-runs-under-allowmanagedhooksonly)
    ///   - `false`: hooks from every settings scope and plugin run
    /// - **Default**: unset, so hooks from every settings scope and plugin run
    /// - **Scope**: managed.
    ///
    /// Docs: [`allowManagedHooksOnly`](https://code.claude.com/docs/en/settings-reference#allowmanagedhooksonly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_hooks_only: Option<bool>,
    /// Make managed settings the only settings source of permission rules. Claude Code then
    /// ignores `allow`, `ask`, and `deny` rules in user, project, local, and `--settings`
    /// files, ignores `--allowedTools`, hides the always-allow choices in permission prompts,
    /// and stops saving new rules. When [parent settings from an embedding
    /// host](https://code.claude.com/docs/en/managed-settings#let-an-embedding-host-add-policy)
    /// apply, Claude Code treats them as part of the managed tier.
    ///
    /// - **Values**: Boolean
    ///   - `true`: managed settings become the only settings source of permission rules
    ///   - `false`: Claude Code applies permission rules from user, project, local, and
    ///     `--settings` files in addition to the managed ones
    /// - **Default**: unset, so Claude Code applies permission rules from user, project, and
    ///   local settings and from `--settings`, in addition to the managed ones
    /// - **Scope**: managed.
    ///
    /// Docs: [`allowManagedPermissionRulesOnly`](https://code.claude.com/docs/en/settings-reference#allowmanagedpermissionrulesonly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_permission_rules_only: Option<bool>,
    /// Run your own command to render a [status
    /// line](https://code.claude.com/docs/en/statusline) below the prompt with context such as
    /// the model, cost, or git branch. Optional fields adjust spacing, add periodic re-runs,
    /// and hide the built-in vim mode indicator when your script renders `vim.mode` itself.
    ///
    /// - **Values**: object with `type` set to `"command"` and a `command` string, plus
    ///   optional `padding` as a number of characters, `refreshInterval` as a number of
    ///   seconds, minimum `1`, and `hideVimModeIndicator` as a Boolean
    /// - **Default**: unset, so no status line
    /// - **Scope**: any settings file. When allowmanagedhooksonly is on, or disableallhooks is
    ///   set outside managed settings, only the managed settings value runs.
    ///
    /// Docs: [`statusLine`](https://code.claude.com/docs/en/settings-reference#statusline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_line: Option<StatusLine>,
    /// Run your own command to supply `@` file path autocomplete instead of the built-in file
    /// suggestion. The built-in suggestion uses fast filesystem traversal; a large monorepo may
    /// do better with project-specific indexing such as a pre-built file index.
    ///
    /// - **Values**: object with `type`, always `"command"`, and `command`, the shell command
    ///   to run
    /// - **Default**: unset, so Claude Code uses the built-in file suggestion
    /// - **Scope**: any settings file. Under the [status line and file suggestion
    ///   gates](https://code.claude.com/docs/en/settings-reference#status-line-and-file-suggestion-gates),
    ///   Claude Code turns the command off or runs only a managed value, and skips yours
    ///   without warning.
    ///
    /// Docs: [`fileSuggestion`](https://code.claude.com/docs/en/settings-reference#filesuggestion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_suggestion: Option<CommandSetting>,
    /// Turn individual [plugins](https://code.claude.com/docs/en/plugins/overview) on or off,
    /// keyed by `plugin-name@marketplace-name`. A plugin with no entry at any scope falls back
    /// to its
    /// [`defaultEnabled`](https://code.claude.com/docs/en/plugins/manifest-reference#fields)
    /// value. When you enable or disable a plugin with `/plugin` or `claude plugin enable`,
    /// Claude Code writes this key for you.
    ///
    /// - **Values**: object mapping `plugin-name@marketplace-name` to a Boolean
    /// - **Default**: unset, so each plugin follows its `defaultEnabled` value
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`enabledPlugins`](https://code.claude.com/docs/en/settings-reference#enabledplugins).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_plugins: Option<BTreeMap<String, PluginEnabled>>,
    /// Register additional plugin marketplaces by name, so that people who open the repository,
    /// or everyone your managed settings reach, get the marketplace without adding it
    /// themselves. Claude Code registers each marketplace it doesn't already know. Whether a
    /// plugin that
    /// [`enabledPlugins`](https://code.claude.com/docs/en/settings-reference#enabledplugins)
    /// names from it installs depends on the plugin's source and which file enables it;
    /// [Enabled in project settings but not
    /// installed](https://code.claude.com/docs/en/plugins/loading#enabled-in-project-settings-but-not-installed)
    /// has the rules.
    ///
    /// - **Values**: object mapping a marketplace name to an object with a `source` object and
    ///   an optional `autoUpdate` Boolean
    /// - **Default**: unset
    /// - **Scope**: any settings file. Claude Code honors entries in a repository's
    ///   `.claude/settings.json` or `.claude/settings.local.json` only after you accept the
    ///   workspace trust dialog for that folder; in a folder you haven't trusted, including a
    ///   `-p` run there, it ignores them without a message.
    ///
    /// Docs: [`extraKnownMarketplaces`](https://code.claude.com/docs/en/settings-reference#extraknownmarketplaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_known_marketplaces: Option<BTreeMap<String, Marketplace>>,
    /// Restrict which plugin marketplace sources people in your organization can add and
    /// install plugins from. Claude Code enforces the allowlist on marketplace add and on
    /// plugin install, update, refresh, and auto-update, before any network or filesystem
    /// operation, so a marketplace someone added before you set the policy can't be used to
    /// fetch plugins once its source no longer matches. Blocked users see an error naming the
    /// managed policy.
    ///
    /// - **Values**: array of marketplace source objects; see [Allowed source
    ///   types](https://code.claude.com/docs/en/settings-reference#allowed-source-types)
    /// - **Default**: unset, so users can add any marketplace. An empty array is a complete
    ///   lockdown that blocks every marketplace source, including the official Anthropic
    ///   marketplace
    /// - **Scope**: managed.
    ///
    /// Docs: [`strictKnownMarketplaces`](https://code.claude.com/docs/en/settings-reference#strictknownmarketplaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_known_marketplaces: Option<Vec<Source>>,
    /// List of marketplace names the user has chosen not to install when prompted.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: not in the settings reference; listed in the
    /// [JSON schema](https://json.schemastore.org/claude-code-settings.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_marketplaces: Option<Vec<String>>,
    /// List of plugin IDs (plugin@marketplace format) the user has chosen not to install when
    /// prompted.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: not in the settings reference; listed in the
    /// [JSON schema](https://json.schemastore.org/claude-code-settings.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_plugins: Option<Vec<String>>,
    /// Restrict which kind of account people can log in with: `"claudeai"` for claude.ai
    /// sign-in or `"console"` for Claude Console sign-in. Set `"gateway"` to send people to a
    /// [cloud gateway](https://code.claude.com/docs/en/claude-apps-gateway) instead of a
    /// first-party login. Administrators set it in managed settings and pair it with
    /// [`forceLoginOrgUUID`](https://code.claude.com/docs/en/settings-reference#forceloginorguuid)
    /// to keep developers' claude.ai logins inside one organization.
    ///
    /// - **Values**: string, one of:
    ///   - `"claudeai"`: claude.ai sign-in
    ///   - `"console"`: Claude Console sign-in
    ///   - `"gateway"`: Claude Code sends people to a cloud gateway instead of a first-party
    ///     login
    /// - **Default**: unset, so people pick a login method
    /// - **Scope**: any settings file. Claude Code honors `"gateway"` from the same sources as
    ///   forcelogingatewayurl and treats it as unset everywhere else.
    ///
    /// Docs: [`forceLoginMethod`](https://code.claude.com/docs/en/settings-reference#forceloginmethod).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_login_method: Option<LoginMethod>,
    /// From a managed source, require claude.ai account logins to belong to one Anthropic
    /// organization, given as a single UUID, or to any of several organizations, given as an
    /// array. From any settings file, Claude Code also uses a single UUID to pre-select that
    /// organization during a claude.ai or Claude Console login, and pre-selects nothing for an
    /// array. If you set the key in any settings file, Claude Code also stops offering the
    /// [keyless Console
    /// sign-in](https://code.claude.com/docs/en/authentication#sign-in-without-an-api-key) in
    /// the sessions that file applies to and creates an API key instead.
    ///
    /// - **Values**: string, one UUID, or array of strings, several UUIDs
    /// - **Default**: unset, so any organization can log in
    /// - **Scope**: any settings file. In a settings file that isn't a managed source, Claude
    ///   Code doesn't check which organization a login belongs to, and a single UUID
    ///   pre-selects the organization during login.
    ///
    /// Docs: [`forceLoginOrgUUID`](https://code.claude.com/docs/en/settings-reference#forceloginorguuid).
    #[serde(skip_serializing_if = "Option::is_none", rename = "forceLoginOrgUUID")]
    pub force_login_org_uuid: Option<OneOrMany>,
    /// Run your own command to generate the headers Claude Code sends with OpenTelemetry
    /// exports, for backends whose tokens rotate. Claude Code runs it at startup and
    /// periodically after that, and expects a JSON object of string header values on stdout.
    ///
    /// - **Values**: string, an executable path or a shell command line
    /// - **Default**: unset, so Claude Code adds no helper-generated headers
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`otelHeadersHelper`](https://code.claude.com/docs/en/settings-reference#otelheadershelper).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_headers_helper: Option<String>,
    /// Select an [output style](https://code.claude.com/docs/en/output-styles) by name. An
    /// output style is a saved set of instructions that changes Claude's role, tone, and output
    /// format, such as the built-in Explanatory and Learning styles or one you wrote yourself.
    /// If you change this key during a session, Claude uses the new style starting with your
    /// next message.
    ///
    /// - **Values**: string, the name of a
    ///   [built-in](https://code.claude.com/docs/en/output-styles#built-in-output-styles) or
    ///   [custom](https://code.claude.com/docs/en/output-styles#create-a-custom-output-style)
    ///   output style
    /// - **Default**: unset, so Claude Code uses the default style
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`outputStyle`](https://code.claude.com/docs/en/settings-reference#outputstyle).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_style: Option<String>,
    /// Skip the [WebFetch domain safety
    /// check](https://code.claude.com/docs/en/data-usage#webfetch-domain-safety-check), which
    /// sends each requested hostname to `api.anthropic.com` before fetching. Set `true` in
    /// environments that block traffic to Anthropic, such as Amazon Bedrock, Google Cloud's
    /// Agent Platform, or Microsoft Foundry deployments with restrictive egress.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code skips the WebFetch domain safety check
    ///   - `false`: the check runs before the first fetch to each hostname in a session, and
    ///     again for a hostname whose earlier check was blocked or failed
    /// - **Default**: unset, so the check runs before the first fetch to each hostname in a
    ///   session
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`skipWebFetchPreflight`](https://code.claude.com/docs/en/settings-reference#skipwebfetchpreflight).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_web_fetch_preflight: Option<bool>,
    /// Isolate the Bash commands Claude runs from your filesystem and network with
    /// [sandboxing](https://code.claude.com/docs/en/sandboxing). Turn the sandbox on with
    /// `enabled`, then narrow or widen what sandboxed commands can touch with the `filesystem`,
    /// `network`, and `credentials` sub-objects. The sandbox runs on macOS, Linux, and WSL2.
    ///
    /// - **Values**: object with `enabled`, `failIfUnavailable`, `autoAllowBashIfSandboxed`,
    ///   `excludedCommands`, `allowUnsandboxedCommands`, `enableWeakerNestedSandbox`,
    ///   `enableWeakerNetworkIsolation`, `allowAppleEvents`, `allowNestedUserNamespaces`,
    ///   `bwrapPath`, `socatPath`, `ignoreViolations`, and `ripgrep`, plus the `filesystem`,
    ///   `network`, and `credentials` objects
    /// - **Default**: unset, so Claude Code runs commands without a sandbox
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`sandbox`](https://code.claude.com/docs/en/settings-reference#sandbox).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<Sandbox>,
    /// While a turn is in progress, the spinner shows a rotating verb such as "Accomplishing",
    /// "Architecting", or "Baking". Use this key to add your own verbs to that rotation or
    /// replace the built-in list with yours.
    ///
    /// - **Values**: object with a `verbs` array of strings and `mode`, one of:
    ///   - `"append"`: Claude Code adds your verbs to the built-in set
    ///   - `"replace"`: Claude Code shows only your verbs
    /// - **Default**: unset, so Claude Code uses the built-in verbs
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`spinnerVerbs`](https://code.claude.com/docs/en/settings-reference#spinnerverbs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinner_verbs: Option<SpinnerVerbs>,
    /// While Claude works, the spinner line rotates through short tips about Claude Code
    /// features, such as "Use Plan Mode to prepare for a complex request before making changes.
    /// Press Shift+Tab twice to enable." Set this key to `false` to hide them. Appears in
    /// `/config` as **Show tips**.
    ///
    /// - **Values**: Boolean
    ///   - `true`: you see tips in the spinner while Claude is working
    ///   - `false`: Claude Code hides spinner tips
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`spinnerTipsEnabled`](https://code.claude.com/docs/en/settings-reference#spinnertipsenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinner_tips_enabled: Option<bool>,
    /// Add your own tips to the [spinner
    /// tips](https://code.claude.com/docs/en/settings-reference#spinnertipsenabled) that Claude
    /// Code shows while Claude works, or replace the built-in tips with yours. Claude Code puts
    /// your tips in the same rotation as the built-in ones. If you set
    /// [`spinnerTipsEnabled`](https://code.claude.com/docs/en/settings-reference#spinnertipsenabled)
    /// to `false`, Claude Code hides all tips, yours included.
    ///
    /// - **Values**: object with `tips`, `tipsFile`, `label`, and `excludeDefault` fields, each
    ///   optional
    /// - **Default**: unset, so Claude Code shows only the built-in tips
    /// - **Scope**: any settings file. Claude Code honors tip objects, `tipsFile`, `label`, and
    ///   `excludeDefault` from user settings, the `--settings` flag, and managed settings; from
    ///   project and local settings it reads plain string tips only.
    ///
    /// Docs: [`spinnerTipsOverride`](https://code.claude.com/docs/en/settings-reference#spinnertipsoverride).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinner_tips_override: Option<SpinnerTips>,
    /// Some terminals can show a progress indicator on the tab or in the taskbar for the
    /// program running in them. While Claude is working, Claude Code reports an in-progress
    /// state to the terminal, so you can see from another tab or window whether the session is
    /// still busy. The indicator stays visible after the turn ends while [background
    /// subagents](https://code.claude.com/docs/en/sub-agents#run-subagents-in-foreground-or-background)
    /// or [dynamic workflows](https://code.claude.com/docs/en/workflows) are still running, and
    /// clears once the session is idle.
    ///
    /// - **Values**: Boolean
    ///   - `true`: you see the terminal progress bar in terminals that support it
    ///   - `false`: Claude Code hides the terminal progress bar
    /// - **Default**: `true`
    /// - **Scope**: any settings file. A value in `~/.claude.json` from an older version
    ///   applies when no settings file sets it.
    ///
    /// Docs: [`terminalProgressBarEnabled`](https://code.claude.com/docs/en/settings-reference#terminalprogressbarenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_progress_bar_enabled: Option<bool>,
    /// Show or hide the turn duration message after each response, such as "Cooked for 1m 6s ·
    /// done 6:05 PM". The clock after "done" shows when the turn finished;
    /// [`timeFormat`](https://code.claude.com/docs/en/settings-reference#timeformat) and
    /// [`timeZone`](https://code.claude.com/docs/en/settings-reference#timezone) control its
    /// format and zone. Appears in `/config` as **Show turn duration**.
    ///
    /// - **Values**: Boolean
    ///   - `true`: you see the turn duration message after each response
    ///   - `false`: Claude Code hides the turn duration message
    /// - **Default**: `true`
    /// - **Scope**: any settings file. A value in `~/.claude.json` from an older version
    ///   applies when no settings file sets it.
    ///
    /// Docs: [`showTurnDuration`](https://code.claude.com/docs/en/settings-reference#showturnduration).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_turn_duration: Option<bool>,
    /// Hide or collapse a
    /// [skill](https://code.claude.com/docs/en/skills#override-skill-visibility-from-settings)
    /// without editing its `SKILL.md`. Claude Code applies the value under each skill's name to
    /// the skill list Claude sees and to your `/` autocomplete.
    ///
    /// - **Values**: object mapping skill name to one of:
    ///   - `"on"`: Claude sees the skill and you can type `/name`
    ///   - `"name-only"`: Claude sees the skill by name without its description
    ///   - `"user-invocable-only"`: Claude doesn't see the skill, but you can still type
    ///     `/name`
    ///   - `"off"`: Claude doesn't see the skill and `/name` is hidden from autocomplete
    /// - **Default**: unset, so every skill is `"on"`
    /// - **Scope**: any settings file. The `/skills` menu writes to
    ///   `.claude/settings.local.json`.
    ///
    /// Docs: [`skillOverrides`](https://code.claude.com/docs/en/settings-reference#skilloverrides).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_overrides: Option<BTreeMap<String, SkillOverride>>,
    /// Reduce or turn off interface animations such as the spinner, shimmer, and flash effects.
    /// Appears in `/config` as **Reduce motion**.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code reduces or turns off interface animations such as the spinner,
    ///     shimmer, and flash effects
    ///   - `false`: the same as unset; Claude Code shows its animations
    /// - **Default**: `false`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`prefersReducedMotion`](https://code.claude.com/docs/en/settings-reference#prefersreducedmotion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefers_reduced_motion: Option<bool>,
    /// Point the PR links Claude Code renders, in the footer badge and in tool-result
    /// summaries, at an internal code-review tool instead of `github.com`. Claude Code
    /// substitutes `{host}`, `{owner}`, `{repo}`, `{number}`, and `{url}` from the PR URL.
    /// [GitLab merge
    /// request](https://code.claude.com/docs/en/interactive-mode#gitlab-merge-requests) links
    /// on both surfaces keep their GitLab URL.
    ///
    /// - **Values**: string, a URL template using any of the five placeholders
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`prUrlTemplate`](https://code.claude.com/docs/en/settings-reference#prurltemplate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url_template: Option<String>,
    /// Turn [extended thinking](https://code.claude.com/docs/en/model-config#extended-thinking)
    /// off for every session by setting this to `false`. Thinking is on by default, so `true`
    /// changes nothing. Most people set this through `/config` rather than by editing the file.
    ///
    /// - **Values**: Boolean
    ///   - `true`: no effect; thinking is already on
    ///   - `false`: Claude Code turns extended thinking off for every session
    /// - **Default**: unset, so thinking is on for models that support it
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`alwaysThinkingEnabled`](https://code.claude.com/docs/en/settings-reference#alwaysthinkingenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_thinking_enabled: Option<bool>,
    /// Show your organization's announcements to users at startup. When you list more than one,
    /// Claude Code picks one at random for each session; on a person's very first launch it
    /// shows the first entry.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset, so no announcement shows
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`companyAnnouncements`](https://code.claude.com/docs/en/settings-reference#companyannouncements).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_announcements: Option<Vec<String>>,
    /// Choose where Claude Code shows [agent team](https://code.claude.com/docs/en/agent-teams)
    /// teammates: inside your main terminal pane, or in split panes when your terminal supports
    /// them. See [Choose a display
    /// mode](https://code.claude.com/docs/en/agent-teams#choose-a-display-mode).
    ///
    /// - **Values**: string, one of:
    ///   - `"in-process"`: teammates run inside your main terminal pane
    ///   - `"auto"`: split panes when you're running inside tmux, or inside iTerm2 with `it2`
    ///     on your `PATH` or tmux installed; in-process otherwise
    ///   - `"tmux"`: split panes using tmux or iTerm2, detected from your terminal
    ///   - `"iterm2"`: iTerm2 native split panes through the `it2` CLI
    /// - **Default**: `"in-process"`
    /// - **Scope**: any settings file. Claude Code also reads a value left in `~/.claude.json`
    ///   by older versions.
    ///
    /// Docs: [`teammateMode`](https://code.claude.com/docs/en/settings-reference#teammatemode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teammate_mode: Option<TeammateMode>,
    /// Configure how Claude Code creates and manages [git
    /// worktrees](https://code.claude.com/docs/en/worktrees) for `--worktree`, the
    /// `EnterWorktree` tool, and isolated subagents and background sessions.
    ///
    /// - **Values**: object with `baseRef`, `symlinkDirectories`, `sparsePaths`, and
    ///   `bgIsolation`
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`worktree`](https://code.claude.com/docs/en/settings-reference#worktree).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<Worktree>,
    /// Choose whether Claude Code applies managed settings supplied by an embedding host
    /// process, such as the Agent SDK or an IDE extension, when an admin-deployed managed tier
    /// is also present. With `"first-wins"`, Claude Code drops the host-supplied settings; with
    /// `"merge"`, it applies them under the admin tier through a restrictive-only filter. Set
    /// `"merge"` when a host needs to pass its own restrictions to the sessions it launches,
    /// for example Claude Desktop delivering a gateway's egress allowlist.
    ///
    /// - **Values**: string, one of:
    ///   - `"first-wins"`: Claude Code drops the host-supplied settings when an admin-deployed
    ///     managed tier is present
    ///   - `"merge"`: Claude Code applies the host-supplied settings under the admin tier
    ///     through a restrictive-only filter
    /// - **Default**: `"first-wins"`
    /// - **Scope**: managed. Claude Code reads it from the highest-priority admin-controlled
    ///   managed source.
    ///
    /// Docs: [`parentSettingsBehavior`](https://code.claude.com/docs/en/settings-reference#parentsettingsbehavior).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_settings_behavior: Option<SourcesBehavior>,
    /// Add your organization's own text to the plugin trust warning Claude Code shows before
    /// installation, for example to confirm that plugins from your internal marketplace are
    /// vetted.
    ///
    /// - **Values**: string
    /// - **Default**: unset, so Claude Code shows the standard warning alone
    /// - **Scope**: managed.
    ///
    /// Docs: [`pluginTrustMessage`](https://code.claude.com/docs/en/settings-reference#plugintrustmessage).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_trust_message: Option<String>,
    /// Store the non-sensitive answers you give a plugin's
    /// [`userConfig`](https://code.claude.com/docs/en/plugins/manifest-reference#user-configuration)
    /// configuration dialog, keyed by plugin ID. Claude Code writes this key to your user
    /// settings when you fill in the dialog, so you don't need to edit it by hand. Claude Code
    /// stores sensitive options in the macOS Keychain instead, falling back to
    /// `~/.claude/.credentials.json` when the Keychain rejects the write; on platforms without
    /// a supported keychain, it stores them in `~/.claude/.credentials.json`.
    ///
    /// - **Values**: object mapping a plugin ID to an object with an `options` field, mapping
    ///   each option name to a string, number, Boolean, or array of strings, and an optional
    ///   `mcpServers` field holding per-server user configuration values in the same shape
    /// - **Default**: unset
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`pluginConfigs`](https://code.claude.com/docs/en/settings-reference#pluginconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_configs: Option<BTreeMap<String, PluginConfig>>,
    /// Make the managed allowlist the only one that applies. Claude Code then reads
    /// [`allowedMcpServers`](https://code.claude.com/docs/en/settings-reference#allowedmcpservers)
    /// from managed settings alone and ignores allowlists in user, project, and local settings;
    /// [`deniedMcpServers`](https://code.claude.com/docs/en/settings-reference#deniedmcpservers)
    /// still merges from every settings scope, so users can still block servers for themselves.
    /// Administrators set it so a user's own settings can't broaden what the managed allowlist
    /// permits.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code reads `allowedMcpServers` from managed settings alone and
    ///     ignores allowlists in user, project, and local settings
    ///   - `false`: allowlists from every settings scope merge
    /// - **Default**: `false`, so allowlists from every settings scope merge
    /// - **Scope**: managed.
    ///
    /// Docs: [`allowManagedMcpServersOnly`](https://code.claude.com/docs/en/settings-reference#allowmanagedmcpserversonly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_mcp_servers_only: Option<bool>,
    /// Block plugin marketplace sources for your organization. Claude Code checks the blocklist
    /// on marketplace add and on plugin install, update, refresh, and auto-update, so a
    /// marketplace someone added before you set the policy can't be used to fetch plugins
    /// either. Blocked sources are checked before download, so they never touch the filesystem.
    ///
    /// - **Values**: array of marketplace source objects, in the same forms as
    ///   [`strictKnownMarketplaces`](https://code.claude.com/docs/en/settings-reference#allowed-source-types)
    /// - **Default**: unset, so no marketplace is blocked
    /// - **Scope**: managed.
    ///
    /// Docs: [`blockedMarketplaces`](https://code.claude.com/docs/en/settings-reference#blockedmarketplaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_marketplaces: Option<Vec<Source>>,
    /// Run the main thread as a named
    /// [subagent](https://code.claude.com/docs/en/sub-agents#invoke-subagents-explicitly), so
    /// Claude Code applies that subagent's system prompt, tool restrictions, and model to your
    /// session. The same key sets the default agent for sessions you dispatch from `claude
    /// agents`.
    ///
    /// - **Values**: string, the name of a built-in or custom agent
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`agent`](https://code.claude.com/docs/en/settings-reference#agent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Store [auto memory](https://code.claude.com/docs/en/memory#storage-location) in a
    /// directory of your choice instead of the per-project default.
    ///
    /// - **Values**: string, an absolute or `~/`-prefixed directory path
    /// - **Default**: unset, so Claude Code uses `~/.claude/projects/<project>/memory/`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`autoMemoryDirectory`](https://code.claude.com/docs/en/settings-reference#automemorydirectory).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_memory_directory: Option<String>,
    /// Add your own rules to what the [auto
    /// mode](https://code.claude.com/docs/en/permission-modes#eliminate-prompts-with-auto-mode)
    /// classifier blocks and allows. Use it to tell the classifier which repos, buckets, and
    /// domains your organization trusts, so it stops blocking routine internal operations. The
    /// classifier ships with [built-in allow and deny
    /// rules](https://code.claude.com/docs/en/auto-mode-config#inspect-the-defaults-and-your-effective-config).
    ///
    /// - **Values**: object with `environment`, `allow`, `soft_deny`, and `hard_deny` arrays of
    ///   prose rules, plus the
    ///   [`classifyAllShell`](https://code.claude.com/docs/en/settings-reference#automode-classifyallshell)
    ///   Boolean
    /// - **Default**: unset, so the classifier uses only its [built-in
    ///   rules](https://code.claude.com/docs/en/auto-mode-config#inspect-the-defaults-and-your-effective-config)
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`autoMode`](https://code.claude.com/docs/en/settings-reference#automode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_mode: Option<AutoMode>,
    /// Allow [channels](https://code.claude.com/docs/en/channels) for your organization. On
    /// claude.ai Team and Enterprise plans, Claude Code blocks channels until you set this to
    /// `true`. For [Anthropic
    /// Console](https://code.claude.com/docs/en/authentication#claude-console-authentication)
    /// accounts that authenticate with an API key, channels are allowed by default.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code allows channels for your organization
    ///   - `false`: the same as unset; whether channels are blocked depends on your plan, as
    ///     the Default says
    /// - **Default**: unset; channels are blocked on Team and Enterprise plans and on Console
    ///   accounts with managed settings, and allowed on Pro, Max, and Ultra plans and on
    ///   Console accounts without managed settings
    /// - **Scope**: managed.
    ///
    /// Docs: [`channelsEnabled`](https://code.claude.com/docs/en/settings-reference#channelsenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels_enabled: Option<bool>,
    /// Choose whether Bash or PowerShell runs the shell commands you type with the [`!`
    /// prefix](https://code.claude.com/docs/en/interactive-mode#shell-mode-with-prefix) in the
    /// input box, the ones Claude Code runs directly and adds to the session. `"powershell"`
    /// works only while the [PowerShell
    /// tool](https://code.claude.com/docs/en/tools-reference#powershell-tool) is on. The tool
    /// is on by default on Windows without Git Bash, and on Windows with Git Bash for claude.ai
    /// and Console accounts.
    ///
    /// - **Values**: string, one of:
    ///   - `"bash"`: Claude Code runs your `!` commands in Bash
    ///   - `"powershell"`: Claude Code runs your `!` commands in PowerShell
    /// - **Default**: `"bash"`, or `"powershell"` on Windows when Bash isn't available
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`defaultShell`](https://code.claude.com/docs/en/settings-reference#defaultshell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_shell: Option<Shell>,
    /// Stop Claude Code from registering the `claude-cli://` protocol handler with the
    /// operating system. [Deep links](https://code.claude.com/docs/en/deep-links) let external
    /// tools open a Claude Code session with a pre-filled prompt. Set this in environments
    /// where protocol handler registration is restricted or managed separately.
    ///
    /// - **Values**: the string `"disable"`
    /// - **Default**: unset, so Claude Code registers the handler
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`disableDeepLinkRegistration`](https://code.claude.com/docs/en/settings-reference#disabledeeplinkregistration).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_deep_link_registration: Option<DisableFlag>,
    /// Turn off inline shell execution for `` !`...` `` and ` ```! ` blocks in
    /// [skills](https://code.claude.com/docs/en/skills) and custom commands from user, project,
    /// plugin, or additional-directory sources. Claude Code replaces each command with `[shell
    /// command execution disabled by policy]` instead of running it.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code replaces each inline shell command with `[shell command
    ///     execution disabled by policy]` instead of running it
    ///   - `false`: inline shell runs
    /// - **Default**: unset, so inline shell runs
    /// - **Scope**: any settings file. A `true` in managed settings can't be overridden by
    ///   `false` elsewhere.
    ///
    /// Docs: [`disableSkillShellExecution`](https://code.claude.com/docs/en/settings-reference#disableskillshellexecution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_skill_shell_execution: Option<bool>,
    /// Block CLI startup until Claude Code has freshly fetched [server-managed
    /// settings](https://code.claude.com/docs/en/server-managed-settings). If the fetch fails,
    /// Claude Code exits instead of continuing with cached or no settings. Set it when your
    /// environment can't accept even a brief window in which a session runs without its managed
    /// policy.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code blocks startup until it has freshly fetched server-managed
    ///     settings, and exits if the fetch fails
    ///   - `false`: unless Anthropic requires the settings, Claude Code doesn't block startup
    ///     on the fetch, though at a sign-in startup it waits up to five seconds for the fetch
    /// - **Default**: `false`
    /// - **Scope**: managed. Claude Code honors a `true` from any admin-controlled managed
    ///   source, even one that isn't the highest-priority source.
    ///
    /// Docs: [`forceRemoteSettingsRefresh`](https://code.claude.com/docs/en/settings-reference#forceremotesettingsrefresh).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_remote_settings_refresh: Option<bool>,
    /// Keep background auto-updates and `claude update` from installing any version below this
    /// one, so moving to the `"stable"` channel doesn't downgrade you from a newer `"latest"`
    /// build. Claude Code writes this key for you when you choose to stay on your current
    /// version while switching channels in `/config`, and clears it when you switch back to
    /// `"latest"`.
    ///
    /// - **Values**: string, a version number such as `"2.1.100"`; a value that isn't a valid
    ///   version is ignored
    /// - **Default**: unset, so updates can install any version the channel offers
    /// - **Scope**: any settings file. Set it in managed settings to pin an organization-wide
    ///   minimum that user and project settings can't lower.
    ///
    /// Docs: [`minimumVersion`](https://code.claude.com/docs/en/settings-reference#minimumversion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_version: Option<String>,
    /// When Claude finishes a plan in [plan
    /// mode](https://code.claude.com/docs/en/permission-modes#review-and-approve-a-plan), it
    /// shows an approval menu. Planning can use a lot of context, so this key adds a first
    /// option to that menu, **Yes, clear context and …**, that approves the plan, clears the
    /// conversation context, and starts implementing from the plan alone. The rest of the label
    /// names the permission mode the session continues in, and shows how much of your context
    /// the planning used.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the plan approval menu gets a first option, **Yes, clear context and …**,
    ///     that approves the plan and clears the conversation context
    ///   - `false`: the plan approval menu shows no clear-context option
    /// - **Default**: `false`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`showClearContextOnPlanAccept`](https://code.claude.com/docs/en/settings-reference#showclearcontextonplanaccept).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_clear_context_on_plan_accept: Option<bool>,
    /// See summaries of Claude's [extended
    /// thinking](https://code.claude.com/docs/en/model-config#extended-thinking) in interactive
    /// sessions. Set it if you want the full summaries when you expand thinking with `Ctrl+O`.
    /// When unset or `false`, the Anthropic API redacts thinking blocks and Claude Code shows a
    /// collapsed stub.
    ///
    /// - **Values**: Boolean
    ///   - `true`: you see full thinking summaries when you expand thinking with `Ctrl+O`
    ///   - `false`: the Anthropic API redacts thinking blocks and Claude Code shows a collapsed
    ///     stub
    /// - **Default**: `false`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`showThinkingSummaries`](https://code.claude.com/docs/en/settings-reference#showthinkingsummaries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_thinking_summaries: Option<bool>,
    /// Skip the confirmation dialog Claude Code shows before a session enters
    /// `bypassPermissions` mode, whether from `--dangerously-skip-permissions` or from
    /// `defaultMode: "bypassPermissions"`. Claude Code writes `true` here in your user settings
    /// when you accept that dialog once.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code skips the confirmation dialog before a session enters
    ///     `bypassPermissions` mode
    ///   - `false`: the same as unset; the dialog appears unless another of these files sets
    ///     `true`
    /// - **Default**: unset, so the dialog appears
    /// - **Scope**: user, local, or managed, and files passed with `--settings`. [Background
    ///   sessions](https://code.claude.com/docs/en/permission-modes#bypass-permissions-ignored-in-a-background-session)
    ///   count only user and managed settings.
    ///
    /// Docs: [`skipDangerousModePermissionPrompt`](https://code.claude.com/docs/en/settings-reference#skipdangerousmodepermissionprompt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_dangerous_mode_permission_prompt: Option<bool>,
    /// Block skills, agents, hooks, and MCP servers from user and project sources. Combine it
    /// with
    /// [`strictKnownMarketplaces`](https://code.claude.com/docs/en/settings-reference#strictknownmarketplaces)
    /// to choose which marketplaces users can install plugins from. Add
    /// [`disableSideloadFlags`](https://code.claude.com/docs/en/settings-reference#disablesideloadflags)
    /// to reject the CLI flags that sideload plugins, agents, and MCP servers for a single run.
    ///
    /// - **Values**: `true` to lock all of them, or an array naming the ones to lock, from
    ///   `"skills"`, `"agents"`, `"hooks"`, and `"mcp"`
    /// - **Default**: unset, so nothing is locked
    /// - **Scope**: managed.
    ///
    /// Docs: [`strictPluginOnlyCustomization`](https://code.claude.com/docs/en/settings-reference#strictpluginonlycustomization).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_plugin_only_customization: Option<StrictCustomization>,
    /// Choose the terminal UI renderer. Use `"fullscreen"` for the flicker-free [alt-screen
    /// renderer](https://code.claude.com/docs/en/fullscreen) with virtualized scrollback, or
    /// `"default"` for the classic main-screen renderer. Running `/tui fullscreen` or `/tui
    /// default` writes this key for you.
    ///
    /// - **Values**: string, one of:
    ///   - `"default"`: the classic main-screen renderer
    ///   - `"fullscreen"`: the flicker-free alt-screen renderer with virtualized scrollback
    /// - **Default**: unset, so Claude Code [picks the renderer for
    ///   you](https://code.claude.com/docs/en/fullscreen#fullscreen-by-default)
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`tui`](https://code.claude.com/docs/en/settings-reference#tui).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tui: Option<Tui>,
    /// Set the transcript view Claude Code starts in: `"default"`, `"verbose"`, or `"focus"`.
    /// When set, it overrides both the sticky `/focus` selection and the
    /// [`verbose`](https://code.claude.com/docs/en/settings-reference#verbose) setting.
    ///
    /// - **Values**: string, one of:
    ///   - `"default"`: the normal transcript with truncated tool output
    ///   - `"verbose"`: the transcript with full tool output
    ///   - `"focus"`: only your last prompt, a one-line summary of tool calls with edit
    ///     diffstats, and the final response. Focus view needs the [fullscreen
    ///     renderer](https://code.claude.com/docs/en/settings-reference#tui)
    /// - **Default**: unset, so the `verbose` setting and your last `/focus` choice apply
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`viewMode`](https://code.claude.com/docs/en/settings-reference#viewmode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_mode: Option<ViewMode>,
    /// Choose whether Claude Code uses the auto mode classifier to review shell commands in
    /// plan mode. With the default `true`, the classifier reviews each command during planning
    /// when auto mode is available and you see no prompt, except for [critical-path
    /// removals](https://code.claude.com/docs/en/permission-modes#critical-paths). Set `false`
    /// to get a permission prompt for every command outside the built-in read-only set.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the same as unset; when auto mode is available, the classifier reviews shell
    ///     commands during planning
    ///   - `false`: you get a permission prompt for every command outside the built-in
    ///     read-only set
    /// - **Default**: `true`
    /// - **Scope**: user, local, or managed. A repository can't turn it off for you.   * A
    ///   `false` in any of these files or in `--settings` turns it off, unless you leave that
    ///   user or local file out with an option such as --setting-sources.
    ///
    /// Docs: [`useAutoModeDuringPlan`](https://code.claude.com/docs/en/settings-reference#useautomodeduringplan).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_auto_mode_during_plan: Option<bool>,
    /// **Warning:** Deprecated since v2.1.92, when the
    /// [`voice`](https://code.claude.com/docs/en/settings-reference#voice) object replaced it.
    /// Claude Code still reads it so older settings files keep working, but new configurations
    /// should set `voice.enabled`. Turn voice dictation on with the single Boolean form that
    /// predates the `voice` object.
    ///
    /// - **Values**: Boolean
    ///   - `true`: voice dictation is on when you're logged in with a claude.ai account and
    ///     your organization's policy allows voice, unless `voice.enabled` is set
    ///   - `false`: voice dictation is off, unless `voice.enabled` is set
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`voiceEnabled`](https://code.claude.com/docs/en/settings-reference#voiceenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_enabled: Option<bool>,
    /// Have Claude Code on WSL read managed settings from the Windows policy chain, with HKLM
    /// and the Windows managed settings file taking priority over `/etc/claude-code` and HKCU
    /// below it. While the chain is on, Claude Code reads `/etc/claude-code` only when [no
    /// Windows admin document is
    /// present](https://code.claude.com/docs/en/managed-settings#present-admin-documents) in
    /// the HKLM registry value or the `C:\Program Files\ClaudeCode\` folder. Set it to extend
    /// the policy you already deploy on Windows to WSL sessions on the same machine, so they
    /// follow the same rules as host sessions.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code on WSL reads managed settings from the Windows policy chain, and
    ///     reads `/etc/claude-code` only when no Windows admin document is present
    ///   - `false`: WSL reads only `/etc/claude-code`
    /// - **Default**: `false`, so WSL reads only `/etc/claude-code`
    /// - **Scope**: managed. In an admin-controlled Windows source.
    ///
    /// Docs: [`wslInheritsWindowsSettings`](https://code.claude.com/docs/en/settings-reference#wslinheritswindowssettings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsl_inherits_windows_settings: Option<bool>,
    /// Status line configuration for subagent sessions.
    ///
    /// - **Values**: object
    /// - **Default**: unset
    ///
    /// Docs: not in the settings reference; listed in the
    /// [JSON schema](https://json.schemastore.org/claude-code-settings.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_status_line: Option<CommandSetting>,
    /// Pick which model answers when Claude calls the server-side [advisor
    /// tool](https://code.claude.com/docs/en/advisor). Unset it to turn the advisor off. The
    /// advisor must be at least as capable as your main model.
    ///
    /// - **Values**: string, one of the aliases `"fable"`, `"opus"`, or `"sonnet"`, which
    ///   resolve to Claude Code's current default version of that model family, or a full model
    ///   ID such as `"claude-opus-5-5"`
    /// - **Default**: unset, so the advisor is off
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`advisorModel`](https://code.claude.com/docs/en/settings-reference#advisormodel).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advisor_model: Option<String>,
    /// Have Claude Code [compact the conversation
    /// automatically](https://code.claude.com/docs/en/context-window#when-your-context-fills-up)
    /// when context approaches the limit. Appears in `/config` as **Auto-compact**, and
    /// toggling it there writes this key to your user settings.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code compacts the conversation automatically when context approaches
    ///     the limit
    ///   - `false`: Claude Code doesn't compact automatically
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`autoCompactEnabled`](https://code.claude.com/docs/en/settings-reference#autocompactenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_enabled: Option<bool>,
    /// Turn off [background agents and agent view](https://code.claude.com/docs/en/agent-view):
    /// `claude agents`, `--bg`, `/background`, and the on-demand supervisor. Set it in [managed
    /// settings](https://code.claude.com/docs/en/managed-settings) to enforce it for an
    /// organization.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code turns off `claude agents`, `--bg`, `/background`, and the
    ///     on-demand supervisor
    ///   - `false`: agent view is available
    /// - **Default**: unset, so agent view is available
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`disableAgentView`](https://code.claude.com/docs/en/settings-reference#disableagentview).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_agent_view: Option<bool>,
    /// Choose the key binding mode for the input prompt.
    ///
    /// - **Values**: string, one of:
    ///   - `"normal"`: standard key bindings in the prompt input
    ///   - `"vim"`: vim-style editing with NORMAL, INSERT, and VISUAL modes
    /// - **Default**: `"normal"`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`editorMode`](https://code.claude.com/docs/en/settings-reference#editormode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_mode: Option<EditorMode>,
    /// The `/model` picker has a **Default** option, and [`default` model
    /// setting](https://code.claude.com/docs/en/model-config#default-model-setting) describes
    /// the model it resolves to. An
    /// [`availableModels`](https://code.claude.com/docs/en/settings-reference#availablemodels)
    /// allowlist limits the models you can name, but with the default [prefix
    /// matching](https://code.claude.com/docs/en/settings-reference#availablemodelsmatch) it
    /// doesn't remap your account type's default, so **Default** can still resolve to a model
    /// outside the list. This key closes that gap.
    ///
    /// - **Values**: Boolean
    ///   - `true`: when **Default** would resolve to a model outside `availableModels`, Claude
    ///     Code resolves it to the first available model in the list
    ///   - `false`: this key doesn't change how **Default** resolves
    /// - **Default**: `false`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`enforceAvailableModels`](https://code.claude.com/docs/en/settings-reference#enforceavailablemodels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_available_models: Option<bool>,
    /// Name backup models for Claude Code to try, in order, when your primary model is
    /// overloaded or unavailable. Claude Code switches to the next available model in the chain
    /// for the rest of the turn and shows a notice. Without a chain, Claude Code retries the
    /// same model and then surfaces the server's error, and you retry or switch models
    /// yourself.
    ///
    /// - **Values**: array of model aliases or IDs; `"default"` expands to the default model
    /// - **Default**: unset, so a failed request isn't retried on another model
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`fallbackModel`](https://code.claude.com/docs/en/settings-reference#fallbackmodel).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<Vec<String>>,
    /// Have Claude Code snapshot files before each edit so
    /// [`/rewind`](https://code.claude.com/docs/en/checkpointing) can restore them. Appears in
    /// `/config` as **Rewind code (checkpoints)**, and toggling it there writes this key to
    /// your user settings.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code snapshots files before each edit so `/rewind` can restore them
    ///   - `false`: Claude Code doesn't snapshot files, so `/rewind` can't restore them
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`fileCheckpointingEnabled`](https://code.claude.com/docs/en/settings-reference#filecheckpointingenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_checkpointing_enabled: Option<bool>,
    /// Run your own command to refresh Google Cloud Application Default Credentials when Claude
    /// Code finds they've expired or can't be loaded, so [Google Cloud's Agent
    /// Platform](https://code.claude.com/docs/en/google-vertex-ai) requests keep working
    /// without you re-authenticating by hand. When several Claude Code processes that use the
    /// same command and credentials, such as separate terminals or IDE windows, find them
    /// expired at the same time, one process runs the command and the rest wait for that run
    /// instead of starting their own. A process that has waited 60 seconds with a request
    /// pending runs the command itself.
    ///
    /// - **Values**: string, a shell command line
    /// - **Default**: unset, so Claude Code's credential error tells you to run `gcloud auth
    ///   application-default login` yourself
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`gcpAuthRefresh`](https://code.claude.com/docs/en/settings-reference#gcpauthrefresh).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_auth_refresh: Option<String>,
    /// Provide remote MCP servers to every user from managed settings. Users keep the servers
    /// they add themselves and can't edit or remove the ones you provide. Requires Claude Code
    /// v2.1.259 or later.
    ///
    /// - **Values**: object keyed by server name. Each entry has the `.mcp.json` shape for an
    ///   `http` or `sse` server: a required `https://` `url`, and optionally `headers`,
    ///   `oauth`, and the other HTTP and SSE options. Claude Code drops entries that fail
    ///   validation, and [What an entry can
    ///   contain](https://code.claude.com/docs/en/managed-mcp#what-an-entry-can-contain) lists
    ///   the conditions
    /// - **Default**: unset, so managed settings provide no servers
    /// - **Scope**: managed. Claude Code drops the key with a warning in user, project, and
    ///   local settings, and doesn't read it in the Claude Desktop app's Code tab on a
    ///   third-party deployment or in the app's Cowork sessions, where Claude Desktop supplies
    ///   and locks those sessions' MCP servers itself.
    ///
    /// Docs: [`managedMcpServers`](https://code.claude.com/docs/en/settings-reference#managedmcpservers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_mcp_servers: Option<ManagedMcpServers>,
    /// Each turn, Claude sees a [listing of your
    /// skills](https://code.claude.com/docs/en/skills#skill-descriptions-are-cut-short) that
    /// shows each skill's `description` and `when_to_use` text. This key caps how many
    /// characters of that text Claude Code shows per skill; longer text is cut at the cap.
    ///
    /// - **Values**: number of characters, a positive integer
    /// - **Default**: `1536`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`skillListingMaxDescChars`](https://code.claude.com/docs/en/settings-reference#skilllistingmaxdescchars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_listing_max_desc_chars: Option<u64>,
    /// Choose how Claude Code notifies you when a task completes or a permission prompt is
    /// waiting. A [mod's
    /// notifications](https://code.claude.com/docs/en/plugins/mods/api#show-something-without-starting-a-turn)
    /// use the same setting. Appears in `/config` as **Local notifications**.
    ///
    /// - **Values**: string, one of:
    ///   - `"auto"`: Claude Code sends a desktop notification when it detects iTerm2, Ghostty,
    ///     or Kitty, rings the bell in Terminal.app only when its audible bell is off, and does
    ///     nothing elsewhere
    ///   - `"terminal_bell"`: Claude Code rings the bell character in any terminal
    ///   - `"iterm2"`: Claude Code sends an iTerm2 desktop notification
    ///   - `"iterm2_with_bell"`: Claude Code sends an iTerm2 desktop notification and rings the
    ///     bell
    ///   - `"kitty"`: Claude Code sends a Kitty desktop notification
    ///   - `"ghostty"`: Claude Code sends a Ghostty desktop notification
    ///   - `"notifications_disabled"`: Claude Code sends no notification
    /// - **Default**: `"auto"`
    /// - **Scope**: any settings file. Claude Code also reads a value left in `~/.claude.json`
    ///   by older versions.
    ///
    /// Docs: [`preferredNotifChannel`](https://code.claude.com/docs/en/settings-reference#preferrednotifchannel).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_notif_channel: Option<NotifChannel>,
    /// Name the marketplaces whose plugins can appear as contextual install suggestions, in
    /// spinner tips and pinned at the top of the `/plugin` **Discover** tab. The built-in
    /// first-party frontend-design tip is unaffected. Suggestions come from each plugin's
    /// `relevance` declaration in its marketplace entry.
    ///
    /// - **Values**: array of marketplace names
    /// - **Default**: unset, so no marketplace-declared suggestions surface
    /// - **Scope**: managed.
    ///
    /// Docs: [`pluginSuggestionMarketplaces`](https://code.claude.com/docs/en/settings-reference#pluginsuggestionmarketplaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_suggestion_marketplaces: Option<Vec<String>>,
    /// Set the newest Claude Code version your organization allows to start. When the running
    /// version is newer, Claude Code exits at startup and tells the user to install an approved
    /// version through your organization's approved method; `claude install <version>` may also
    /// work. Requires Claude Code v2.1.163 or later.
    ///
    /// - **Values**: string, a version number such as `"2.1.150"`; a value that isn't a valid
    ///   version is ignored
    /// - **Default**: unset, so no ceiling applies
    /// - **Scope**: managed. Claude Code gives no warning when it ignores the key elsewhere.
    ///
    /// Docs: [`requiredMaximumVersion`](https://code.claude.com/docs/en/settings-reference#requiredmaximumversion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_maximum_version: Option<String>,
    /// Set the oldest Claude Code version your organization allows to start. When the running
    /// version is older, Claude Code exits at startup and tells the user to update through your
    /// organization's approved method. The check runs at startup only, so a session that's
    /// already running continues.
    ///
    /// - **Values**: string, a version number such as `"2.1.150"`; a value that isn't a valid
    ///   version is ignored
    /// - **Default**: unset, so no floor applies
    /// - **Scope**: managed. Claude Code gives no warning when it ignores the key elsewhere.
    ///
    /// Docs: [`requiredMinimumVersion`](https://code.claude.com/docs/en/settings-reference#requiredminimumversion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_minimum_version: Option<String>,
    /// Choose whether Claude responds after you run a shell command with the [`!`
    /// prefix](https://code.claude.com/docs/en/interactive-mode#shell-mode-with-prefix) in the
    /// input box. By default, Claude Code adds the command's output to the conversation and
    /// Claude replies to it. Set this key to `false` to add the output to context without a
    /// reply, so you can run several commands and ask about them together.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code adds the command's output to the conversation and Claude replies
    ///     to it
    ///   - `false`: Claude Code adds the output to context without a reply
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`respondToBashCommands`](https://code.claude.com/docs/en/settings-reference#respondtobashcommands).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respond_to_bash_commands: Option<bool>,
    /// Each turn, Claude sees a [listing of your
    /// skills](https://code.claude.com/docs/en/skills#skill-descriptions-are-cut-short) with
    /// their descriptions, and Claude Code caps that listing at a share of the context window.
    /// When the listing is over the cap, Claude Code keeps every skill's name but drops the
    /// descriptions of the least-used skills, so Claude can still invoke those skills but is
    /// less likely to choose one on its own. Raise this key to keep more descriptions visible
    /// at the cost of more context per turn.
    ///
    /// - **Values**: number, a fraction greater than `0` and at most `1`
    /// - **Default**: `0.01`, which reserves 1% of the context window
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`skillListingBudgetFraction`](https://code.claude.com/docs/en/settings-reference#skilllistingbudgetfraction).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_listing_budget_fraction: Option<f64>,
    /// Add SSH connections to the
    /// [Desktop](https://code.claude.com/docs/en/desktop#pre-configure-ssh-connections-for-your-team)
    /// environment selector. Administrators use `sshConfigs` to distribute shared connections
    /// to a team. Users can select a managed connection and [set their own **Worktree
    /// folder**](https://code.claude.com/docs/en/desktop#set-the-worktree-folder-on-an-existing-connection)
    /// for it.
    ///
    /// - **Values**: array of objects, each with required `id`, `name`, and `sshHost` and
    ///   optional `sshPort` and `sshIdentityFile`
    /// - **Default**: unset
    /// - **Scope**: user or managed. The desktop app reads this key.
    ///
    /// Docs: [`sshConfigs`](https://code.claude.com/docs/en/settings-reference#sshconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_configs: Option<Vec<SshConfig>>,
    /// Limit the hosts a [Desktop SSH
    /// session](https://code.claude.com/docs/en/desktop#restrict-which-ssh-hosts-users-can-connect-to)
    /// can connect to. Only the Desktop app reads this key; the CLI doesn't. Patterns are
    /// case-insensitive: `*` matches any host, `*.example.com` matches `example.com` and every
    /// subdomain, and anything else is an exact match against the hostname after
    /// `~/.ssh/config` resolution.
    ///
    /// - **Values**: array of hostname patterns
    /// - **Default**: unset, so any host is allowed
    /// - **Scope**: managed. By default, Desktop reads the key from [one managed
    ///   source](https://code.claude.com/docs/en/managed-settings#how-claude-code-combines-managed-sources).
    ///
    /// Docs: [`sshHostAllowlist`](https://code.claude.com/docs/en/settings-reference#sshhostallowlist).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host_allowlist: Option<Vec<String>>,
    /// Pick the color theme for the interface. Appears in `/config` as **Theme**.
    ///
    /// - **Values**: string, one of:
    ///   - `"auto"`: matches your terminal's light or dark background
    ///   - `"dark"`: the dark theme
    ///   - `"light"`: the light theme
    ///   - `"dark-daltonized"`: the dark theme with colorblind-friendly colors
    ///   - `"light-daltonized"`: the light theme with colorblind-friendly colors
    ///   - `"dark-ansi"`: the dark theme using only your terminal's ANSI color palette
    ///   - `"light-ansi"`: the light theme using only your terminal's ANSI color palette
    ///   - `"custom:<slug>"` or `"custom:<plugin-name>:<slug>"`: a custom theme from
    ///     `~/.claude/themes/` or a plugin
    /// - **Default**: `"dark"`
    /// - **Scope**: any settings file. A value in `~/.claude.json` from an older version
    ///   applies when no settings file sets it.
    ///
    /// Docs: [`theme`](https://code.claude.com/docs/en/settings-reference#theme).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// Turn on [voice dictation](https://code.claude.com/docs/en/voice-dictation) and choose
    /// how the dictation key behaves. Claude Code writes this object for you when you run
    /// `/voice`.
    ///
    /// - **Values**: object with `enabled` as a Boolean, `autoSubmit` as a Boolean that applies
    ///   in hold mode only, and `mode`, one of:
    ///   - `"hold"`: you hold the dictation key while speaking and release it to stop
    ///   - `"tap"`: you tap the key once to start recording and again to send
    /// - **Default**: unset, so dictation is off; when `enabled` is `true` and `mode` is unset,
    ///   Claude Code uses `"hold"`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`voice`](https://code.claude.com/docs/en/settings-reference#voice).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,
    /// Accelerate mouse-wheel scroll speed during fast scrolls in [fullscreen
    /// rendering](https://code.claude.com/docs/en/fullscreen#mouse-wheel-scrolling). Set it to
    /// `false` for a constant scroll rate per wheel notch.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code accelerates mouse-wheel scroll speed during fast scrolls
    ///   - `false`: Claude Code scrolls at a constant rate per wheel notch
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`wheelScrollAccelerationEnabled`](https://code.claude.com/docs/en/settings-reference#wheelscrollaccelerationenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wheel_scroll_acceleration_enabled: Option<bool>,
    /// Turn off the [skills](https://code.claude.com/docs/en/skills) and workflows included
    /// with Claude Code. Claude Code removes bundled skills and workflows entirely, while
    /// built-in commands such as `/init` stay typable but are hidden from the model.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code removes bundled skills and workflows and hides built-in commands
    ///     such as `/init` from the model
    ///   - `false`: bundled skills load
    /// - **Default**: unset, so bundled skills load
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`disableBundledSkills`](https://code.claude.com/docs/en/settings-reference#disablebundledskills).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_bundled_skills: Option<bool>,
    /// Show a one-line session recap when you return to the terminal after a few minutes away.
    /// Set it to `false`, or turn off **Session recap** in `/config`, to stop the recap.
    ///
    /// - **Values**: Boolean
    ///   - `true`: you see a one-line session recap when you return after a few minutes away
    ///   - `false`: Claude Code shows no recap
    /// - **Default**: unset, so the recap is on
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`awaySummaryEnabled`](https://code.claude.com/docs/en/settings-reference#awaysummaryenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub away_summary_enabled: Option<bool>,
    /// Follow new output to the bottom of the conversation in [fullscreen
    /// rendering](https://code.claude.com/docs/en/fullscreen). Turn it off to stay where you
    /// scrolled while Claude keeps working; permission prompts still scroll into view.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the conversation follows new output to the bottom
    ///   - `false`: you stay where you scrolled while Claude keeps working; permission prompts
    ///     still appear below the transcript
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`autoScrollEnabled`](https://code.claude.com/docs/en/settings-reference#autoscrollenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_scroll_enabled: Option<bool>,
    /// Load the [claude.ai
    /// connectors](https://code.claude.com/docs/en/mcp#use-mcp-servers-from-claude-ai) Claude
    /// Code fetches itself alongside a deployed `managed-mcp.json`. Without this key,
    /// `managed-mcp.json` takes exclusive control of MCP servers and suppresses those
    /// connectors.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code loads the claude.ai connectors alongside a deployed
    ///     `managed-mcp.json`
    ///   - `false`: a deployed `managed-mcp.json` takes exclusive control of MCP servers and
    ///     suppresses the claude.ai connectors [Claude Code fetches
    ///     itself](https://code.claude.com/docs/en/mcp#how-connectors-reach-claude-code)
    /// - **Default**: `false`, so a deployed `managed-mcp.json` suppresses the claude.ai
    ///   connectors Claude Code fetches itself
    /// - **Scope**: managed. Users can't re-enable connectors that exclusive control
    ///   suppressed.
    ///
    /// Docs: [`allowAllClaudeAiMcps`](https://code.claude.com/docs/en/settings-reference#allowallclaudeaimcps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_claude_ai_mcps: Option<bool>,
    /// Allow Claude to send a push notification to your phone when it decides one is worth
    /// sending, for example when a long task finishes. In a local session, Claude can send one
    /// while [Remote Control](https://code.claude.com/docs/en/remote-control) is connected.
    /// Appears in `/config` as **Push when Claude decides**. When you change it there, Claude
    /// Code also saves the value to your account.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude can send a push notification to your phone when it decides one is
    ///     worth sending
    ///   - `false`: Claude doesn't send those notifications
    /// - **Default**: `false`
    /// - **Scope**: any settings file. Claude Code also reads a value left in `~/.claude.json`
    ///   by older versions.
    ///
    /// Docs: [`agentPushNotifEnabled`](https://code.claude.com/docs/en/settings-reference#agentpushnotifenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_push_notif_enabled: Option<bool>,
    /// Render screen-reader friendly output: flat text without decorative borders or
    /// animations. Screen-reader mode uses the classic renderer, so the `tui` setting has no
    /// effect while it is active; attached [background
    /// sessions](https://code.claude.com/docs/en/agent-view) still render fullscreen.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code renders flat text without decorative borders or animations,
    ///     using the classic renderer
    ///   - `false`: Claude Code renders normally
    /// - **Default**: unset, so screen-reader mode is off
    /// - **Scope**: user or managed. You can also set the key in a file you pass with
    ///   `--settings`.
    ///
    /// Docs: [`axScreenReader`](https://code.claude.com/docs/en/settings-reference#axscreenreader).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ax_screen_reader: Option<bool>,
    /// Inject CLAUDE.md-style instructions as organization-managed memory without deploying a
    /// separate file. Claude Code loads the text as a managed memory entry ahead of user and
    /// project CLAUDE.md files.
    ///
    /// - **Values**: string, the text of a CLAUDE.md file; write it as you would the file,
    ///   Markdown included, with line breaks as `\n`
    /// - **Default**: unset
    /// - **Scope**: managed.
    ///
    /// Docs: [`claudeMd`](https://code.claude.com/docs/en/settings-reference#claudemd).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_md: Option<String>,
    /// **Warning:** Deprecated, and replaced by
    /// [`enableArtifact`](https://code.claude.com/docs/en/settings-reference#enableartifact).
    /// Claude Code still honors `disableArtifact: true` as equivalent to `enableArtifact:
    /// false`, and ignores `disableArtifact: false`. Use
    /// [`enableArtifact`](https://code.claude.com/docs/en/settings-reference#enableartifact)
    /// instead to turn off the [Artifact](https://code.claude.com/docs/en/artifacts) tool,
    /// which publishes session output as a private web page on claude.ai.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code turns the Artifact tool off for every session the file applies
    ///     to, and no other file turns it back on
    ///   - `false`: ignored; to leave the tool on, remove the key
    /// - **Default**: unset, so the tool follows your account's
    ///   [availability](https://code.claude.com/docs/en/artifacts#availability)
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`disableArtifact`](https://code.claude.com/docs/en/settings-reference#disableartifact).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_artifact: Option<bool>,
    /// Remove [auto
    /// mode](https://code.claude.com/docs/en/permission-modes#eliminate-prompts-with-auto-mode)
    /// from the `Shift+Tab` cycle. Any session that would otherwise [start in auto
    /// mode](https://code.claude.com/docs/en/permission-modes#which-mode-a-session-starts-in),
    /// whether from `--permission-mode auto`, a settings file, or the built-in default, starts
    /// in `default` instead. Administrators set it in managed settings to prevent developers in
    /// their organization from using auto mode.
    ///
    /// - **Values**: the string `"disable"`
    /// - **Default**: unset
    /// - **Scope**: any settings file. Most useful in [managed
    ///   settings](https://code.claude.com/docs/en/managed-settings), where users can't
    ///   override it.
    ///
    /// Docs: [`disableAutoMode`](https://code.claude.com/docs/en/settings-reference#disableautomode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_auto_mode: Option<DisableFlag>,
    /// Turn off the [claude.ai MCP
    /// connectors](https://code.claude.com/docs/en/mcp#use-mcp-servers-from-claude-ai) [Claude
    /// Code fetches
    /// itself](https://code.claude.com/docs/en/mcp#how-connectors-reach-claude-code), so it
    /// neither fetches nor connects them. A `true` in any settings file applies: a checked-in
    /// project `.claude/settings.json` can opt a repository out of those connectors, but a
    /// project-level `false` can't override a user- or managed-level `true`.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code neither fetches nor connects those connectors
    ///   - `false`: the same as unset; Claude Code fetches your connectors unless another
    ///     settings file or `ENABLE_CLAUDEAI_MCP_SERVERS` turns them off
    /// - **Default**: `false`, so Claude Code fetches your connectors
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`disableClaudeAiConnectors`](https://code.claude.com/docs/en/settings-reference#disableclaudeaiconnectors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_claude_ai_connectors: Option<bool>,
    /// Turn off [Remote Control](https://code.claude.com/docs/en/remote-control): Claude Code
    /// then refuses `claude remote-control`, the `--remote-control` flag, auto-start, and the
    /// in-session toggle, and reports that your organization's policy disabled it. Place it in
    /// [managed settings](https://code.claude.com/docs/en/managed-settings) for per-device MDM
    /// enforcement.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code refuses `claude remote-control`, the `--remote-control` flag,
    ///     auto-start, and the in-session toggle
    ///   - `false`: Remote Control stays available
    /// - **Default**: `false`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`disableRemoteControl`](https://code.claude.com/docs/en/settings-reference#disableremotecontrol).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_remote_control: Option<bool>,
    /// Turn off [dynamic
    /// workflows](https://code.claude.com/docs/en/workflows#turn-workflows-off) and the bundled
    /// workflow commands for everyone your settings reach, such as an organization through
    /// managed settings. To turn workflows on or off just for yourself, use
    /// [`enableWorkflows`](https://code.claude.com/docs/en/settings-reference#enableworkflows)
    /// instead, which the **Dynamic workflows** toggle in `/config` writes to your user
    /// settings.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code turns off dynamic workflows and the bundled workflow commands
    ///     for everyone your settings reach
    ///   - `false`: the same as unset; whether workflows are on then follows
    ///     [`enableWorkflows`](https://code.claude.com/docs/en/settings-reference#enableworkflows)
    ///     and your plan's default
    /// - **Default**: `false`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`disableWorkflows`](https://code.claude.com/docs/en/settings-reference#disableworkflows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_workflows: Option<bool>,
    /// Render extra clickable badges in the footer below the input box when a regex matches
    /// turn output: tool results, including file contents and fetched pages, and Claude's own
    /// responses. Use it to turn IDs printed by project CLIs, such as review tools and issue
    /// trackers, into session links.
    ///
    /// - **Values**: array of objects, each with `type` set to `"regex"`, a `pattern` regex, a
    ///   `url` template, and an optional `label`; `{name}` placeholders in `url` and `label`
    ///   are filled from named capture groups in `pattern`
    /// - **Default**: unset, so no badges render
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`footerLinksRegexes`](https://code.claude.com/docs/en/settings-reference#footerlinksregexes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_links_regexes: Option<Vec<FooterLink>>,
    /// Get a push notification on your phone when a permission prompt or question is waiting
    /// for your input. In a local session, Claude Code sends these only while [Remote
    /// Control](https://code.claude.com/docs/en/remote-control) is connected. Appears in
    /// `/config` as **Push when actions required**.
    ///
    /// - **Values**: Boolean
    /// - **Default**: `false`
    /// - **Scope**: any settings file. Claude Code also reads a value left in `~/.claude.json`
    ///   by older versions.
    ///
    /// Docs: [`inputNeededNotifEnabled`](https://code.claude.com/docs/en/settings-reference#inputneedednotifenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_needed_notif_enabled: Option<bool>,
    /// Run an executable you deploy that computes managed settings at startup, so you can
    /// derive policy from device posture, identity, or a remote service instead of a static
    /// file. Claude Code runs the helper before it accepts the first prompt and treats the
    /// settings it emits as the managed settings for the session.
    ///
    /// - **Values**: object with `path`, `timeoutMs`, and `refreshIntervalMs`
    /// - **Default**: unset, so no helper runs
    /// - **Scope**: managed. Read from the macOS plist, the Windows HKLM registry, or the
    ///   managed settings file.
    ///
    /// Docs: [`policyHelper`](https://code.claude.com/docs/en/settings-reference#policyhelper).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_helper: Option<PolicyHelper>,
    /// Connect [Remote Control](https://code.claude.com/docs/en/remote-control) automatically
    /// when each interactive session starts, instead of waiting for `/remote-control`. Set it
    /// to `true` to turn auto-connect on, `false` to turn it off. Appears in `/config` as
    /// **Enable Remote Control for all sessions**.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code connects Remote Control automatically when each interactive
    ///     session starts
    ///   - `false`: Claude Code waits for `/remote-control`
    /// - **Default**: unset, so the [auto-connect
    ///   default](https://code.claude.com/docs/en/remote-control#enable-remote-control-for-all-sessions)
    ///   applies
    /// - **Scope**: any settings file. Claude Code also reads a value left in `~/.claude.json`
    ///   by older versions.
    ///
    /// Docs: [`remoteControlAtStartup`](https://code.claude.com/docs/en/settings-reference#remotecontrolatstartup).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_control_at_startup: Option<bool>,
    /// Claude Code colors code by language in the diffs, code blocks, and file previews it
    /// shows in the terminal, with its built-in highlighter. Set this key to `true` to show
    /// them as plain text instead, for example if the colors clash with your terminal theme or
    /// slow a screen reader.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code turns off syntax highlighting in diffs, code blocks, and file
    ///     previews
    ///   - `false`: Claude Code highlights syntax
    /// - **Default**: `false`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`syntaxHighlightingDisabled`](https://code.claude.com/docs/en/settings-reference#syntaxhighlightingdisabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syntax_highlighting_disabled: Option<bool>,
    /// By default, the transcript collapses each tool call to a short summary, such as the
    /// command Claude ran and a line count of its output, and you press `Ctrl+O` to switch the
    /// whole transcript to the expanded view when you want the details. Set this key to `true`
    /// to show every tool call's full input and output inline as it happens, which is useful
    /// when you're debugging a hook, an MCP server, or a long shell command. Appears in
    /// `/config` as **Verbose output**.
    ///
    /// - **Values**: Boolean
    ///   - `true`: you see full tool output
    ///   - `false`: you see truncated summaries of tool output
    /// - **Default**: `false`
    /// - **Scope**: any settings file. A value in `~/.claude.json` from an older version
    ///   applies when no settings file sets it.
    ///
    /// Docs: [`verbose`](https://code.claude.com/docs/en/settings-reference#verbose).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    /// Choose whether typing the keyword `ultracode` in a prompt triggers a [dynamic
    /// workflow](https://code.claude.com/docs/en/workflows#ask-for-a-workflow-in-your-prompt).
    /// Set it to `false` to type the word without triggering one.
    ///
    /// - **Values**: Boolean
    ///   - `true`: typing `ultracode` in a prompt triggers a dynamic workflow
    ///   - `false`: you can type the word without triggering one
    /// - **Default**: `true`
    /// - **Scope**: any settings file. Appears in `/config` as **Ultracode keyword trigger**.
    ///
    /// Docs: [`workflowKeywordTriggerEnabled`](https://code.claude.com/docs/en/settings-reference#workflowkeywordtriggerenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_keyword_trigger_enabled: Option<bool>,
    /// Let an unanswered [`AskUserQuestion`](https://code.claude.com/docs/en/tools-reference)
    /// dialog auto-continue after a period of idle time, submitting whatever options you had
    /// already selected. Set it when you step away and want Claude to continue without you.
    /// With the default, questions wait until you answer them.
    ///
    /// - **Values**: string, one of `"60s"`, `"5m"`, `"10m"`, or `"never"`
    /// - **Default**: `"never"`
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`askUserQuestionTimeout`](https://code.claude.com/docs/en/settings-reference#askuserquestiontimeout).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_user_question_timeout: Option<DialogTimeout>,
    /// **Warning:** Removed in v2.1.234, together with its `/config` row **Default teammate
    /// model**. Setting it has no effect on current versions. Through v2.1.233, you set this
    /// key to the model for [agent
    /// team](https://code.claude.com/docs/en/agent-teams#specify-teammates-and-models)
    /// teammates your prompt didn't name a model for: an alias such as `"sonnet"`, or `null` to
    /// follow the lead's model. For the model Claude Code picks for such teammates now, see
    /// [specify teammates and
    /// models](https://code.claude.com/docs/en/agent-teams#specify-teammates-and-models).
    ///
    /// - **Values**: string, a model alias or full model ID, or `null`
    /// - **Default**: unset
    /// - **Scope**: global config. On v2.1.233 and earlier.
    ///
    /// Docs: [`teammateDefaultModel`](https://code.claude.com/docs/en/settings-reference#teammatedefaultmodel).
    #[serde(
        default,
        deserialize_with = "nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub teammate_default_model: Option<Option<String>>,
    /// Set the [agent count Claude aims
    /// for](https://code.claude.com/docs/en/workflows#set-a-size-guideline) in the dynamic
    /// workflows it writes. Claude Code sends the value to Claude as advice, not an enforced
    /// cap: `"small"` asks for fewer than 5 agents, `"medium"` fewer than 10, and `"large"`
    /// fewer than 50. Choose `"small"` when you want to bound what a workflow spends.
    ///
    /// - **Values**: string, one of:
    ///   - `"unrestricted"`: no guideline, so Claude sizes the workflow to the task
    ///   - `"small"`: Claude aims for fewer than 5 agents
    ///   - `"medium"`: Claude aims for fewer than 10 agents
    ///   - `"large"`: Claude aims for fewer than 50 agents
    /// - **Default**: `"medium"`, or `"small"` when you're signed in on a Pro plan with Claude
    ///   Code v2.1.271 or later
    /// - **Scope**: any settings file. A value there takes precedence over the **Dynamic
    ///   workflow size** choice in `/config`, which Claude Code stores in `~/.claude.json`, and
    ///   Claude Code hides that row while a settings file sets the key.
    ///
    /// Docs: [`workflowSizeGuideline`](https://code.claude.com/docs/en/settings-reference#workflowsizeguideline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_size_guideline: Option<WorkflowSize>,
    /// Turn off the [Artifact](https://code.claude.com/docs/en/artifacts) tool, which publishes
    /// session output as a private web page on claude.ai. When you turn the **Artifacts** row
    /// off in `/config`, Claude Code writes this key to your user settings, so you don't
    /// usually edit it by hand.
    ///
    /// - **Values**: Boolean
    ///   - `false`: Claude Code turns the Artifact tool off for every session the file applies
    ///     to
    ///   - `true`: the same as leaving the key unset, because it never overrides a `false` from
    ///     another file, from
    ///     [`CLAUDE_CODE_DISABLE_ARTIFACT`](https://code.claude.com/docs/en/env-vars), or from
    ///     your organization's [admin
    ///     setting](https://code.claude.com/docs/en/artifacts#manage-artifacts-for-your-organization)
    /// - **Default**: unset, so the tool follows your account's
    ///   [availability](https://code.claude.com/docs/en/artifacts#availability)
    /// - **Scope**: any settings file. Every file can turn the tool off, and none can turn it
    ///   back on.
    ///
    /// Docs: [`enableArtifact`](https://code.claude.com/docs/en/settings-reference#enableartifact).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_artifact: Option<bool>,
    /// **Warning:** Removed in v2.1.257, together with the `Ctrl+E` command explanation on Bash
    /// and PowerShell permission prompts. Setting it has no effect on current versions. Through
    /// v2.1.256, you could press `Ctrl+E` on a Bash or PowerShell permission prompt to see a
    /// model-generated explanation of the command, and set this key to `false` to turn that
    /// shortcut off.
    ///
    /// - **Values**: Boolean
    /// - **Default**: `true`
    /// - **Scope**: global config. On v2.1.256 and earlier.
    ///
    /// Docs: [`permissionExplainerEnabled`](https://code.claude.com/docs/en/settings-reference#permissionexplainerenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_explainer_enabled: Option<bool>,
    /// Choose where Claude Code shows the diff of an `Edit` or `Write` change it proposes when
    /// a [VS Code](https://code.claude.com/docs/en/vs-code) or
    /// [JetBrains](https://code.claude.com/docs/en/jetbrains#features) IDE is connected:
    /// `"auto"` opens it in the IDE's diff viewer, `"terminal"` keeps it in the terminal.
    /// Appears in `/config` as **Diff tool** only while Claude Code is connected to a VS Code
    /// or JetBrains IDE.
    ///
    /// - **Values**: string, one of:
    ///   - `"auto"`: Claude Code opens the diff in the IDE's diff viewer when a VS Code or
    ///     JetBrains IDE is connected
    ///   - `"terminal"`: Claude Code keeps the diff in the terminal
    /// - **Default**: `"auto"`
    /// - **Scope**: global config.
    ///
    /// Docs: [`diffTool`](https://code.claude.com/docs/en/settings-reference#difftool).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_tool: Option<DiffTool>,
    /// When you press `Ctrl+G`, Claude Code opens the prompt you're typing in your [external
    /// editor](https://code.claude.com/docs/en/interactive-mode#general-controls). With this
    /// key on, the editor buffer starts with Claude's previous response as `#` comment lines,
    /// so you can read it while you write, and Claude Code strips those lines when you save.
    /// Appears in `/config` as **Show last response in external editor**.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the editor buffer starts with Claude's previous response as `#` comment
    ///     lines, which Claude Code strips when you save
    ///   - `false`: the editor buffer opens with only your prompt
    /// - **Default**: `false`
    /// - **Scope**: global config.
    ///
    /// Docs: [`externalEditorContext`](https://code.claude.com/docs/en/settings-reference#externaleditorcontext).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_editor_context: Option<bool>,
    /// Connect to a running IDE automatically when you start Claude Code from an external
    /// terminal. Appears in `/config` as **Auto-connect to IDE (external terminal)** when you
    /// run Claude Code outside a VS Code or JetBrains terminal.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code connects to a running IDE automatically when you start it from
    ///     an external terminal
    ///   - `false`: Claude Code doesn't connect automatically from an external terminal; inside
    ///     a VS Code or JetBrains terminal, or with `--ide`, it still connects
    /// - **Default**: `false`
    /// - **Scope**: global config.
    ///
    /// Docs: [`autoConnectIde`](https://code.claude.com/docs/en/settings-reference#autoconnectide).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_connect_ide: Option<bool>,
    /// Let [`/ide`](https://code.claude.com/docs/en/commands) install the Claude Code IDE
    /// extension in a running VS Code, Cursor, or Devin Desktop when you run the command from
    /// an external terminal. Claude Code doesn't install or update the extension when you start
    /// `claude`. Before v2.1.318, Claude Code installs or updates the extension when you start
    /// `claude` in one of those editors' terminals, unless this key is `false`.
    ///
    /// - **Values**: Boolean
    ///   - `true`: `/ide` installs the extension
    ///   - `false`: `/ide` doesn't install the extension
    /// - **Default**: `true`
    /// - **Scope**: global config.
    ///
    /// Docs: [`autoInstallIdeExtension`](https://code.claude.com/docs/en/settings-reference#autoinstallideextension).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_install_ide_extension: Option<bool>,
    /// Show emoji suggestions when you type `:` plus a shortcode in the prompt input, and
    /// replace a completed shortcode such as `:heart:` with its emoji. Set it to `false` to
    /// turn off both.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code shows emoji suggestions after `:` and replaces a completed
    ///     shortcode with its emoji
    ///   - `false`: Claude Code neither suggests emoji nor replaces shortcodes
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`emojiCompletionEnabled`](https://code.claude.com/docs/en/settings-reference#emojicompletionenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_completion_enabled: Option<bool>,
    /// Map two-key INSERT-mode sequences to Escape in [vim editor
    /// mode](https://code.claude.com/docs/en/interactive-mode#vim-editor-mode). Each key is
    /// exactly two printable characters typed in sequence, and `"<Esc>"` is the only supported
    /// target; Claude Code ignores other entries.
    ///
    /// - **Values**: object mapping a two-character sequence to `"<Esc>"`
    /// - **Default**: unset
    /// - **Scope**: user or managed. A repository can't remap your keystrokes.
    ///
    /// Docs: [`vimInsertModeRemaps`](https://code.claude.com/docs/en/settings-reference#viminsertmoderemaps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vim_insert_mode_remaps: Option<BTreeMap<String, String>>,
    /// On macOS and Linux, place a corporate launcher command in front of the [background
    /// processes Claude Code
    /// starts](https://code.claude.com/docs/en/corporate-launcher#what-the-launcher-covers).
    /// Claude Code runs the launcher with its own command line appended, so the launcher must
    /// exec into Claude Code; see [Run Claude Code behind a corporate
    /// launcher](https://code.claude.com/docs/en/corporate-launcher) for the launcher contract.
    /// Requires Claude Code v2.1.210 or later.
    ///
    /// - **Values**: string, the launcher command as an argv prefix, such as an absolute path
    ///   with optional arguments
    /// - **Default**: unset, so background processes start unwrapped
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`processWrapper`](https://code.claude.com/docs/en/settings-reference#processwrapper).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_wrapper: Option<String>,
    /// Set the gateway URL the `/login` Cloud gateway screen connects to, so people reach your
    /// [cloud gateway](https://code.claude.com/docs/en/claude-apps-gateway) without typing its
    /// address. The screen has no URL field: with this key set, it shows your gateway URL and
    /// connects when the person presses Enter; without it, it tells them to contact their IT
    /// administrator. In managed settings, either this key or `forceLoginMethod: "gateway"`
    /// makes the machine gateway-only, except for sessions that select a cloud provider with
    /// `CLAUDE_CODE_USE_*`.
    ///
    /// - **Values**: string, a full URL including the scheme
    /// - **Default**: unset, so the Cloud gateway screen shows an error telling people to
    ///   contact their IT administrator
    /// - **Scope**: user or managed. Read from a managed source on the machine:
    ///   `managed-settings.json`, the macOS plist or Windows HKLM registry, or a policy helper.
    ///
    /// Docs: [`forceLoginGatewayUrl`](https://code.claude.com/docs/en/settings-reference#forcelogingatewayurl).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_login_gateway_url: Option<String>,
    /// Reject the `--plugin-dir`, `--plugin-url`, `--agents`, and `--mcp-config` CLI flags at
    /// startup, which users could otherwise pass to bypass
    /// [`strictKnownMarketplaces`](https://code.claude.com/docs/en/settings-reference#strictknownmarketplaces)
    /// for a single run. Claude Code exits with an error naming the rejected flags. In [cloud
    /// sessions](https://code.claude.com/docs/en/claude-code-on-the-web), Claude Code instead
    /// starts the session and drops every server-delivered `--mcp-config` entry except
    /// in-process `type: "sdk"` entries and a [Claude
    /// Tag](https://code.claude.com/docs/en/claude-tag) session's Slack tools.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code rejects `--plugin-dir`, `--plugin-url`, `--agents`, and
    ///     `--mcp-config` at startup and exits with an error naming them. In cloud sessions, it
    ///     instead starts the session and drops every server-delivered `--mcp-config` entry
    ///     except in-process `type: "sdk"` entries and a Claude Tag session's Slack tools
    ///   - `false`: Claude Code accepts those flags
    /// - **Default**: `false`
    /// - **Scope**: managed.
    ///
    /// Docs: [`disableSideloadFlags`](https://code.claude.com/docs/en/settings-reference#disablesideloadflags).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_sideload_flags: Option<bool>,
    /// Stop Claude from using its tools to read or act on external pages in the desktop app's
    /// [Browser pane](https://code.claude.com/docs/en/desktop#browse-external-sites). People in
    /// your organization can still open external sites themselves, and local dev server
    /// previews keep working with Claude's tools. The desktop app reads this key; the terminal
    /// CLI ignores it.
    ///
    /// - **Values**: string, `"disabled"`; the desktop app also accepts `"disable"`, in either
    ///   case
    /// - **Default**: unset, so Claude's tools work on external pages
    /// - **Scope**: managed.
    ///
    /// Docs: [`browserExternalPageTools`](https://code.claude.com/docs/en/settings-reference#browserexternalpagetools).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_external_page_tools: Option<DisabledFlag>,
    /// Turn off external browsing in the desktop app's [Browser
    /// pane](https://code.claude.com/docs/en/desktop#browse-external-sites) for people and
    /// Claude alike. Localhost dev server previews keep working. HTML previews in SSH and WSL
    /// sessions still open, but they [don't run
    /// scripts](https://code.claude.com/docs/en/desktop#turn-off-scripts-in-ssh-and-wsl-previews-for-your-organization).
    ///
    /// - **Values**: Boolean; only the JSON Boolean `true` takes effect
    ///   - `true`: the desktop app turns off external browsing in the Browser pane for people
    ///     and Claude alike; localhost previews keep working
    ///   - `false`: external browsing stays on
    /// - **Default**: unset, so external browsing is on
    /// - **Scope**: managed.
    ///
    /// Docs: [`disableBrowserExternalNavigation`](https://code.claude.com/docs/en/settings-reference#disablebrowserexternalnavigation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_browser_external_navigation: Option<bool>,
    /// Block Claude's tools for the desktop app's [iOS Simulator
    /// pane](https://code.claude.com/docs/en/desktop-ios-simulator#turn-off-simulator-access).
    /// People keep manual use of the pane; only Claude's access is removed, and nobody can turn
    /// it back on from inside the app. The desktop app reads this key; the terminal CLI ignores
    /// it.
    ///
    /// - **Values**: Boolean; only the JSON Boolean `true` takes effect
    ///   - `true`: the desktop app blocks Claude's tools for the iOS Simulator pane
    ///   - `false`: Claude's simulator tools follow each person's settings toggle in the
    ///     desktop app
    /// - **Default**: unset, so Claude's simulator tools follow each person's settings toggle
    ///   in the desktop app
    /// - **Scope**: managed.
    ///
    /// Docs: [`disableMobileSimulatorTools`](https://code.claude.com/docs/en/settings-reference#disablemobilesimulatortools).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_mobile_simulator_tools: Option<bool>,
    /// (Managed settings only) Run Claude's tools inside an isolated virtual machine instead of
    /// on your Mac. Disables the desktop app's iOS Simulator pane and Claude's simulator tools
    /// entirely, so the pane cannot attach a device while it is set.
    ///
    /// - **Values**: Boolean
    /// - **Default**: unset
    ///
    /// Docs: not in the settings reference; listed in the
    /// [JSON schema](https://json.schemastore.org/claude-code-settings.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_cowork_full_vm_sandbox: Option<bool>,
    /// Let the built-in [Claude in Chrome](https://code.claude.com/docs/en/chrome) server run
    /// alongside a deployed `managed-mcp.json`. Without this key, a deployed `managed-mcp.json`
    /// blocks Claude in Chrome in terminal sessions. Requires Claude Code v2.1.282 or later.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the built-in Claude in Chrome server can run alongside a deployed
    ///     `managed-mcp.json`
    ///   - `false`: a deployed `managed-mcp.json` blocks Claude in Chrome in terminal sessions
    /// - **Default**: `false`, so a deployed `managed-mcp.json` blocks Claude in Chrome in
    ///   terminal sessions
    /// - **Scope**: managed, from the device's own managed settings only: an MDM-deployed plist
    ///   or HKLM registry key, or a system `managed-settings.json` file. Claude Code ignores it
    ///   in server-managed settings, in the user-writable HKCU registry, and in user or project
    ///   settings.
    ///
    /// Docs: [`allowClaudeInChromeWithManagedMcp`](https://code.claude.com/docs/en/settings-reference#allowclaudeinchromewithmanagedmcp).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_claude_in_chrome_with_managed_mcp: Option<bool>,
    /// List the services a machine may reach Claude through, such as the Anthropic API, Amazon
    /// Bedrock, or an LLM gateway. A session on a provider that isn't listed is refused at
    /// startup, at login, and when it next contacts the API, so switching to an unlisted
    /// provider mid-session is refused too. The [refusal
    /// message](https://code.claude.com/docs/en/errors#managed-settings-dont-allow-this-api-provider)
    /// names what selected the provider and the steps to continue.
    ///
    /// - **Values**: array of strings, each one of:
    ///   - `"anthropic"`: the Anthropic API on Anthropic's own host, through a claude.ai or
    ///     Console sign-in or an API key. Pair it with
    ///     [`forceLoginMethod`](https://code.claude.com/docs/en/settings-reference#forceloginmethod)
    ///     or
    ///     [`forceLoginOrgUUID`](https://code.claude.com/docs/en/settings-reference#forceloginorguuid)
    ///     to also restrict the sign-in
    ///   - `"bedrock"`: [Amazon Bedrock](https://code.claude.com/docs/en/amazon-bedrock)
    ///   - `"vertex"`: [Google Cloud's Agent
    ///     Platform](https://code.claude.com/docs/en/google-vertex-ai), formerly Vertex AI
    ///   - `"foundry"`: [Microsoft Foundry](https://code.claude.com/docs/en/microsoft-foundry)
    ///   - `"anthropicAws"`: [Claude Platform on
    ///     AWS](https://code.claude.com/docs/en/claude-platform-on-aws)
    ///   - `"mantle"`: the Amazon Bedrock [Mantle
    ///     endpoint](https://code.claude.com/docs/en/amazon-bedrock#use-the-mantle-endpoint). A
    ///     session that [runs Mantle alongside the Invoke
    ///     API](https://code.claude.com/docs/en/amazon-bedrock#run-mantle-alongside-the-invoke-api)
    ///     uses both providers, so list `"bedrock"` and `"mantle"` together for it
    ///   - `"customEndpoint"`: the Anthropic API or a cloud provider's API sent to another
    ///     host, such as an [LLM gateway](https://code.claude.com/docs/en/llm-gateway) named by
    ///     `ANTHROPIC_BASE_URL` or by a provider's endpoint variable such as
    ///     `ANTHROPIC_BEDROCK_BASE_URL`. Claude Code admits it only for the exact value a
    ///     managed [`env`](https://code.claude.com/docs/en/settings-reference#env) block pins
    ///   - `"gateway"`: a [Cloud gateway](https://code.claude.com/docs/en/claude-apps-gateway)
    ///     sign-in
    /// - **Default**: unset, so any provider can be used
    /// - **Scope**: managed. A list that the machine's own admin sources set, MDM policies and
    ///   managed settings files, keeps applying when server-managed settings also deliver one:
    ///   a session may then use only the providers on both lists, so a server-managed list can
    ///   narrow what the machine allows but never widen it.
    ///
    /// Docs: [`allowedProviders`](https://code.claude.com/docs/en/settings-reference#allowedproviders).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_providers: Option<Vec<Provider>>,
    /// List the managed plugins whose
    /// [mods](https://code.claude.com/docs/en/plugins/mods/overview) run after every mod a user
    /// installs, in the listed order. An id listed in both `prependPlugins` and `appendPlugins`
    /// is prepended. In managed settings, Claude Code skips an id whose plugin doesn't [count
    /// as your
    /// organization's](https://code.claude.com/docs/en/plugins/mods/admin#install-your-organizations-mods).
    ///
    /// - **Values**: array of `plugin-name@marketplace-name` strings
    /// - **Default**: unset
    /// - **Scope**: user or managed. Claude Code reads the key from managed settings.
    ///
    /// Docs: [`appendPlugins`](https://code.claude.com/docs/en/settings-reference#appendplugins).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append_plugins: Option<Vec<String>>,
    /// Set how full the context window gets before Claude Code [compacts
    /// automatically](https://code.claude.com/docs/en/context-window#when-your-context-fills-up).
    ///
    /// - **Values**: number of tokens, from `100000` to `1000000`. Claude Code caps the value
    ///   at your model's context window; the [models
    ///   overview](https://platform.claude.com/docs/en/about-claude/models/overview) lists each
    ///   model's window
    /// - **Default**: unset, so Claude Code picks a window tuned for your model
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`autoCompactWindow`](https://code.claude.com/docs/en/settings-reference#autocompactwindow).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_window: Option<u64>,
    /// After a claude.ai usage limit stops your session, wait in the open session and continue
    /// the task automatically after the reset. See [Turn automatic continue
    /// off](https://code.claude.com/docs/en/interactive-mode#turn-automatic-continue-off).
    /// Requires Claude Code v2.1.234 or later.
    ///
    /// - **Values**: Boolean
    ///   - `true`: after a claude.ai usage limit stops your session, Claude Code waits in the
    ///     open session and continues the task automatically after the reset
    ///   - `false`: Claude Code doesn't start the wait on its own. You can still [start a wait
    ///     yourself](https://code.claude.com/docs/en/interactive-mode#start-a-wait-yourself)
    ///     from the usage-limit options menu
    /// - **Default**: `true`
    /// - **Scope**: user or managed. Read from user settings, `--settings`, and managed
    ///   settings only.
    ///
    /// Docs: [`autoContinueAtUsageLimit`](https://code.claude.com/docs/en/settings-reference#autocontinueatusagelimit).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_continue_at_usage_limit: Option<bool>,
    /// Choose how
    /// [`availableModels`](https://code.claude.com/docs/en/settings-reference#availablemodels)
    /// entries match model IDs. By default a model ID entry also permits later versions that
    /// extend it, so `"claude-opus-5"` permits Opus 5.5. With `"exact"`, each model ID entry
    /// permits only the version it names, so a newer version of that model stays blocked until
    /// you list it.
    ///
    /// - **Values**: string, one of:
    ///   - `"prefix"`: a model ID entry permits its version and any model ID that extends it
    ///     with another segment
    ///   - `"exact"`: a model ID entry permits only the version it names, so `"claude-opus-5"`
    ///     permits Opus 5 but not `claude-opus-5-5`. An entry such as `"claude-opus-4-5"` also
    ///     permits its dated Anthropic model ID `claude-opus-4-5-20251101`. A family alias such
    ///     as `"opus"` still permits the whole family, and `best`, `opusplan`, and `default`
    ///     entries are ignored
    /// - **Default**: `"prefix"`
    /// - **Scope**: managed. Claude Code ignores the key in user, project, and local settings
    ///   and in `--settings`, with a warning.
    ///
    /// Docs: [`availableModelsMatch`](https://code.claude.com/docs/en/settings-reference#availablemodelsmatch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_models_match: Option<ModelMatch>,
    /// Choose whether Claude Code records which files changed in a Git repository while a Bash
    /// command runs. When it records them, you see their diff in the terminal after the
    /// command, and your [PostToolUse Bash hooks](https://code.claude.com/docs/en/hooks#bash)
    /// receive the changed-file list. A listed file isn't always one the command changed.
    ///
    /// - **Values**: Boolean
    /// - **Default**: unset, so Claude Code records changes in auto mode and
    ///   `bypassPermissions` mode when it directs Claude to edit files through Bash
    /// - **Scope**: user or managed. A `true` counts only from your user settings, JSON passed
    ///   with `--settings`, or [managed
    ///   settings](https://code.claude.com/docs/en/managed-settings), so a `true` in a
    ///   repository's `.claude/settings.json` or `.claude/settings.local.json` can't turn the
    ///   recording on.
    ///
    /// Docs: [`bashEditDiffEnabled`](https://code.claude.com/docs/en/settings-reference#basheditdiffenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bash_edit_diff_enabled: Option<bool>,
    /// Set how many characters of a successful Bash or PowerShell command's [output Claude
    /// receives inline](https://code.claude.com/docs/en/tools-reference#output-limits). When
    /// output passes the limit, Claude Code saves it to a file and Claude receives a short
    /// preview plus the file's path. Raise the limit when command output, such as a verbose
    /// build or a full test-suite log, routinely overflows the default and you want Claude to
    /// read it without opening the file.
    ///
    /// - **Values**: number of characters, a positive integer. Claude Code clamps the value
    ///   into the range `4000` to `128000`
    /// - **Default**: unset, so Claude receives up to 30,000 characters inline
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`bashOutputMaxChars`](https://code.claude.com/docs/en/settings-reference#bashoutputmaxchars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bash_output_max_chars: Option<u64>,
    /// Choose what this session does with [messages arriving from your other Claude Code
    /// sessions](https://code.claude.com/docs/en/cross-session-messaging#control-inbound-messages).
    /// When no value applies, Claude Code decides per message from the two sessions'
    /// permission-mode classes.
    ///
    /// - **Values**: string, one of:
    ///   - `"accept"`: Claude Code delivers the message to Claude
    ///   - `"hold"`: Claude Code shows a notice for the message without delivering it
    ///   - `"refuse"`: Claude Code drops the message
    /// - **Default**: unset, so Claude Code decides per message
    /// - **Scope**: any settings file. A project or local value applies only when it's stricter
    ///   than the value managed settings, the `--settings` flag, or user settings give.
    ///
    /// Docs: [`crossSessionInbound`](https://code.claude.com/docs/en/settings-reference#crosssessioninbound).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_session_inbound: Option<InboundMode>,
    /// Block specific models, with or without an
    /// [`availableModels`](https://code.claude.com/docs/en/settings-reference#availablemodels)
    /// allowlist and even when that list permits them. Claude Code hides a blocked model from
    /// the `/model` picker, and the model can't be selected anywhere `availableModels` is
    /// enforced. A session on the Default option doesn't run a blocked model either, as [Block
    /// specific models or
    /// versions](https://code.claude.com/docs/en/model-config#block-specific-models-or-versions)
    /// describes.
    ///
    /// - **Values**: array of model aliases or IDs
    ///   - A family alias such as `"opus"` blocks every model in that family
    ///   - A model ID such as `"claude-opus-5-5"` blocks that version in every spelling,
    ///     including dated and provider-specific IDs
    ///   - A model ID with no minor version, such as `"claude-opus-5"`, also blocks later minor
    ///     versions such as Opus 5.5. Write `"claude-opus-5-0"` to block Opus 5 alone
    ///   - `best`, `opusplan`, and `default` entries are ignored
    /// - **Default**: unset, so no model is blocked
    /// - **Scope**: managed. Claude Code ignores the key in user, project, and local settings
    ///   and in `--settings`, with a warning.
    ///
    /// Docs: [`deniedModels`](https://code.claude.com/docs/en/settings-reference#deniedmodels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_models: Option<Vec<String>>,
    /// Set an age limit in days for the transcripts of sessions you started or most recently
    /// continued in Claude Desktop or Cowork. Without this key, Claude Code [keeps those
    /// transcripts at any
    /// age](https://code.claude.com/docs/en/claude-directory#cleaned-up-automatically), except
    /// in cases such as a session you deleted in Claude Desktop. Requires Claude Code v2.1.248
    /// or later.
    ///
    /// - **Values**: number of days, a whole number, minimum `0`
    /// - **Default**: `0`, which sets no age limit
    /// - **Scope**: user or managed. Claude Code also reads the key from a file you pass with
    ///   `--settings`, and ignores it in project and local settings.
    ///
    /// Docs: [`desktopSessionCleanupPeriodDays`](https://code.claude.com/docs/en/settings-reference#desktopsessioncleanupperioddays).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop_session_cleanup_period_days: Option<u64>,
    /// Set the deadline for dialogs Claude Code [forwards to a remote
    /// client](https://code.claude.com/docs/en/remote-control#limitations), such as a Remote
    /// Control or SDK host, and for the approval dialog for a [held cross-session
    /// message](https://code.claude.com/docs/en/cross-session-messaging#control-inbound-messages).
    /// On Claude Code v2.1.236 or later, the same deadline bounds the mid-session [Fable
    /// usage-credits consent
    /// prompt](https://code.claude.com/docs/en/model-config#fable-and-usage-credits) in a
    /// session that may have nobody at the terminal. When no answer arrives before the
    /// deadline, Claude Code cancels the dialog and continues with its no-action default.
    ///
    /// - **Values**: string, one of `"60s"`, `"5m"`, `"10m"`, or `"never"`, which disables the
    ///   deadline
    /// - **Default**: `"5m"`
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`dialogExpiry`](https://code.claude.com/docs/en/settings-reference#dialogexpiry).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialog_expiry: Option<DialogTimeout>,
    /// Block the [`command` plugin
    /// source](https://code.claude.com/docs/en/plugins/marketplace-reference#command-plugin-source),
    /// which installs a plugin by running a marketplace-declared command on the user's machine.
    /// When you set it to `true`, Claude Code never runs the command, doesn't install or update
    /// command-sourced plugins, and stops loading the ones already installed. Set it to `false`
    /// to allow them explicitly.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code never runs the marketplace-declared command, doesn't install or
    ///     update command-sourced plugins, and stops loading the ones already installed
    ///   - `false`: Claude Code allows command-sourced plugins explicitly
    /// - **Default**: unset, so Claude Code follows
    ///   [`allowManagedHooksOnly`](https://code.claude.com/docs/en/settings-reference#allowmanagedhooksonly):
    ///   an organization that restricts hook execution to managed settings gets command sources
    ///   disabled too. An application that passes [`disableAllHooks:
    ///   true`](https://code.claude.com/docs/en/settings-reference#disableallhooks) in the
    ///   Agent SDK's `managedSettings` option doesn't by itself disable command sources.
    /// - **Scope**: managed.
    ///
    /// Docs: [`disableCommandPluginSources`](https://code.claude.com/docs/en/settings-reference#disablecommandpluginsources).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_command_plugin_sources: Option<bool>,
    /// Turn off Code sessions that run on the device in the [desktop
    /// app](https://code.claude.com/docs/en/desktop#local-sessions-on-managed-devices), for
    /// deployments where developers should work on remote machines over SSH. With the key set
    /// to `true`, users see this in the Code tab:
    ///
    /// - **Values**: Boolean; only the JSON Boolean `true` takes effect
    ///   - `true`: the desktop app offers no on-device Code sessions; existing local sessions
    ///     stay listed but can't continue
    ///   - `false`: local sessions stay available
    /// - **Default**: unset, so local sessions are available
    /// - **Scope**: managed. By default, the desktop app reads the key from [one managed
    ///   source](https://code.claude.com/docs/en/managed-settings#how-claude-code-combines-managed-sources).
    ///
    /// Docs: [`disableDesktopLocalSessions`](https://code.claude.com/docs/en/settings-reference#disabledesktoplocalsessions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_desktop_local_sessions: Option<bool>,
    /// Keep scripts from running when people [preview an HTML
    /// file](https://code.claude.com/docs/en/desktop#preview-html-files-in-ssh-and-wsl-sessions)
    /// from an SSH or WSL session in the desktop app. External browsing in the Browser pane is
    /// unaffected. The desktop app reads this key; the terminal CLI ignores it.
    ///
    /// - **Values**: Boolean; only the JSON Boolean `true` takes effect
    ///   - `true`: HTML previews in SSH and WSL sessions don't run scripts
    ///   - `false`: the key doesn't turn scripts off
    /// - **Default**: unset, so the key has no effect
    /// - **Scope**: managed. The desktop app doesn't read the key from the remote machine or
    ///   the WSL distribution.
    ///
    /// Docs: [`disableRemotePreviewScripts`](https://code.claude.com/docs/en/settings-reference#disableremotepreviewscripts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_remote_preview_scripts: Option<bool>,
    /// Turn [dynamic workflows](https://code.claude.com/docs/en/workflows) on or off for
    /// yourself when your plan's default isn't what you want. Appears in `/config` as **Dynamic
    /// workflows**, which writes this key to your user settings and removes it again when you
    /// toggle back to your plan's default. To turn workflows off for everyone from managed
    /// settings, use
    /// [`disableWorkflows`](https://code.claude.com/docs/en/settings-reference#disableworkflows)
    /// instead.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code turns dynamic workflows on for you
    ///   - `false`: Claude Code turns dynamic workflows off for you
    /// - **Default**: unset, so workflows are on unless you're on the Pro plan, where they're
    ///   off
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`enableWorkflows`](https://code.claude.com/docs/en/settings-reference#enableworkflows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_workflows: Option<bool>,
    /// Control [Claude-drafted
    /// feedback](https://code.claude.com/docs/en/tools-reference#sendfeedback-tool-behavior):
    /// whether Claude can queue feedback drafts for you to review, and whether Claude Code
    /// shows a card when Claude queues one.
    ///
    /// - **Values**: string, one of `"notify"`, `"quiet"`, or `"off"`
    ///   - `"notify"`: Claude Code shows a card above the prompt when Claude queues a draft, up
    ///     to [three cards in a
    ///     session](https://code.claude.com/docs/en/tools-reference#what-you-see-when-claude-drafts)
    ///     by default
    ///   - `"quiet"`: Claude drafts without a card. You see the count of queued drafts in the
    ///     prompt footer and review them in `/feedback`
    ///   - `"off"`: Claude Code removes the SendFeedback tool, so Claude can't queue drafts
    /// - **Default**: `"notify"`
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`feedbackDrafts`](https://code.claude.com/docs/en/settings-reference#feedbackdrafts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_drafts: Option<FeedbackDrafts>,
    /// Declare the public IPv4 blocks that your organization numbers its internal network from,
    /// so `/login` accepts a [cloud
    /// gateway](https://code.claude.com/docs/en/claude-apps-gateway) there. Requires Claude
    /// Code v2.1.268 or later. Without this key, `/login` connects to any gateway on a private
    /// address and nothing else.
    ///
    /// - **Values**: array of strings, at most four IPv4 CIDR blocks, each `/8` to `/32`, not
    ///   overlapping one another, and none overlapping private space.
    /// - **Default**: unset, so `/login` accepts only gateways on private addresses
    /// - **Scope**: managed. Read only from a source on the machine: `managed-settings.json`,
    ///   the macOS plist or Windows HKLM registry, or a policy helper.
    ///
    /// Docs: [`gatewayInternalNetworks`](https://code.claude.com/docs/en/settings-reference#gatewayinternalnetworks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_internal_networks: Option<Vec<String>>,
    /// Idle compaction
    /// [compacts](https://code.claude.com/docs/en/context-window#what-survives-compaction) a
    /// long conversation that you've stepped away from. Your next message then sends a summary
    /// instead of the whole conversation. Once compacted, the conversation shows a notice:
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code compacts idle conversations that meet the conditions
    ///   - `false`: Claude Code doesn't compact while your session is idle
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`idleCompaction`](https://code.claude.com/docs/en/settings-reference#idlecompaction).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_compaction: Option<bool>,
    /// Require your explicit approval before Claude's `SendMessage` reaches one of your
    /// sessions beyond this machine; see [Require approval for cross-machine
    /// messages](https://code.claude.com/docs/en/cross-session-messaging#require-approval-for-cross-machine-messages).
    /// The approval prompt appears even in [`bypassPermissions`
    /// mode](https://code.claude.com/docs/en/permission-modes#skip-all-checks-with-bypasspermissions-mode).
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code asks for your approval before Claude's `SendMessage` reaches one
    ///     of your sessions beyond this machine
    ///   - `false`: cross-machine messages don't prompt
    /// - **Default**: unset, so cross-machine messages don't prompt
    /// - **Scope**: any settings file. A `true` from any scope applies, so a checked-in project
    ///   file can turn the requirement on but not off.
    ///
    /// Docs: [`isolatePeerMachines`](https://code.claude.com/docs/en/settings-reference#isolatepeermachines).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolate_peer_machines: Option<bool>,
    /// **Warning:** Deprecated since v2.1.261 and has no effect. The prompt's word-editing keys
    /// always [follow readline
    /// conventions](https://code.claude.com/docs/en/interactive-mode#make-ctrl-w-delete-back-to-whitespace),
    /// as in Bash. Claude Code still accepts `keybindingFlavor`, so a settings file that sets
    /// it stays valid.
    ///
    /// - **Values**: string, `"classic"` or `"readline"`
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`keybindingFlavor`](https://code.claude.com/docs/en/settings-reference#keybindingflavor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keybinding_flavor: Option<KeybindingFlavor>,
    /// Choose whether Claude Code applies only the highest-priority [managed
    /// source](https://code.claude.com/docs/en/managed-settings#how-claude-code-combines-managed-sources)
    /// your organization delivers, or combines every admin source it delivers. By default
    /// Claude Code takes the highest-priority source that carries a [policy
    /// key](https://code.claude.com/docs/en/managed-settings#how-claude-code-combines-managed-sources)
    /// and ignores the rest. A policy key is any settings key other than this one and
    /// `wslInheritsWindowsSettings`.
    ///
    /// - **Values**: string, one of:
    ///   - `"first-wins"`: the highest-priority source that carries a policy key supplies the
    ///     policy, and lower sources contribute only the [keys Claude Code reads from every
    ///     admin
    ///     source](https://code.claude.com/docs/en/managed-settings#keys-read-from-every-admin-source)
    ///   - `"merge"`: every admin source you deliver contributes its keys, combined by the
    ///     rules below
    /// - **Default**: `"first-wins"`
    /// - **Scope**: managed. Claude Code reads this key from the highest-priority source that
    ///   carries either this key or a policy key, and ignores this key in every source ranked
    ///   lower, so a lower source can't opt itself into combining with the source above it.
    ///
    /// Docs: [`managedSourcesBehavior`](https://code.claude.com/docs/en/settings-reference#managedsourcesbehavior).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_sources_behavior: Option<SourcesBehavior>,
    /// Cap the [effort level](https://code.claude.com/docs/en/model-config#adjust-effort-level)
    /// a session can use, leaving lower levels available. Any higher level runs at the cap
    /// instead, including one from `/effort`, the `/model` picker, `--effort`,
    /// [`CLAUDE_CODE_EFFORT_LEVEL`](https://code.claude.com/docs/en/env-vars), a skill's or
    /// subagent's `effort` frontmatter, or the model's own default. Claude Code applies the cap
    /// itself before each request, so it holds on every provider, including Amazon Bedrock,
    /// Google Cloud's Agent Platform, and Microsoft Foundry.
    ///
    /// - **Values**: string, one of `"low"`, `"medium"`, `"high"`, `"xhigh"`, or `"max"`. A
    ///   `"max"` value sets no cap
    /// - **Default**: unset, so no cap applies
    /// - **Scope**: any settings file. Deploy it in managed settings to enforce it for an
    ///   organization.
    ///
    /// Docs: [`maxEffortLevel`](https://code.claude.com/docs/en/settings-reference#maxeffortlevel).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_effort_level: Option<EffortLevel>,
    /// Cap the width of the prose in Claude's responses so lines stay readable in a wide
    /// terminal. Paragraphs, headings, lists, and blockquotes wrap within this many columns,
    /// while tables and code blocks keep the full terminal width. Requires Claude Code v2.1.282
    /// or later.
    ///
    /// - **Values**: number of terminal columns, a whole number, minimum `40`. Claude Code
    ///   ignores any other value
    /// - **Default**: unset, so prose wraps at the terminal edge
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`maxProseWidth`](https://code.claude.com/docs/en/settings-reference#maxprosewidth).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_prose_width: Option<u64>,
    /// List the models the `/model` picker offers, in the order you write them and under labels
    /// you choose, so the picker lists the models your organization runs, after the built-in
    /// lineup or instead of it. Each row's `model` is taken verbatim, so it accepts anything
    /// `--model` accepts: an alias such as `opus`, an Anthropic model ID, or a provider-format
    /// ID for Amazon Bedrock, Google Cloud's Agent Platform, Microsoft Foundry, or an LLM
    /// gateway. Requires Claude Code v2.1.242 or later.
    ///
    /// - **Values**: object with an `options` array of rows and an optional
    ///   `replaceBuiltInOptions` Boolean
    /// - **Default**: unset, so the picker shows the built-in lineup
    /// - **Scope**: user or managed. Claude Code reads the key from managed settings,
    ///   `--settings`, and user settings, and ignores it in project and local settings so a
    ///   repository you clone can't relabel the picker.
    ///
    /// Docs: [`modelPicker`](https://code.claude.com/docs/en/settings-reference#modelpicker).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_picker: Option<ModelPicker>,
    /// Report spend at the rates your organization pays instead of list price. Set it when your
    /// organization has contracted rates, so the dollar figures developers see match your bill.
    /// Claude Code applies the rates in `/usage`, the [status
    /// line](https://code.claude.com/docs/en/statusline), the Agent SDK's `total_cost_usd`, the
    /// [`--max-budget-usd`](https://code.claude.com/docs/en/cli-reference) limit, and the
    /// [OpenTelemetry](https://code.claude.com/docs/en/monitoring-usage) cost metric and
    /// events.
    ///
    /// - **Values**: object with an optional `multiplier` and an optional `overrides` map
    /// - **Default**: unset, so Claude Code reports list price unless a host application
    ///   supplies a table
    /// - **Scope**: managed. Deploy the key through server-managed settings, an MDM policy, a
    ///   `managed-settings.json` file, or a [policy
    ///   helper](https://code.claude.com/docs/en/managed-settings#compute-the-policy-with-a-helper-program).
    ///
    /// Docs: [`modelPricing`](https://code.claude.com/docs/en/settings-reference#modelpricing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_pricing: Option<ModelPricing>,
    /// Save an [effort level](https://code.claude.com/docs/en/model-config#adjust-effort-level)
    /// for each model you use. Requires Claude Code v2.1.251 or later. In an interactive
    /// session on your machine, when you save `low`, `medium`, `high`, or `xhigh` as your
    /// default with `/effort` or the `/model` picker's effort slider, Claude Code writes that
    /// level here under the model you're using, so you rarely edit this key yourself.
    ///
    /// - **Values**: object mapping a model name to an object with any of these fields:
    ///   - `effortLevel`: one of `"low"`, `"medium"`, `"high"`, or `"xhigh"`
    ///   - [`maxEffortLevel`](https://code.claude.com/docs/en/settings-reference#maxeffortlevel):
    ///     the highest effort level the model may run at
    ///   - `autoCompactWindow`: a number of tokens from `100000` to `1000000`, or `"auto"` for
    ///     the window tuned for the model.
    ///     [`/autocompact`](https://code.claude.com/docs/en/model-config#set-the-auto-compact-window)
    ///     saves here. For that model, the value takes precedence over a top-level
    ///     [`autoCompactWindow`](https://code.claude.com/docs/en/settings-reference#autocompactwindow)
    ///     in the same settings file. Requires Claude Code v2.1.288 or later
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`modelSettings`](https://code.claude.com/docs/en/settings-reference#modelsettings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_settings: Option<BTreeMap<String, ModelSettings>>,
    /// List the managed plugins whose
    /// [mods](https://code.claude.com/docs/en/plugins/mods/overview) run before every mod a
    /// user installs, in the listed order. When you set this key in managed settings, name
    /// `sec-default@builtin` in the list to keep the built-in guard. In managed settings,
    /// Claude Code skips an id whose plugin doesn't count as your organization's.
    ///
    /// - **Values**: array of `plugin-name@marketplace-name` strings
    /// - **Default**: unset
    /// - **Scope**: user or managed. Claude Code reads the key from managed settings.
    ///
    /// Docs: [`prependPlugins`](https://code.claude.com/docs/en/settings-reference#prependplugins).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepend_plugins: Option<Vec<String>>,
    /// Choose how long the [prompt cache](https://code.claude.com/docs/en/prompt-caching) holds
    /// the main conversation. This key applies to your interactive, `-p`, and Agent SDK turns,
    /// together with the helpers Claude Code runs inline with them. The one-hour lifetime keeps
    /// the cache warm across longer breaks, and the API [bills each cache write at a higher
    /// rate](https://platform.claude.com/docs/en/build-with-claude/prompt-caching#pricing) than
    /// at the five-minute lifetime.
    ///
    /// - **Values**: string, one of:
    ///   - `"5m"`: the cache holds for five minutes
    ///   - `"1h"`: the cache holds for an hour
    /// - **Default**: unset, so each main-conversation request gets [its default
    ///   lifetime](https://code.claude.com/docs/en/prompt-caching#which-ttl-each-request-gets)
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`promptCacheTtl`](https://code.claude.com/docs/en/settings-reference#promptcachettl).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_ttl: Option<CacheTtl>,
    /// Show or hide [prompt
    /// suggestions](https://code.claude.com/docs/en/interactive-mode#prompt-suggestions), the
    /// grayed-out predictions that appear in your prompt input. Set it to `false`, or turn off
    /// **Prompt suggestions** in `/config`, to hide them.
    ///
    /// - **Values**: Boolean
    ///   - `true`: you see prompt suggestions in your prompt input
    ///   - `false`: Claude Code hides prompt suggestions
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`promptSuggestionEnabled`](https://code.claude.com/docs/en/settings-reference#promptsuggestionenabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_suggestion_enabled: Option<bool>,
    /// Skip the one-time notice describing [auto
    /// mode](https://code.claude.com/docs/en/permission-modes#eliminate-prompts-with-auto-mode)
    /// that Claude Code shows when you first enter auto mode yourself, for example through your
    /// own settings or the mode selector, rather than when the built-in default starts a
    /// session in it. Claude Code shows that notice once and then records that it was shown, so
    /// this key only matters where the notice hasn't appeared yet.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code skips the notice
    ///   - `false`: the same as unset; the notice appears once unless another of these files
    ///     sets `true`
    /// - **Default**: unset, so the notice appears once
    /// - **Scope**: user or managed. A repository can't set it for you.
    ///
    /// Docs: [`skipAutoPermissionPrompt`](https://code.claude.com/docs/en/settings-reference#skipautopermissionprompt).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_auto_permission_prompt: Option<bool>,
    /// Underline misspelled words in the prompt input as you type, using a spell checker you
    /// install. Claude Code checks only the text in the input box. [Check spelling as you
    /// type](https://code.claude.com/docs/en/interactive-mode#check-spelling-as-you-type)
    /// covers installing aspell, hunspell, or ispell and what the checker covers.
    ///
    /// - **Values**: object with `enabled` (Boolean), `checker` (`"aspell"`, `"hunspell"`,
    ///   `"ispell"`, or `"auto"`), `language` (string, passed to the checker as its dictionary
    ///   name), and `color` (string, a terminal color name, `#rrggbb`, `rgb(r,g,b)`,
    ///   `ansi256(n)`, or `ansi:<name>`)
    /// - **Default**: unset, so spell checking is off; `checker` defaults to `"auto"`, the
    ///   first of the three found on `PATH`; `language` defaults to the checker's own
    ///   dictionary; `color` defaults to the theme's error color
    /// - **Scope**: user or managed. The block from the highest tier that sets it applies as a
    ///   whole.
    ///
    /// Docs: [`spellcheck`](https://code.claude.com/docs/en/settings-reference#spellcheck).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spellcheck: Option<Spellcheck>,
    /// Add SSH connections whose host comes from a command to the
    /// [Desktop](https://code.claude.com/docs/en/desktop#create-an-ssh-connection-from-a-command)
    /// environment selector. When you create a connection or create another machine for it,
    /// Desktop runs the entry's command on your computer and connects to the host that the
    /// command prints. Requires Claude Desktop v3.6556.0 or later on macOS or Linux, or
    /// v4.16336.0 or later on Windows, signed in with a Claude account.
    ///
    /// - **Values**: array of objects, each with required `id`, `name`, and `command` and
    ///   optional `sshIdentityFile` and `startDirectory`. Desktop doesn't read `command` as
    ///   shell syntax, so pipes and variables don't work: [write it as a program and its
    ///   arguments](https://code.claude.com/docs/en/desktop#write-the-command).
    /// - **Default**: unset
    /// - **Scope**: user or managed. Only the desktop app reads this key.
    ///
    /// Docs: [`sshHostCommands`](https://code.claude.com/docs/en/settings-reference#sshhostcommands).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host_commands: Option<Vec<SshHostCommand>>,
    /// Choose how long the [prompt cache](https://code.claude.com/docs/en/prompt-caching) holds
    /// the requests Claude Code makes outside the main conversation. This key applies to
    /// [subagents](https://code.claude.com/docs/en/sub-agents),
    /// [workflows](https://code.claude.com/docs/en/workflows), and Claude Code's own background
    /// and helper requests, such as compaction and session titles. The one-hour lifetime keeps
    /// the cache warm across longer breaks, and the API [bills each cache write at a higher
    /// rate](https://platform.claude.com/docs/en/build-with-claude/prompt-caching#pricing) than
    /// at the five-minute lifetime.
    ///
    /// - **Values**: string, one of:
    ///   - `"5m"`: the cache holds for five minutes
    ///   - `"1h"`: the cache holds for an hour
    /// - **Default**: unset, so each of these requests gets [its default
    ///   lifetime](https://code.claude.com/docs/en/prompt-caching#which-ttl-each-request-gets)
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`subagentPromptCacheTtl`](https://code.claude.com/docs/en/settings-reference#subagentpromptcachettl).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_prompt_cache_ttl: Option<CacheTtl>,
    /// Choose what happens when a [safety classifier flags a
    /// request](https://code.claude.com/docs/en/model-config#automatic-model-fallback): switch
    /// to the fallback model and continue, or pause so you can choose between switching and
    /// editing the prompt.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code switches to the fallback model and continues
    ///   - `false`: in an interactive session Claude Code pauses so you can choose between
    ///     switching and editing the prompt; where no dialog can show, such as a `-p` run, the
    ///     flagged request ends as an error
    /// - **Default**: unset. Claude Code switches automatically, though it may [ask
    ///   first](https://code.claude.com/docs/en/model-config#ask-before-switching) in an
    ///   interactive session
    /// - **Scope**: any settings file. Appears in `/config` as **Switch models when a message
    ///   is flagged**, with the options **Switch automatically** and **Ask each time**.
    ///
    /// Docs: [`switchModelsOnFlag`](https://code.claude.com/docs/en/settings-reference#switchmodelsonflag).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub switch_models_on_flag: Option<bool>,
    /// Turn off the download of the [plugins enabled for your claude.ai
    /// account](https://code.claude.com/docs/en/plugins/loading#synced-plugins). Claude Code
    /// downloads them into `~/.claude/plugins/synced/` at the start of terminal sessions where
    /// you sign in with your claude.ai account and in Cowork sessions, and loads each one as
    /// `<name>@synced`. Set `false` to stop that download and stop loading the plugins it
    /// already synced.
    ///
    /// - **Values**: Boolean
    ///   - `false`: Claude Code stops downloading synced plugins and stops loading the ones
    ///     already in `~/.claude/plugins/synced/`. In user or managed settings, it also moves
    ///     them to `~/.claude/plugins/.trash/`
    ///   - `true`: the same as unset
    /// - **Default**: unset, so sessions signed in with your claude.ai account sync your
    ///   plugins
    /// - **Scope**: user, local, or managed, and files passed with `--settings`. A repository
    ///   can't turn it off for you.
    ///
    /// Docs: [`syncClaudeAiPlugins`](https://code.claude.com/docs/en/settings-reference#syncclaudeaiplugins).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_claude_ai_plugins: Option<bool>,
    /// Turn off the download of the [skills enabled for your claude.ai
    /// account](https://code.claude.com/docs/en/skills#how-synced-skills-behave). Claude Code
    /// downloads them into `~/.claude/skills/synced/` in [terminal sessions where you sign in
    /// with your claude.ai
    /// account](https://code.claude.com/docs/en/skills#where-synced-skills-load), interactive
    /// or non-interactive, and in Cowork and cloud sessions. Set `false` to stop that download
    /// and stop loading the skills it already synced.
    ///
    /// - **Values**: Boolean
    ///   - `false`: Claude Code stops downloading synced skills and stops loading the ones
    ///     already in `~/.claude/skills/synced/`. In user or managed settings, it also moves
    ///     them to `~/.claude/skills/.trash/`
    ///   - `true`: the same as unset
    /// - **Default**: unset, so sessions signed in with your claude.ai account sync your skills
    /// - **Scope**: user, local, or managed, and files passed with `--settings`. A repository
    ///   can't turn it off for you.
    ///
    /// Docs: [`syncClaudeAiSkills`](https://code.claude.com/docs/en/settings-reference#syncclaudeaiskills).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_claude_ai_skills: Option<bool>,
    /// **Warning:** Removed in v2.1.277, together with the `TaskOutput` tool it sized. Setting
    /// it has no effect on current versions. Claude reads a background task's [output
    /// file](https://code.claude.com/docs/en/tools-reference#background-commands) with `Read`
    /// instead.
    ///
    /// - **Values**: a whole number of characters
    /// - **Default**: unset
    ///
    /// Docs: [`taskOutputMaxChars`](https://code.claude.com/docs/en/settings-reference#taskoutputmaxchars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_output_max_chars: Option<u64>,
    /// Claude Code sets your terminal tab's title. By default it uses a title it generates from
    /// the conversation, and once you give the session a
    /// [name](https://code.claude.com/docs/en/sessions#name-your-sessions) with `/rename` or
    /// `--name`, the tab shows that name instead. Set this key to `false` to keep the generated
    /// title on the tab even after you name the session.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the terminal tab title shows the session name you set
    ///   - `false`: the tab keeps the title Claude Code generates from your conversation
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`terminalTitleFromRename`](https://code.claude.com/docs/en/settings-reference#terminaltitlefromrename).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_title_from_rename: Option<bool>,
    /// Choose how Claude Code writes the times it shows in the interface, such as the `done
    /// 6:05 PM` at the end of each turn duration message and the timestamps in the [transcript
    /// viewer](https://code.claude.com/docs/en/interactive-mode#transcript-viewer). To pick a
    /// preset, run `/config` and set **Time format**. Requires Claude Code v2.1.257 or later.
    ///
    /// - **Values**: string, one of:
    ///   - `"auto"`: the same as unset; each time keeps its built-in format, which follows your
    ///     locale on the turn duration message
    ///   - `"12-hour"`: a 12-hour clock
    ///   - `"24-hour"`: a 24-hour clock
    ///   - `"24-hour-utc"`: a 24-hour clock in UTC with `Z` after the minutes, such as
    ///     `18:05Z`; Claude Code ignores
    ///     [`timeZone`](https://code.claude.com/docs/en/settings-reference#timezone) for this
    ///     preset
    ///   - A strftime pattern such as `"%H:%M"`: Claude Code writes each time with the pattern.
    ///     Any value that contains a `%` is a pattern, and any other value outside the presets
    ///     counts as `"auto"`
    /// - **Default**: `"auto"`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`timeFormat`](https://code.claude.com/docs/en/settings-reference#timeformat).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_format: Option<TimeFormat>,
    /// Show the times in the interface in a time zone other than your system's. Set it to an
    /// [IANA time zone name](https://www.iana.org/time-zones), such as `"UTC"` or
    /// `"Europe/Dublin"`. The times that
    /// [`timeFormat`](https://code.claude.com/docs/en/settings-reference#timeformat) controls
    /// then show in this zone.
    ///
    /// - **Values**: string, an IANA time zone name. When Claude Code doesn't recognize the
    ///   name, it uses your system time zone
    /// - **Default**: unset, so times show in your system time zone
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`timeZone`](https://code.claude.com/docs/en/settings-reference#timezone).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    /// Start sessions with
    /// [ultracode](https://code.claude.com/docs/en/workflows#let-claude-decide-with-ultracode)
    /// on. With it on, Claude plans a workflow for each substantive task instead of waiting for
    /// you to ask. Claude plans workflows only when [dynamic
    /// workflows](https://code.claude.com/docs/en/workflows) are enabled for you and your model
    /// supports `xhigh` effort.
    ///
    /// - **Values**: Boolean
    ///   - `true`: sessions start with ultracode on when dynamic workflows are enabled for you
    ///     and your model supports `xhigh`
    ///   - `false`: sessions start with ultracode off
    /// - **Default**: unset, so ultracode is off
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`ultracode`](https://code.claude.com/docs/en/settings-reference#ultracode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ultracode: Option<bool>,

    /// Defaults for cloud sessions.
    ///
    /// - **Values**: object with `defaultEnvironmentId`
    /// - **Default**: unset
    ///
    /// Docs: [`remote`](https://code.claude.com/docs/en/settings-reference#remote-defaultenvironmentid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<Remote>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

impl Settings {
    /// Parses a settings document.
    ///
    /// # Errors
    ///
    /// When `json` isn't a JSON object, or a key this crate models holds
    /// a value of the wrong type (which Claude Code rejects as well).
    pub fn from_json_str(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    /// The document as compact JSON: the argument of `claude --settings`.
    ///
    /// # Errors
    ///
    /// Only if a value can't be written as JSON, which the types in this
    /// crate don't produce; the error is returned rather than a panic.
    pub fn to_json_string(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// The document as indented JSON, for a settings file.
    ///
    /// # Errors
    ///
    /// As [`to_json_string`](Self::to_json_string).
    pub fn to_json_string_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Layers `higher` over `self` the way Claude Code combines its
    /// settings sources: call it on the lowest layer with each higher one
    /// in turn (user, then project, then local, then `--settings`, then
    /// managed). [`merge_values`](crate::merge_values) has the rules.
    ///
    /// # Errors
    ///
    /// Only if the merged document doesn't parse as `Settings` again,
    /// which two documents of these types can't produce; `self` is left
    /// unchanged then.
    pub fn merge(&mut self, higher: Settings) -> serde_json::Result<()> {
        let mut merged = serde_json::to_value(&*self)?;
        crate::merge_values(&mut merged, serde_json::to_value(higher)?);
        *self = serde_json::from_value(merged)?;
        Ok(())
    }

    /// The dotted paths of the keys this document holds but this crate
    /// doesn't model (`extra` maps, hook types and hook events it doesn't
    /// know), such as `sandbox.network.newKey` or `hooks.NewEvent`.
    pub fn unknown_keys(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.unknown("", &mut out);
        out
    }
}

/// `null` as `Some(None)`, so a key set to `null` stays apart from an
/// absent one (`None`, from `default`).
#[allow(
    clippy::option_option,
    reason = "absent, null and a model are three states"
)]
fn nullable<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

walk!(Settings {
    attribution: "attribution",
    permissions: "permissions",
    allowed_mcp_servers: "allowedMcpServers",
    denied_mcp_servers: "deniedMcpServers",
    hooks: "hooks",
    allowed_channel_plugins: "allowedChannelPlugins",
    status_line: "statusLine",
    file_suggestion: "fileSuggestion",
    extra_known_marketplaces: "extraKnownMarketplaces",
    strict_known_marketplaces: "strictKnownMarketplaces",
    sandbox: "sandbox",
    spinner_verbs: "spinnerVerbs",
    spinner_tips_override: "spinnerTipsOverride",
    worktree: "worktree",
    plugin_configs: "pluginConfigs",
    blocked_marketplaces: "blockedMarketplaces",
    auto_mode: "autoMode",
    subagent_status_line: "subagentStatusLine",
    ssh_configs: "sshConfigs",
    voice: "voice",
    footer_links_regexes: "footerLinksRegexes",
    policy_helper: "policyHelper",
    model_picker: "modelPicker",
    model_pricing: "modelPricing",
    model_settings: "modelSettings",
    spellcheck: "spellcheck",
    ssh_host_commands: "sshHostCommands",
    remote: "remote",
});
