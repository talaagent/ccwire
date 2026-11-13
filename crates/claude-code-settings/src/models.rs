//! `modelPicker`, `modelPricing` and `modelSettings`.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::Extra;
use crate::enums::EffortLevel;
use crate::walk::walk;

/// `modelPicker`: the `/model` picker's rows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelPicker {
    /// The rows, in order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ModelPickerOption>>,
    /// Show only these rows instead of adding them to the built-in ones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace_built_in_options: Option<bool>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A row of `modelPicker.options`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelPickerOption {
    /// The model, as `--model` takes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The row's title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Its second line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A known model whose capabilities a newer `model` takes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behaves_as: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `modelPricing`: contracted rates (managed settings).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelPricing {
    /// Scales every cost (below 1 a discount, up to 10).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<f64>,
    /// Model ID → its rates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<BTreeMap<String, ModelRates>>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A model's rates, in USD per million tokens.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelRates {
    /// Input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<f64>,
    /// Output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<f64>,
    /// Cache reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,
    /// Cache writes (five-minute and one-hour).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// A value of `modelSettings`: one model's saved settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ModelSettings {
    /// Its effort level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_level: Option<EffortLevel>,
    /// Its effort cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_effort_level: Option<EffortLevel>,
    /// Its auto-compact window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_compact_window: Option<CompactWindow>,
    /// Keys this crate doesn't model.
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
