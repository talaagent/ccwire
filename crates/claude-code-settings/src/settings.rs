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

/// Claude Code's settings.json: a settings file, a `--settings`
/// document, or the merge of several.
///
/// Every key is optional and left out when `None`; keys this crate
/// doesn't model (a newer Claude Code's, or undocumented ones) are kept
/// in [`extra`](Self::extra) and written back, so a document round-trips.
/// Field docs are short: the settings reference
/// (<https://code.claude.com/docs/en/settings-reference>) has each key.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Settings {
    /// JSON Schema reference for Claude Code settings.
    #[serde(skip_serializing_if = "Option::is_none", rename = "$schema")]
    pub schema: Option<String>,
    /// Generate the API credential with your own command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_helper: Option<String>,
    /// Turn auto memory off or on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_memory_enabled: Option<bool>,
    /// Follow the stable release channel instead of latest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_updates_channel: Option<UpdatesChannel>,
    /// Supply Bedrock credentials as JSON from your own command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_credential_export: Option<String>,
    /// Refresh expired Bedrock credentials in .aws with your own command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_auth_refresh: Option<String>,
    /// Skip specific CLAUDE.md files when memory loads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_md_excludes: Option<Vec<String>>,
    /// Choose how many days Claude Code keeps transcripts before deleting them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_period_days: Option<u64>,
    /// Set environment variables for every session and its subprocesses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<BTreeMap<String, String>>,
    /// Customize the attribution Claude Code adds to commits and pull requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<AttributionSetting>,
    /// Remove the built-in commit and PR instructions from Claude's context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_git_instructions: Option<bool>,
    /// Deprecated; use attribution to hide or change commit and PR attribution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_co_authored_by: Option<bool>,
    /// Choose where plan mode writes plan files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plans_directory: Option<String>,
    /// Keep gitignored files out of the @ file picker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respect_gitignore: Option<bool>,
    /// Set allow, ask, and deny rules and the default permission mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// Have Claude respond in a language other than English.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Change the model Claude Code starts with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Restrict which models people can pick.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_models: Option<Vec<String>>,
    /// Map model IDs to your provider's IDs, such as Bedrock ARNs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_overrides: Option<BTreeMap<String, String>>,
    /// Set a default effort level for models without a saved level of
    /// their own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_level: Option<EffortLevel>,
    /// Turn fast mode on for sessions where it's available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode: Option<bool>,
    /// Require people to turn fast mode on each session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fast_mode_per_session_opt_in: Option<bool>,
    /// Change how often the session quality survey appears.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_survey_rate: Option<f64>,
    /// Approve every server in project .mcp.json files without a prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_all_project_mcp_servers: Option<bool>,
    /// Approve specific servers from a project's .mcp.json.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_mcpjson_servers: Option<Vec<String>>,
    /// Reject specific servers from a project's .mcp.json.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_mcpjson_servers: Option<Vec<String>>,
    /// Allowlist which MCP servers users can add.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mcp_servers: Option<Vec<McpServerRule>>,
    /// Block specific MCP servers by URL, command, or name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_mcp_servers: Option<Vec<McpServerRule>>,
    /// Limit which env vars HTTP hooks can put in headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_hook_allowed_env_vars: Option<Vec<String>>,
    /// Run your own commands as hooks at points in Claude Code's lifecycle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Hooks>,
    /// Turn off hooks, a custom status line, and a custom @ file
    /// suggestion command at once.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_all_hooks: Option<bool>,
    /// Replace the default allowlist of channel plugins that can push messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_channel_plugins: Option<Vec<ChannelPluginEntry>>,
    /// Limit which URLs HTTP hooks can target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_http_hook_urls: Option<Vec<String>>,
    /// Run only the hooks your organization deploys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_hooks_only: Option<bool>,
    /// Make managed settings the only settings source of permission rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_permission_rules_only: Option<bool>,
    /// Run your own command to render a status line below the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_line: Option<StatusLine>,
    /// Supply @ file autocomplete from your own command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_suggestion: Option<CommandSetting>,
    /// Turn individual plugins on or off per scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_plugins: Option<BTreeMap<String, PluginEnabled>>,
    /// Register marketplaces for a repository or an organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_known_marketplaces: Option<BTreeMap<String, Marketplace>>,
    /// Allowlist the marketplace sources users can add and install from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_known_marketplaces: Option<Vec<Source>>,
    /// List of marketplace names the user has chosen not to install
    /// when prompted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_marketplaces: Option<Vec<String>>,
    /// List of plugin IDs (plugin@marketplace format) the user has
    /// chosen not to install when prompted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped_plugins: Option<Vec<String>>,
    /// Restrict login to claude.ai, Claude Console, or a cloud gateway.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_login_method: Option<LoginMethod>,
    /// Pin claude.ai logins to your organization; only managed settings
    /// enforce it.
    #[serde(skip_serializing_if = "Option::is_none", rename = "forceLoginOrgUUID")]
    pub force_login_org_uuid: Option<OneOrMany>,
    /// Generate rotating OpenTelemetry headers with your own command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otel_headers_helper: Option<String>,
    /// Change Claude's role, tone, and output format with an output style.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_style: Option<String>,
    /// Skip the `WebFetch` hostname check when Anthropic is unreachable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_web_fetch_preflight: Option<bool>,
    /// Isolate Bash commands from your filesystem and network on macOS,
    /// Linux, and WSL2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<Sandbox>,
    /// Add or replace the verbs shown while a turn runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinner_verbs: Option<SpinnerVerbs>,
    /// Hide tips in the spinner while Claude works.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinner_tips_enabled: Option<bool>,
    /// Add your own tips to the spinner rotation, or replace the built-in tips.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spinner_tips_override: Option<SpinnerTips>,
    /// Hide the terminal progress bar in terminals that support it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_progress_bar_enabled: Option<bool>,
    /// Hide the "Cooked for" duration after each response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_turn_duration: Option<bool>,
    /// Hide or collapse a skill without editing its SKILL.md.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_overrides: Option<BTreeMap<String, SkillOverride>>,
    /// Reduce or turn off spinner, shimmer, and flash animations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefers_reduced_motion: Option<bool>,
    /// Point PR links at an internal code-review tool instead of github.com.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_url_template: Option<String>,
    /// Turn extended thinking off for every session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_thinking_enabled: Option<bool>,
    /// Show your organization's announcements at startup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_announcements: Option<Vec<String>>,
    /// Choose how agent team teammates display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teammate_mode: Option<TeammateMode>,
    /// Configure how Claude Code creates git worktrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<Worktree>,
    /// Apply or drop restrictions an SDK or IDE host passes when you
    /// deploy managed settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_settings_behavior: Option<SourcesBehavior>,
    /// Add your own text to the plugin trust warning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_trust_message: Option<String>,
    /// Store the answers you gave a plugin's configuration dialog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_configs: Option<BTreeMap<String, PluginConfig>>,
    /// Make the managed MCP allowlist the only one that applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_mcp_servers_only: Option<bool>,
    /// Block plugin marketplace sources for your organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_marketplaces: Option<Vec<Source>>,
    /// Start every session as a named subagent with its prompt, tools,
    /// and model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// Store auto memory in a directory you choose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_memory_directory: Option<String>,
    /// Add your own allow and deny rules to the auto mode classifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_mode: Option<AutoMode>,
    /// Allow channels for your organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels_enabled: Option<bool>,
    /// Choose whether Bash or PowerShell runs the shell commands you
    /// type with the ! prefix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_shell: Option<Shell>,
    /// Stop Claude Code from registering the claude-cli:// handler.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_deep_link_registration: Option<DisableFlag>,
    /// Stop skills and custom commands from running inline shell.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_skill_shell_execution: Option<bool>,
    /// Block startup until server-managed settings are freshly fetched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_remote_settings_refresh: Option<bool>,
    /// Keep auto-updates from installing anything below a version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_version: Option<String>,
    /// Show a "clear context" option on the plan accept screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_clear_context_on_plan_accept: Option<bool>,
    /// See summaries of Claude's thinking instead of a collapsed stub.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_thinking_summaries: Option<bool>,
    /// Skip the confirmation dialog before bypassPermissions mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_dangerous_mode_permission_prompt: Option<bool>,
    /// Block skills, agents, hooks, and MCP servers from user and
    /// project sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_plugin_only_customization: Option<StrictCustomization>,
    /// Choose the fullscreen or classic terminal renderer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tui: Option<Tui>,
    /// Start every session in default, verbose, or focus view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_mode: Option<ViewMode>,
    /// Let the auto mode classifier review shell commands in plan mode;
    /// set false to get prompts instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_auto_mode_during_plan: Option<bool>,
    /// Turn on voice dictation with the older single-key form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_enabled: Option<bool>,
    /// Have WSL read managed settings from the Windows policy chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsl_inherits_windows_settings: Option<bool>,
    /// Status line configuration for subagent sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_status_line: Option<CommandSetting>,
    /// Pick which model answers when Claude asks the advisor tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advisor_model: Option<String>,
    /// Turn automatic compaction off or on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_enabled: Option<bool>,
    /// Turn off background agents and agent view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_agent_view: Option<bool>,
    /// Use vim key bindings in the input prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_mode: Option<EditorMode>,
    /// Keep the /model Default choice inside your availableModels allowlist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_available_models: Option<bool>,
    /// Name backup models for when the primary is overloaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_model: Option<Vec<String>>,
    /// Turn off or on the file snapshots that /rewind restores.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_checkpointing_enabled: Option<bool>,
    /// Refresh Google Cloud credentials with your own command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp_auth_refresh: Option<String>,
    /// Provide remote MCP servers to every user alongside the ones they add.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_mcp_servers: Option<ManagedMcpServers>,
    /// Cap each skill's description length in the skill listing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_listing_max_desc_chars: Option<u64>,
    /// Choose a terminal bell or desktop notification for task completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_notif_channel: Option<NotifChannel>,
    /// Choose which marketplaces can surface plugin install suggestions
    /// in /plugin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_suggestion_marketplaces: Option<Vec<String>>,
    /// Refuse to start on a version newer than your organization allows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_maximum_version: Option<String>,
    /// Refuse to start on a version older than your organization requires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_minimum_version: Option<String>,
    /// Stop Claude from responding after a ! shell command runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub respond_to_bash_commands: Option<bool>,
    /// Reserve more or less context for the skill listing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_listing_budget_fraction: Option<f64>,
    /// Add SSH connections to the Desktop environment selector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_configs: Option<Vec<SshConfig>>,
    /// Limit which hosts Desktop SSH sessions can reach.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host_allowlist: Option<Vec<String>>,
    /// Pick the interface color theme, built-in or custom.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// Turn on voice dictation and pick hold or tap mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,
    /// Turn off mouse-wheel acceleration in fullscreen rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wheel_scroll_acceleration_enabled: Option<bool>,
    /// Turn off the skills and workflows included with Claude Code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_bundled_skills: Option<bool>,
    /// Turn off the session recap shown when you come back to the terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub away_summary_enabled: Option<bool>,
    /// Follow new output to the bottom in fullscreen rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_scroll_enabled: Option<bool>,
    /// Load the claude.ai connectors Claude Code fetches itself
    /// alongside a deployed managed-mcp.json.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_claude_ai_mcps: Option<bool>,
    /// Let Claude send a push notification to your phone when it decides to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_push_notif_enabled: Option<bool>,
    /// Render screen-reader friendly output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ax_screen_reader: Option<bool>,
    /// Inject organization-wide CLAUDE.md instructions from managed settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_md: Option<String>,
    /// Deprecated; use enableArtifact to turn the Artifact tool off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_artifact: Option<bool>,
    /// Remove auto mode from the permission mode cycle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_auto_mode: Option<DisableFlag>,
    /// Turn off claude.ai connectors so Claude Code doesn't fetch them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_claude_ai_connectors: Option<bool>,
    /// Turn off Remote Control everywhere it can start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_remote_control: Option<bool>,
    /// Turn dynamic workflows off for everyone; use enableWorkflows for
    /// yourself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_workflows: Option<bool>,
    /// Make issue or review IDs in output into clickable links below
    /// the input box.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_links_regexes: Option<Vec<FooterLink>>,
    /// Get a push notification when Claude is waiting on you.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_needed_notif_enabled: Option<bool>,
    /// Run an executable that computes managed settings at startup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_helper: Option<PolicyHelper>,
    /// Connect Remote Control automatically when a session starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_control_at_startup: Option<bool>,
    /// Turn off syntax highlighting in diffs and code blocks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syntax_highlighting_disabled: Option<bool>,
    /// Show full tool output instead of truncated summaries; viewMode
    /// takes precedence when both are set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    /// Let the word ultracode in a prompt start a workflow; set false
    /// to type it without starting one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_keyword_trigger_enabled: Option<bool>,
    /// Let an unanswered question auto-continue after idle time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_user_question_timeout: Option<DialogTimeout>,
    /// The default model for agent-team teammates; `null` follows the
    /// lead's. Removed in v2.1.234 (still in the schema).
    #[serde(
        default,
        deserialize_with = "nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub teammate_default_model: Option<Option<String>>,
    /// Set the agent count Claude aims for in dynamic workflows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_size_guideline: Option<WorkflowSize>,
    /// Turn the Artifact tool off with a false in any file; no file can
    /// turn it back on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_artifact: Option<bool>,
    /// The Ctrl+E command explanation on shell permission prompts.
    /// Removed in v2.1.257 (still in the schema).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_explainer_enabled: Option<bool>,
    /// Choose whether Claude's proposed file changes open in the VS
    /// Code or `JetBrains` diff viewer or stay in the terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_tool: Option<DiffTool>,
    /// Show Claude's last response as comments when you press Ctrl+G to edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_editor_context: Option<bool>,
    /// Connect to a running VS Code or `JetBrains` IDE automatically
    /// from an external terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_connect_ide: Option<bool>,
    /// Let /ide install the IDE extension from an external terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_install_ide_extension: Option<bool>,
    /// Turn off :shortcode: emoji suggestions and replacement in the
    /// prompt input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_completion_enabled: Option<bool>,
    /// Map a two-key INSERT-mode sequence such as jj to Escape.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vim_insert_mode_remaps: Option<BTreeMap<String, String>>,
    /// Run Claude Code's background processes through a corporate
    /// launcher on macOS and Linux.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_wrapper: Option<String>,
    /// Set the gateway URL the login screen connects to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_login_gateway_url: Option<String>,
    /// Reject the CLI flags that sideload plugins, subagents, and MCP servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_sideload_flags: Option<bool>,
    /// Keep Claude's tools off external pages in the desktop Browser pane.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_external_page_tools: Option<DisabledFlag>,
    /// Limit the desktop Browser pane to localhost for people and Claude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_browser_external_navigation: Option<bool>,
    /// Block Claude's tools in the desktop iOS Simulator pane.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_mobile_simulator_tools: Option<bool>,
    /// (Managed settings only) Run Claude's tools inside an isolated
    /// virtual machine instead of on your Mac.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_cowork_full_vm_sandbox: Option<bool>,
    /// Let the built-in Claude in Chrome server run alongside a
    /// deployed managed-mcp.json.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_claude_in_chrome_with_managed_mcp: Option<bool>,
    /// Limit which API providers a machine may use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_providers: Option<Vec<Provider>>,
    /// Run your organization's mods after every mod a user installs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append_plugins: Option<Vec<String>>,
    /// Set how full the context gets before Claude Code compacts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_window: Option<u64>,
    /// Wait in the open session and continue the task automatically
    /// after a claude.ai usage limit resets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_continue_at_usage_limit: Option<bool>,
    /// Make each availableModels model ID entry permit only the version
    /// it names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_models_match: Option<ModelMatch>,
    /// Record the files that changed while a Bash command ran in every
    /// permission mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bash_edit_diff_enabled: Option<bool>,
    /// Set how much of a successful command's output Claude receives inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bash_output_max_chars: Option<u64>,
    /// Choose whether Claude Code delivers messages from your other
    /// sessions, shows a notice without delivering them, or refuses
    /// them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross_session_inbound: Option<InboundMode>,
    /// Block specific models, even ones availableModels permits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_models: Option<Vec<String>>,
    /// Set an age limit in days for Claude Desktop and Cowork transcripts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop_session_cleanup_period_days: Option<u64>,
    /// Set how long Claude Code waits for Remote Control or an SDK host
    /// to answer a forwarded dialog before it cancels the dialog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialog_expiry: Option<DialogTimeout>,
    /// Block plugins that install by running a marketplace-declared command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_command_plugin_sources: Option<bool>,
    /// Turn off Desktop Code sessions that run on the device, leaving
    /// SSH to other hosts and cloud.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_desktop_local_sessions: Option<bool>,
    /// Turn off scripts in the desktop app's HTML previews in SSH and
    /// WSL sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_remote_preview_scripts: Option<bool>,
    /// Turn dynamic workflows on or off against your plan's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_workflows: Option<bool>,
    /// Control whether Claude queues feedback drafts for you to review.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback_drafts: Option<FeedbackDrafts>,
    /// Let /login reach a cloud gateway on public IPv4 space your
    /// organization uses internally.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_internal_networks: Option<Vec<String>>,
    /// Stop Claude Code from compacting a long conversation that you've
    /// stepped away from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_compaction: Option<bool>,
    /// Ask you before Claude messages one of your sessions on another machine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolate_peer_machines: Option<bool>,
    /// Deprecated and has no effect; the word-editing shortcuts always
    /// follow readline conventions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keybinding_flavor: Option<KeybindingFlavor>,
    /// Compose every managed source you deploy instead of using the
    /// highest-priority one alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed_sources_behavior: Option<SourcesBehavior>,
    /// Cap the effort level for every model or per model, on every provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_effort_level: Option<EffortLevel>,
    /// Cap how wide the prose in Claude's responses runs in a wide terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_prose_width: Option<u64>,
    /// Choose which models the /model picker lists, in your own order
    /// and with your own labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_picker: Option<ModelPicker>,
    /// Report spend at your organization's contracted rates instead of
    /// list price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_pricing: Option<ModelPricing>,
    /// Keep a saved effort level or auto-compact window per model, or
    /// cap one model's effort.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_settings: Option<BTreeMap<String, ModelSettings>>,
    /// Run your organization's mods before every mod a user installs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepend_plugins: Option<Vec<String>>,
    /// Choose the prompt cache lifetime for the main conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_ttl: Option<CacheTtl>,
    /// Hide the grayed-out prompt suggestions in the input box.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_suggestion_enabled: Option<bool>,
    /// Skip the one-time notice Claude Code shows when you first enter
    /// auto mode yourself rather than through the built-in default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_auto_permission_prompt: Option<bool>,
    /// Underline misspelled words in the prompt input with a spell
    /// checker you install.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spellcheck: Option<Spellcheck>,
    /// Add SSH connections whose host comes from a command to the
    /// Desktop environment selector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host_commands: Option<Vec<SshHostCommand>>,
    /// Choose the prompt cache lifetime for subagents and other
    /// requests outside the main conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_prompt_cache_ttl: Option<CacheTtl>,
    /// Switch models automatically or pause when a safety classifier
    /// flags a request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub switch_models_on_flag: Option<bool>,
    /// Stop loading the plugins enabled on your claude.ai account and
    /// stop downloading new ones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_claude_ai_plugins: Option<bool>,
    /// Stop loading the skills enabled on your claude.ai account and
    /// stop downloading new ones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_claude_ai_skills: Option<bool>,
    /// The `TaskOutput` tool's output size. Removed in v2.1.277 (still
    /// documented).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_output_max_chars: Option<u64>,
    /// Stop /rename and --name from changing the terminal tab title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_title_from_rename: Option<bool>,
    /// Show the times in the interface on a 12-hour or 24-hour clock,
    /// in UTC, or with a strftime pattern.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_format: Option<TimeFormat>,
    /// Show the times in the interface in a time zone other than your system's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    /// Have Claude plan a workflow for each substantive task without
    /// being asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ultracode: Option<bool>,

    /// Cloud sessions' defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<Remote>,
    /// Keys this crate doesn't model.
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
