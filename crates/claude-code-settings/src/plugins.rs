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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Marketplace {
    /// Where to fetch it from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Where its manifest is cached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_location: Option<String>,
    /// Update it on startup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
    /// When it was last refreshed (ISO 8601; Claude Code writes it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    /// Keys this crate doesn't model.
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
    /// The kind of source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceKind>,
    /// `url`, `git`, `git-subdir`: the URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// `github`: `owner/repo`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// `github`, `git`, `git-subdir`: a branch or tag.
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// A plugin's exact commit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    /// `github`, `git`: the manifest in the repository; `git-subdir`: the
    /// subdirectory; `file`, `directory`: the local path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `github`, `git`: skip Git LFS downloads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_lfs: Option<bool>,
    /// `npm`: the package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// `npm` (a plugin): the version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// `npm` (a plugin): the registry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    /// `url`: request headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    /// `url`: a command that prints request headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers_helper: Option<String>,
    /// `hostPattern`: the git hosts to trust.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_pattern: Option<String>,
    /// `pathPattern`: the paths to trust.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_pattern: Option<String>,
    /// `settings`: the inline marketplace's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `settings`: its plugins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<InlinePlugin>>,
    /// Keys this crate doesn't model.
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
    /// Its name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Where it is fetched from (`github`, `url`, `git-subdir`, `npm`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `allowedChannelPlugins`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ChannelPlugin {
    /// The marketplace it is installed from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marketplace: Option<String>,
    /// The plugin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    /// Keys this crate doesn't model.
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PluginConfig {
    /// Values for its MCP servers: server → option → value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<BTreeMap<String, BTreeMap<String, OptionValue>>>,
    /// Its non-sensitive options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<BTreeMap<String, OptionValue>>,
    /// Keys this crate doesn't model.
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
