//! Plugins and marketplaces.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};
use serde_json::Number;

use crate::Extra;
use crate::enums::{CustomizationKind, SourceKind};
use crate::walk::{Unknown, walk};

/// A value of `enabledPlugins`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum PluginEnabled {
    /// On or off.
    Enabled(bool),
    /// On, at these versions.
    Versions(Vec<String>),
}

/// An entry of `extraKnownMarketplaces`.
///
/// Register additional plugin marketplaces by name, so that people who open the repository, or
/// everyone your managed settings reach, get the marketplace without adding it themselves.
/// Claude Code registers each marketplace it doesn't already know.
///
/// Docs: [`extraKnownMarketplaces`](https://code.claude.com/docs/en/settings-reference#extraknownmarketplaces).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Marketplace {
    /// Where to fetch the marketplace from.
    ///
    /// - **Values**: a marketplace source object: `source` names the kind (`github`, `git`,
    ///   `url`, `npm`, `file`, `directory`, `hostPattern`, `settings`), and the other fields
    ///   are the ones that kind takes
    /// - **Default**: unset
    ///
    /// Docs: [`source`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Local cache path where marketplace manifest is stored (auto-generated if not provided).
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`installLocation`](https://code.claude.com/docs/en/settings-reference#extraknownmarketplaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_location: Option<String>,
    /// Whether to automatically update this marketplace on Claude Code startup. Written
    /// automatically by Claude Code when you toggle auto-update for a marketplace.
    ///
    /// - **Values**: Boolean
    /// - **Default**: unset
    ///
    /// Docs: [`autoUpdate`](https://code.claude.com/docs/en/settings-reference#extraknownmarketplaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
    /// ISO 8601 timestamp of the last marketplace refresh. Written automatically by Claude
    /// Code.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`lastUpdated`](https://code.claude.com/docs/en/settings-reference#extraknownmarketplaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A marketplace or plugin source: `source` names the kind, and the
/// other fields are the ones that kind takes (`github`: `repo`, `ref`,
/// `path`; `npm`: `package`; and so on). One struct for every kind, so a
/// kind this crate doesn't know still parses.
///
/// Used by `extraKnownMarketplaces`, `strictKnownMarketplaces`,
/// `blockedMarketplaces` and an inline marketplace's plugins.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Source {
    /// Identifies the marketplace source type.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`source`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceKind>,
    /// Direct URL to marketplace.json file.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`url`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// GitHub repository in owner/repo format.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`repo`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// Git branch or tag to use (e.g., "main", "v1.0.0"). Defaults to repository default
    /// branch.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`ref`](None).
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// A plugin source's exact commit SHA, which pins the plugin.
    ///
    /// - **Values**: string, a full commit SHA
    /// - **Default**: unset
    ///
    /// Docs: [`sha`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// Path to marketplace.json within repo (defaults to .claude-plugin/marketplace.json).
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`path`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Skip Git LFS downloads when Claude Code clones or updates the marketplace repository.
    /// LFS pointer files remain as pointers instead of downloading their content. Use this when
    /// the repository contains large LFS objects unrelated to plugin content.
    ///
    /// - **Values**: Boolean
    /// - **Default**: unset
    ///
    /// Docs: [`skipLfs`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_lfs: Option<bool>,
    /// NPM package containing marketplace.json.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`package`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// An `npm` plugin source's package version.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`version`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// An `npm` plugin source's registry URL.
    ///
    /// - **Values**: string, a URL
    /// - **Default**: unset
    ///
    /// Docs: [`registry`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// A `url` source's extra request headers, for authenticated access.
    ///
    /// - **Values**: object mapping header names to values
    /// - **Default**: unset
    ///
    /// Docs: [`headers`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    /// A `url` source's command that prints headers whose values are too short-lived to list in
    /// `headers`. Claude Code runs it before each fetch of the marketplace and each plugin
    /// download from its origin, reusing its output for up to 60 seconds. Requires Claude Code
    /// v2.1.238 or later.
    ///
    /// - **Values**: string, a command
    /// - **Default**: unset
    ///
    /// Docs: [`headersHelper`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers_helper: Option<String>,
    /// Git host pattern to trust for repositories in source specifications.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`hostPattern`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_pattern: Option<String>,
    /// A `pathPattern` source (in `strictKnownMarketplaces` and `blockedMarketplaces`): a
    /// regular expression matched against local marketplace paths.
    ///
    /// - **Values**: string, a regular expression
    /// - **Default**: unset
    ///
    /// Docs: [`pathPattern`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_pattern: Option<String>,
    /// Name of the inline marketplace.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`name`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Plugins declared inline. Each must reference an external source such as GitHub or npm.
    /// You still need to enable each plugin separately in enabledPlugins.
    ///
    /// - **Values**: array of object
    /// - **Default**: unset
    ///
    /// Docs: [`plugins`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<InlinePlugin>>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

impl Source {
    /// A source of kind `kind` with no other field set.
    pub fn new(kind: SourceKind) -> Self {
        Self {
            source: Some(kind),
            ..Self::default()
        }
    }
}

/// A plugin of an inline (`settings`) marketplace.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct InlinePlugin {
    /// Plugin name.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`name`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// External source the plugin is fetched from. Inline plugins must reference external
    /// sources such as GitHub or npm, not relative paths.
    ///
    /// - **Values**: a plugin source object: `github`, `url`, `git-subdir` or `npm`
    /// - **Default**: unset
    ///
    /// Docs: [`source`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `allowedChannelPlugins`.
///
/// Choose which [channel](https://code.claude.com/docs/en/channels) plugins can push messages
/// into sessions in your organization. When you set it, Claude Code uses your list in place of
/// the default Anthropic allowlist; each entry names a plugin and the marketplace it comes
/// from.
///
/// Docs: [`allowedChannelPlugins`](https://code.claude.com/docs/en/settings-reference#allowedchannelplugins).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ChannelPlugin {
    /// Name of the marketplace the channel plugin is installed from.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`marketplace`](https://code.claude.com/docs/en/settings-reference#allowedchannelplugins).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace: Option<String>,
    /// Name of the channel plugin allowed to run.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`plugin`](https://code.claude.com/docs/en/settings-reference#allowedchannelplugins).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `allowedChannelPlugins`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum ChannelPluginEntry {
    /// `plugin@marketplace`.
    Id(String),
    /// The object form.
    Plugin(ChannelPlugin),
}

