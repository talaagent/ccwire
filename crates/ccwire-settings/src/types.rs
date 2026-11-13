//! The other objects: attribution, status line, spinner, worktree, SSH,
//! voice, footer links, policy helper, spellcheck, remote.

use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::Extra;
use crate::enums::{BgIsolation, SpellChecker, SpinnerVerbsMode, VoiceMode, WorktreeBaseRef};
use crate::walk::walk;

/// `attribution`: what Claude Code adds to commits and pull requests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribution {
    /// The commit trailer; empty hides it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    /// The pull request line; empty hides it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr: Option<String>,
    /// Add the claude.ai session link (cloud and Remote Control sessions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_url: Option<bool>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `statusLine`: a command that prints the status line from the session
/// JSON on its stdin.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusLine {
    /// `command`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Extra horizontal padding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    /// Also re-run it every this many seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval: Option<u64>,
    /// The command draws the vim mode itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_vim_mode_indicator: Option<bool>,
    /// Keys this crate doesn't model.
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandSetting {
    /// `command`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Keys this crate doesn't model.
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpinnerVerbs {
    /// Add to the built-in verbs or replace them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<SpinnerVerbsMode>,
    /// The verbs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbs: Option<Vec<String>>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `spinnerTipsOverride`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpinnerTips {
    /// Show only these tips.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_default: Option<bool>,
    /// The tips.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tips: Option<Vec<String>>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `worktree`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    /// Check out only these directories (sparse, cone mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_paths: Option<Vec<String>>,
    /// Branch from `origin/<default>` (`fresh`) or `HEAD`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<WorktreeBaseRef>,
    /// Background sessions' isolation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_isolation: Option<BgIsolation>,
    /// Directories to symlink from the main checkout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symlink_directories: Option<Vec<String>>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sshConfigs`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshConfig {
    /// Its identifier across settings sources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Its display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `user@host`, `host` or an `~/.ssh/config` alias.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host: Option<String>,
    /// The port (22 by default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_port: Option<u16>,
    /// The private key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_identity_file: Option<String>,
    /// The remote working directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_directory: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sshHostCommands`: a desktop SSH connection opened by a
/// command.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHostCommand {
    /// Its identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Its display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The program and its arguments (not shell syntax).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// The private key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_identity_file: Option<String>,
    /// The remote working directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_directory: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `voice`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voice {
    /// Voice dictation on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Hold to talk, or tap to start and stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<VoiceMode>,
    /// Hold mode: submit on release (three words or more).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_submit: Option<bool>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `footerLinksRegexes`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FooterLink {
    /// `regex`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The regex matched against turn output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// The link; `{name}` takes a named capture group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The badge text; `{name}` as in `url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `policyHelper`: a program that computes managed settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyHelper {
    /// The program (an absolute path).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// How long to wait for it, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// How often to re-run it, in milliseconds (0: never, else at least
    /// 60 000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval_ms: Option<u64>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `spellcheck`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Spellcheck {
    /// Spellcheck on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The checker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checker: Option<SpellChecker>,
    /// The dictionary (`en_GB`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// The underline color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `remote`: cloud sessions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    /// The environment new cloud sessions use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_environment_id: Option<String>,
    /// Keys this crate doesn't model.
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
walk!(SpinnerTips {});
walk!(Worktree {});
walk!(SshConfig {});
walk!(SshHostCommand {});
walk!(Voice {});
walk!(FooterLink {});
walk!(PolicyHelper {});
walk!(Spellcheck {});
walk!(Remote {});
