//! Claude Code's `settings.json` as Rust types.
//!
//! [Claude Code](https://code.claude.com/docs) reads its configuration from
//! layered JSON settings files and from the `--settings` command-line
//! argument. This crate types every key those documents accept, so a
//! program can build one, write it for `claude --settings`, read settings
//! files, and layer them the way Claude Code does, without hand-built JSON.
//!
//! - [`Settings`]: the whole document. Every key Claude Code's published
//!   JSON schema and its [settings reference] list is a field, with the
//!   nested objects typed: [`Permissions`], [`Hooks`], [`Sandbox`],
//!   marketplaces, plugin configuration, model settings and the rest. Each
//!   field's docs say what Claude Code does with it, the values it accepts,
//!   its default and where it may be set, with a link to its entry in the
//!   settings reference.
//! - [`Settings::to_json_string`]: the document for `--settings`;
//!   [`Settings::to_json_string_pretty`] for a file.
//! - [`Settings::merge`] and [`merge_values`]: Claude Code's layer merge.
//! - [`Settings::unknown_keys`]: the keys a document holds that this crate
//!   doesn't model.
//!
//! The crate depends on serde and serde_json only, and does no I/O. It is
//! not affiliated with or endorsed by Anthropic.
//!
//! [settings reference]: https://code.claude.com/docs/en/settings-reference
//!
//! # Build a `--settings` document
//!
//! Every type is `#[non_exhaustive]`, so that Claude Code's new keys can be
//! added without a breaking release: start from `Default::default()` and
//! set fields. Only what is set is written.
//!
//! ```
//! use claude_code_settings::{Hook, HookEvent, HookMatcher, PermissionMode, Permissions, Settings};
//!
//! let mut permissions = Permissions::default();
//! permissions.allow = Some(vec!["Bash(git diff *)".into(), "Read".into()]);
//! permissions.deny = Some(vec!["Bash(rm *)".into()]);
//! permissions.default_mode = Some(PermissionMode::DontAsk);
//!
//! let mut settings = Settings::default();
//! settings.permissions = Some(permissions);
//! settings.include_git_instructions = Some(false);
//! settings.hooks.get_or_insert_default().insert(
//!     HookEvent::PreToolUse,
//!     vec![HookMatcher::matching("Bash", [Hook::command("./scripts/check-command")])],
//! );
//!
//! let arg = settings.to_json_string()?;
//! assert!(arg.contains(r#""defaultMode":"dontAsk""#));
//! // std::process::Command::new("claude").args(["--settings", &arg, "-p", "..."]);
//! # Ok::<(), serde_json::Error>(())
//! ```
//!
//! # Read a settings file, keeping what this crate doesn't know
//!
//! A key, an enum value, a hook type or a hook event this crate doesn't
//! model is kept as written (in an `extra` map, an `Other` variant or
//! [`Hook::Other`]) and written back unchanged, so a file from a newer
//! Claude Code survives a parse, a change and a write.
//!
//! ```
//! use claude_code_settings::{PermissionMode, Settings};
//!
//! let file = r#"{
//!     "model": "opus",
//!     "permissions": {"defaultMode": "someFutureMode", "allow": ["Read"]},
//!     "aKeyFromANewerVersion": {"enabled": true}
//! }"#;
//! let mut settings = Settings::from_json_str(file)?;
//! assert_eq!(
//!     settings.permissions.as_ref().and_then(|p| p.default_mode.clone()),
//!     Some(PermissionMode::Other("someFutureMode".into())),
//! );
//! assert_eq!(settings.unknown_keys(), ["aKeyFromANewerVersion"]);
//!
//! settings.model = Some("sonnet".into());
//! let written = settings.to_json_string_pretty()?;
//! assert!(written.contains("aKeyFromANewerVersion"));
//! assert!(written.contains("someFutureMode"));
//! # Ok::<(), serde_json::Error>(())
//! ```
//!
//! # Layer settings
//!
//! Claude Code reads settings from several sources. Highest precedence
//! first:
//!
//! 1. **Managed settings**: what an organization deploys
//!    (`managed-settings.json`, MDM, or server-managed settings). Nothing
//!    below overrides a managed key.
//! 2. **Command line**: `--settings <file-or-json>`, for one session.
//! 3. **Local project settings**: `.claude/settings.local.json`.
//! 4. **Shared project settings**: `.claude/settings.json`.
//! 5. **User settings**: `~/.claude/settings.json`.
//!
//! Claude Code merges them rather than picking one:
//!
//! - a scalar (a string, number or Boolean) comes from the highest source
//!   that sets it;
//! - objects merge key by key, so `env`, `permissions` and `sandbox`
//!   combine across sources;
//! - lists concatenate, so `permissions.allow`, `ask` and `deny`, and each
//!   hook event's matcher groups, add up; `fallbackModel` and
//!   `modelPicker` are exceptions, taken whole from the highest source.
//!
//! Merging decides what each key holds, not how rules are applied: a
//! `deny` rule from any source beats an `allow` rule from any other
//! (Claude Code evaluates `deny`, then `ask`, then `allow`). Some keys are
//! honored only from some sources (managed-only keys, keys project settings
//! can't set), which each field's **Scope** line says and the merge does
//! not enforce. Claude Code also watches its settings files and reloads
//! most keys in a running session when a file changes; a few, such as
//! `model`, are read only at session start.
//!
//! [`Settings::merge`] applies one layer over another; call it on the
//! lowest layer with each higher one in turn:
//!
//! ```
//! use claude_code_settings::Settings;
//!
//! let user = Settings::from_json_str(r#"{
//!     "model": "opus",
//!     "env": {"EDITOR": "vim"},
//!     "permissions": {"allow": ["Read"]}
//! }"#)?;
//! let project = Settings::from_json_str(r#"{
//!     "model": "sonnet",
//!     "env": {"RUST_LOG": "info"},
//!     "permissions": {"allow": ["Bash(cargo test *)"], "deny": ["Bash(rm *)"]}
//! }"#)?;
//!
//! let mut effective = user;
//! effective.merge(project)?;
//! assert_eq!(effective.model.as_deref(), Some("sonnet"));
//! assert_eq!(effective.env.as_ref().map(|e| e.len()), Some(2));
//! let permissions = effective.permissions.unwrap_or_default();
//! assert_eq!(permissions.allow.unwrap_or_default(), ["Read", "Bash(cargo test *)"]);
//! assert_eq!(permissions.deny.unwrap_or_default(), ["Bash(rm *)"]);
//! # Ok::<(), serde_json::Error>(())
//! ```
//!
//! # Compatibility
//!
//! The types track [`CLAUDE_CODE_VERSION`] and the JSON schema fetched on
//! [`SCHEMA_FETCHED`]. A fully populated document is checked against that
//! Claude Code release: it loads with no key rejected. New keys and values
//! are additive: the types are `#[non_exhaustive]`, and an older release of
//! this crate keeps a newer key in `extra` rather than dropping it.
//!
//! # Keeping up with Claude Code
//!
//! The repository keeps a copy of the published schema
//! (<https://json.schemastore.org/claude-code-settings.json>) next to the
//! crate. `scripts/refresh-settings-schema` fetches it again, and the
//! crate's `schema` test then lists, by JSON path, every property and every
//! enum value the new schema has that the types don't. An opt-in test hands
//! a fully populated document to a real `claude --settings` and fails on any
//! key it rejects.
//!
//! # License
//!
//! Licensed under the Apache License, Version 2.0.