impl Unknown for ChannelPluginEntry {
    fn unknown(&self, path: &str, out: &mut Vec<String>) {
        if let Self::Plugin(p) = self {
            p.unknown(path, out);
        }
    }
}

/// A value of `pluginConfigs`: a plugin's user configuration.
///
/// Store the non-sensitive answers you give a plugin's
/// [`userConfig`](https://code.claude.com/docs/en/plugins/manifest-reference#user-configuration)
/// configuration dialog, keyed by plugin ID. Claude Code writes this key to your user settings
/// when you fill in the dialog, so you don't need to edit it by hand. Claude Code stores
/// sensitive options in the macOS Keychain instead, falling back to
/// `~/.claude/.credentials.json` when the Keychain rejects the write; on platforms without a
/// supported keychain, it stores them in `~/.claude/.credentials.json`.
///
/// Docs: [`pluginConfigs`](https://code.claude.com/docs/en/settings-reference#pluginconfigs).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PluginConfig {
    /// User configuration values for MCP servers keyed by server name.
    ///
    /// - **Values**: object
    /// - **Default**: unset
    ///
    /// Docs: [`mcpServers`](https://code.claude.com/docs/en/settings-reference#pluginconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<BTreeMap<String, BTreeMap<String, OptionValue>>>,
    /// Non-sensitive option values from the plugin manifest's userConfig, keyed by option name.
    /// Sensitive values go to secure storage instead.
    ///
    /// - **Values**: object
    /// - **Default**: unset
    ///
    /// Docs: [`options`](https://code.claude.com/docs/en/settings-reference#pluginconfigs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<BTreeMap<String, OptionValue>>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A plugin option's value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum OptionValue {
    /// A boolean.
    Bool(bool),
    /// A number.
    Number(Number),
    /// A string.
    String(String),
    /// A list of strings.
    List(Vec<String>),
}

/// `strictPluginOnlyCustomization`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum StrictCustomization {
    /// Every kind (`true`) or none (`false`).
    All(bool),
    /// These kinds only.
    Only(Vec<CustomizationKind>),
}

walk!(Marketplace { source: "source" });
walk!(Source { plugins: "plugins" });
walk!(InlinePlugin { source: "source" });
walk!(ChannelPlugin {});
walk!(PluginConfig {});
