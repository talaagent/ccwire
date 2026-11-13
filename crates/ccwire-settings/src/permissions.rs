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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    /// Rules for tool uses to allow without asking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// Rules for tool uses to deny.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Vec<String>>,
    /// Rules for tool uses to always ask about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<Vec<String>>,
    /// The permission mode a session starts in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<PermissionMode>,
    /// `disable` turns off `bypassPermissions` mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_bypass_permissions_mode: Option<DisableFlag>,
    /// `disable` turns off auto mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_auto_mode: Option<DisableFlag>,
    /// More directories the session may work in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_directories: Option<Vec<String>>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `autoMode`: the auto-mode classifier's sections. Each list replaces
/// the built-in one unless it holds the literal `$defaults`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoMode {
    /// Rules for the allow section.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// Rules for the soft-deny section.
    #[serde(rename = "soft_deny", skip_serializing_if = "Option::is_none")]
    pub soft_deny: Option<Vec<String>>,
    /// Entries for the environment section.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<String>>,
    /// Rules for the hard-deny section, which block unconditionally.
    #[serde(rename = "hard_deny", skip_serializing_if = "Option::is_none")]
    pub hard_deny: Option<Vec<String>>,
    /// Route every shell command through the classifier (undocumented).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classify_all_shell: Option<bool>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `allowedMcpServers` or `deniedMcpServers`: one of the
/// three fields.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerRule {
    /// A server by name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    /// A stdio server by its exact command line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_command: Option<Vec<String>>,
    /// A remote server by URL (wildcards allowed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

walk!(Permissions {});
walk!(AutoMode {});
walk!(McpServerRule {});