#![deny(missing_docs)]
#![allow(
    clippy::doc_markdown,
    reason = "the docs quote Claude Code's settings reference, product names and all"
)]
#![deny(rustdoc::broken_intra_doc_links)]

extern crate alloc;

mod enums;
mod hooks;
mod merge;
mod models;
mod permissions;
mod plugins;
mod sandbox;
mod settings;
mod types;
mod walk;

pub use enums::{
    BgIsolation, CacheTtl, CredentialMode, CustomizationKind, DialogTimeout, DiffTool, DisableFlag,
    DisabledFlag, EditorMode, EffortLevel, ExtractNoMatch, FeedbackDrafts, HookEvent,
    HookOnFailure, InboundMode, KeybindingFlavor, LoginMethod, ModelMatch, NotifChannel,
    PermissionMode, Platform, Provider, Shell, Sigv4Action, SkillOverride, SourceKind,
    SourcesBehavior, SpellChecker, SpinnerVerbsMode, TeammateMode, Theme, TimeFormat, TokenDecode,
    Tui, UpdatesChannel, ViewMode, VoiceMode, WorkflowSize, WorktreeBaseRef,
};
pub use hooks::{
    AgentHook, CommandHook, Hook, HookMatcher, Hooks, HttpHook, McpToolHook, PromptHook,
};
pub use merge::merge_values;
pub use models::{
    CompactWindow, ModelPicker, ModelPickerOption, ModelPricing, ModelRates, ModelSettings,
};
pub use permissions::{AutoMode, McpServerRule, Permissions};
pub use plugins::{
    ChannelPlugin, ChannelPluginEntry, InlinePlugin, Marketplace, OptionValue, PluginConfig,
    PluginEnabled, Source, StrictCustomization,
};
pub use sandbox::{
    AwsPair, CredentialEnvVar, CredentialFile, Ripgrep, Sandbox, SandboxCredentials,
    SandboxFilesystem, SandboxNetwork, Sigv4, TlsTerminate,
};
pub use settings::Settings;
pub use types::{
    Attribution, AttributionSetting, CommandSetting, FooterLink, ManagedMcpServers, OneOrMany,
    PolicyHelper, Remote, Spellcheck, SpinnerTip, SpinnerTips, SpinnerVerbs, SshConfig,
    SshHostCommand, StatusLine, TipEntry, Voice, Worktree,
};

/// The README's examples, compiled as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// The Claude Code release these types were last checked against.
pub const CLAUDE_CODE_VERSION: &str = "2.1.322";

/// The date (UTC) the copy of the settings JSON schema these types follow
/// was fetched.
pub const SCHEMA_FETCHED: &str = "2026-11-13";

/// The keys an object holds that this crate doesn't model, by name.
pub type Extra = serde_json::Map<alloc::string::String, serde_json::Value>;
