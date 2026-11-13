//! Claude Code's settings.json as Rust types.
//!
//! [`Settings`] has every key Claude Code's settings accept: the ones in
//! the published JSON schema (a copy is in the repository, with the test
//! that keeps the two in step) and the ones its settings reference
//! documents. Build one, and hand it to `claude --settings` with
//! [`Settings::to_json_string`]; or read a settings file, layer several
//! with [`Settings::merge`], and write the result.
//!
//! - Every key is an `Option`, left out when `None`.
//! - Every object keeps the keys this crate doesn't model in its `extra`
//!   map and writes them back, so a file from a newer Claude Code
//!   round-trips. [`Settings::unknown_keys`] lists them.
//! - Every enum has an `Other(String)` for a value this crate doesn't
//!   know; hooks of an unknown `type` are [`Hook::Other`].
//! - `env` is a plain map: its names are environment variables, not
//!   settings.
//!
//! ```
//! use claude_code_settings::{Hook, HookEvent, HookMatcher, Permissions, PermissionMode, Settings};
//!
//! let mut settings = Settings {
//!     model: Some("sonnet".into()),
//!     permissions: Some(Permissions {
//!         allow: Some(vec!["Bash(git diff:*)".into()]),
//!         default_mode: Some(PermissionMode::DontAsk),
//!         ..Permissions::default()
//!     }),
//!     ..Settings::default()
//! };
//! settings.hooks.get_or_insert_default().insert(
//!     HookEvent::Stop,
//!     vec![HookMatcher::all([Hook::command("notify-send done")])],
//! );
//! let json = settings.to_json_string();
//! assert!(json.contains(r#""defaultMode":"dontAsk""#));
//!
//! // A project file over the user's: lists and hooks combine, scalars
//! // are the project's.
//! let mut user = Settings::from_json_str(r#"{"model":"opus","permissions":{"allow":["Read"]}}"#)?;
//! user.merge(settings);
//! assert_eq!(user.model.as_deref(), Some("sonnet"));
//! assert_eq!(user.permissions.unwrap().allow.unwrap(), ["Read", "Bash(git diff:*)"]);
//! # Ok::<(), serde_json::Error>(())
//! ```
//!
//! This crate is not affiliated with or endorsed by Anthropic.
//!
//! # License
//!
//! Licensed under the Apache License, Version 2.0 (`LICENSE`).

#![warn(missing_docs)]
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
    DisabledFlag, EditorMode, EffortLevel, FeedbackDrafts, HookEvent, InboundMode,
    KeybindingFlavor, LoginMethod, ModelMatch, NotifChannel, PermissionMode, Platform, Provider,
    Shell, SkillOverride, SourceKind, SourcesBehavior, SpellChecker, SpinnerVerbsMode,
    TeammateMode, Theme, TimeFormat, Tui, UpdatesChannel, ViewMode, VoiceMode, WorkflowSize,
    WorktreeBaseRef,
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
    ChannelPlugin, InlinePlugin, Marketplace, OptionValue, PluginConfig, PluginEnabled, Source,
    StrictCustomization,
};
pub use sandbox::{
    AwsPair, CredentialEnvVar, CredentialFile, Ripgrep, Sandbox, SandboxCredentials,
    SandboxFilesystem, SandboxNetwork, TlsTerminate,
};
pub use settings::Settings;
pub use types::{
    Attribution, CommandSetting, FooterLink, OneOrMany, PolicyHelper, Remote, Spellcheck,
    SpinnerTips, SpinnerVerbs, SshConfig, SshHostCommand, StatusLine, Voice, Worktree,
};

/// The README's examples, compiled as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// The keys an object holds that this crate doesn't model, by name.
pub type Extra = serde_json::Map<alloc::string::String, serde_json::Value>;
