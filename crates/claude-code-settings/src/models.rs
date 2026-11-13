//! `modelPicker`, `modelPricing` and `modelSettings`.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::Extra;
use crate::enums::EffortLevel;
use crate::walk::walk;

/// `modelPicker`: the `/model` picker's rows.
///
/// List the models the `/model` picker offers, in the order you write them and under labels you
/// choose, so the picker lists the models your organization runs, after the built-in lineup or
/// instead of it. Each row's `model` is taken verbatim, so it accepts anything `--model`
/// accepts: an alias such as `opus`, an Anthropic model ID, or a provider-format ID for Amazon
/// Bedrock, Google Cloud's Agent Platform, Microsoft Foundry, or an LLM gateway.
///
/// Docs: [`modelPicker`](https://code.claude.com/docs/en/settings-reference#modelpicker).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelPicker {
    /// The rows the picker shows, in this order, except that a grayed-out row moves to the
    /// bottom. Without a `label`, Claude Code titles the row with the built-in name for a model
    /// it knows, or the model ID otherwise, and without a `description` it writes a generic
    /// second line.
    ///
    /// - **Values**: array of rows, each with a required `model` and optional `label`,
    ///   `description`, and `behavesAs`
    /// - **Default**: unset
    ///
    /// Docs: [`options`](https://code.claude.com/docs/en/settings-reference#fields-for-modelpicker).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ModelPickerOption>>,
    /// Set it to `true` to show only these rows, **Default**, and a row for the model the
    /// session is already using. Leave it unset to add these rows after the built-in lineup.
    ///
    /// - **Values**: Boolean, default `false`
    /// - **Default**: unset
    ///
    /// Docs: [`replaceBuiltInOptions`](https://code.claude.com/docs/en/settings-reference#fields-for-modelpicker).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_built_in_options: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A row of `modelPicker.options`.
///
/// The rows the picker shows, in this order, except that a grayed-out row moves to the bottom.
/// Without a `label`, Claude Code titles the row with the built-in name for a model it knows,
/// or the model ID otherwise, and without a `description` it writes a generic second line.
///
/// Docs: [`modelPicker.options`](https://code.claude.com/docs/en/settings-reference#fields-for-modelpicker).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelPickerOption {
    /// The model the row selects, taken verbatim: anything `--model` accepts, such as an alias,
    /// an Anthropic model ID, or a provider-format ID.
    ///
    /// - **Values**: string (required)
    /// - **Default**: unset
    ///
    /// Docs: [`model`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The row's title. Without it, Claude Code uses the built-in name for a model it knows, or
    /// the model ID.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`label`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The row's second line. Without it, Claude Code writes a generic one.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`description`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The ID of a model this Claude Code version knows, such as `claude-opus-4-8`, whose
    /// capabilities and effort defaults apply to a newer `model`. Requires Claude Code v2.1.257
    /// or later.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`behavesAs`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behaves_as: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `modelPricing`: contracted rates (managed settings).
///
/// Report spend at the rates your organization pays instead of list price. Set it when your
/// organization has contracted rates, so the dollar figures developers see match your bill.
///
/// Docs: [`modelPricing`](https://code.claude.com/docs/en/settings-reference#modelpricing).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelPricing {
    /// Scales every cost Claude Code computes, whether or not an `overrides` row covers it.
    /// Below 1 is a discount, above 1 a markup.
    ///
    /// - **Values**: number greater than 0 and at most 10
    /// - **Default**: unset
    ///
    /// Docs: [`multiplier`](https://code.claude.com/docs/en/settings-reference#fields-for-modelpricing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<f64>,
    /// The USD-per-million-token rates for that model, all four required. `cacheWrite` covers
    /// both five-minute and one-hour cache writes. See [Which models a row applies
    /// to](https://code.claude.com/docs/en/settings-reference#which-models-a-modelpricing-row-applies-to).
    ///
    /// - **Values**: map of model ID to a rate object with `input`, `output`, `cacheRead`, and
    ///   `cacheWrite`, each 0 to 10000
    /// - **Default**: unset
    ///
    /// Docs: [`overrides`](https://code.claude.com/docs/en/settings-reference#fields-for-modelpricing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<BTreeMap<String, ModelRates>>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A model's rates, in USD per million tokens.
///
/// The USD-per-million-token rates for that model, all four required. `cacheWrite` covers both
/// five-minute and one-hour cache writes.
///
/// Docs: [`modelPricing.overrides`](https://code.claude.com/docs/en/settings-reference#fields-for-modelpricing).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelRates {
    /// The rate for input tokens.
    ///
    /// - **Values**: USD per million tokens, `0` to `10000`; all four rates are required
    /// - **Default**: unset
    ///
    /// Docs: [`input`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    /// The rate for output tokens.
    ///
    /// - **Values**: USD per million tokens, `0` to `10000`; all four rates are required
    /// - **Default**: unset
    ///
    /// Docs: [`output`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<f64>,
    /// The rate for cache reads.
    ///
    /// - **Values**: USD per million tokens, `0` to `10000`; all four rates are required
    /// - **Default**: unset
    ///
    /// Docs: [`cacheRead`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    /// The rate for cache writes, five-minute and one-hour alike.
    ///
    /// - **Values**: USD per million tokens, `0` to `10000`; all four rates are required
    /// - **Default**: unset
    ///
    /// Docs: [`cacheWrite`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A value of `modelSettings`: one model's saved settings.
///
/// Save an [effort level](https://code.claude.com/docs/en/model-config#adjust-effort-level) for
/// each model you use. Requires Claude Code v2.1.251 or later.
///
/// Docs: [`modelSettings`](https://code.claude.com/docs/en/settings-reference#modelsettings).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelSettings {
    /// This model's effort level. It takes precedence over the top-level `effortLevel` in the
    /// same file.
    ///
    /// - **Values**: one of `"low"`, `"medium"`, `"high"`, or `"xhigh"`
    /// - **Default**: unset
    ///
    /// Docs: [`effortLevel`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_level: Option<EffortLevel>,
    /// The highest effort level this model may run at. It replaces the top-level
    /// `maxEffortLevel` for this model. Requires Claude Code v2.1.267 or later.
    ///
    /// - **Values**: one of `"low"`, `"medium"`, `"high"`, `"xhigh"`, or `"max"` (no cap)
    /// - **Default**: unset
    ///
    /// Docs: [`maxEffortLevel`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_effort_level: Option<EffortLevel>,
    /// This model's auto-compact window. It takes precedence over a top-level
    /// `autoCompactWindow` in the same file; `/autocompact` saves here. Requires Claude Code
    /// v2.1.288 or later.
    ///
    /// - **Values**: a number of tokens from `100000` to `1000000`, or `"auto"` for the window
    ///   tuned for the model
    /// - **Default**: unset
    ///
    /// Docs: [`autoCompactWindow`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_window: Option<CompactWindow>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `modelSettings.*.autoCompactWindow`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum CompactWindow {
    /// A number of tokens (100 000 to 1 000 000).
    Tokens(u64),
    /// `auto`: the window tuned for the model.
    Named(String),
}

walk!(ModelPicker { options: "options" });
walk!(ModelPickerOption {});
walk!(ModelPricing {
    overrides: "overrides"
});
walk!(ModelRates {});
walk!(ModelSettings {});
