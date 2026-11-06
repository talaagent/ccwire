//! The response types.
//!
//! Every field the API is known to send is typed, and nearly all are
//! `Option`: the endpoints are undocumented, so a missing field reads as
//! `None` instead of failing. Whatever else a response carries lands in the
//! struct's `extra` map.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::Deserialize;

/// Fields a response carried that this crate doesn't type, by name.
pub type Extra = BTreeMap<String, serde_json::Value>;

/// `GET /api/oauth/usage`: the subscription's rate-limit windows and its
/// extra (paid) usage.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Usage {
    /// The rolling 5-hour window.
    #[serde(default)]
    pub five_hour: Option<Window>,
    /// The 7-day window over all models.
    #[serde(default)]
    pub seven_day: Option<Window>,
    /// The 7-day window for Opus models, on plans that have one.
    #[serde(default)]
    pub seven_day_opus: Option<Window>,
    /// The 7-day window for Sonnet models, on plans that have one.
    #[serde(default)]
    pub seven_day_sonnet: Option<Window>,
    /// The 7-day window for third-party OAuth apps.
    #[serde(default)]
    pub seven_day_oauth_apps: Option<Window>,
    /// Extra usage (usage credits billed past the plan), if the account
    /// has it.
    #[serde(default)]
    pub extra_usage: Option<ExtraUsage>,
    /// The server's list of meters, when it sends one (newer responses).
    #[serde(default)]
    pub limits: Vec<Limit>,
    /// Anything else (more windows as the plans grow them).
    #[serde(flatten)]
    pub extra: Extra,
}

/// One rate-limit window.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Window {
    /// Percent of the window used, `0.0` to `100.0` (it can pass 100).
    #[serde(default)]
    pub utilization: Option<f64>,
    /// When the window resets, RFC 3339. `None` for an idle window that
    /// hasn't started.
    #[serde(default)]
    pub resets_at: Option<String>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Extra usage: credits spent past the plan's limits this month.
///
/// Amounts are in the currency's minor units (cents).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct ExtraUsage {
    /// Whether extra usage is turned on.
    #[serde(default)]
    pub is_enabled: Option<bool>,
    /// The monthly limit, minor units.
    #[serde(default)]
    pub monthly_limit: Option<f64>,
    /// Spent this month, minor units.
    #[serde(default)]
    pub used_credits: Option<f64>,
    /// Percent of the monthly limit spent.
    #[serde(default)]
    pub utilization: Option<f64>,
    /// ISO 4217 code (`USD`).
    #[serde(default)]
    pub currency: Option<String>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// One row of [`Usage::limits`].
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Limit {
    /// The meter's kind (`session`, `weekly_all`, `weekly_scoped`, ...).
    #[serde(default)]
    pub kind: Option<String>,
    /// The row's group (`session`, `weekly`, ...).
    #[serde(default)]
    pub group: Option<String>,
    /// Percent used.
    #[serde(default)]
    pub utilization: Option<f64>,
    /// When it resets, RFC 3339.
    #[serde(default)]
    pub resets_at: Option<String>,
    /// Whether this meter is the one currently limiting.
    #[serde(default)]
    pub is_active: Option<bool>,
    /// What the meter covers, for scoped meters.
    #[serde(default)]
    pub scope: Option<LimitScope>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// What a scoped [`Limit`] covers.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct LimitScope {
    /// The model family, for a per-model meter.
    #[serde(default)]
    pub model: Option<LimitModel>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The model of a per-model [`Limit`].
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct LimitModel {
    /// Its display name, as the server labels it.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `GET /api/oauth/profile`: the account and its organization.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Profile {
    /// The user.
    pub account: Account,
    /// The organization the token acts for.
    #[serde(default)]
    pub organization: Option<Organization>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

impl Profile {
    /// The consumer subscription, from the account's flags. `None` for an
    /// account with neither (a Team or Enterprise seat, or no plan): see
    /// [`Organization::organization_type`] for those.
    pub fn plan(&self) -> Option<Plan> {
        if self.account.has_claude_max == Some(true) {
            Some(Plan::Max)
        } else if self.account.has_claude_pro == Some(true) {
            Some(Plan::Pro)
        } else {
            None
        }
    }
}

/// A consumer subscription, as [`Profile::plan`] reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Plan {
    /// Claude Pro.
    Pro,
    /// Claude Max.
    Max,
}

/// The user behind a token.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Account {
    /// The account's id.
    pub uuid: String,
    /// The login email.
    #[serde(default, alias = "email_address")]
    pub email: Option<String>,
    /// The full name.
    #[serde(default)]
    pub full_name: Option<String>,
    /// The display name.
    #[serde(default)]
    pub display_name: Option<String>,
    /// Whether the account has Claude Max.
    #[serde(default)]
    pub has_claude_max: Option<bool>,
    /// Whether the account has Claude Pro.
    #[serde(default)]
    pub has_claude_pro: Option<bool>,
    /// When the account was created, RFC 3339.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// The organization a token acts for.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Organization {
    /// The organization's id: the `org` argument of the org-scoped calls.
    pub uuid: String,
    /// Its name.
    #[serde(default)]
    pub name: Option<String>,
    /// Its type (`claude_pro`, `claude_max`, `claude_team`, ...).
    #[serde(default)]
    pub organization_type: Option<String>,
    /// How it is billed.
    #[serde(default)]
    pub billing_type: Option<String>,
    /// Its rate-limit tier.
    #[serde(default)]
    pub rate_limit_tier: Option<String>,
    /// Whether extra usage is turned on for it.
    #[serde(default)]
    pub has_extra_usage_enabled: Option<bool>,
    /// The subscription's status (`active`, ...).
    #[serde(default)]
    pub subscription_status: Option<String>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `GET /api/oauth/claude_cli/roles`: the user's roles in the token's
/// organization.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Roles {
    /// The organization role (`admin`, `user`, ...).
    #[serde(default)]
    pub organization_role: Option<String>,
    /// The workspace role, where workspaces apply.
    #[serde(default)]
    pub workspace_role: Option<String>,
    /// The organization's name.
    #[serde(default)]
    pub organization_name: Option<String>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `GET /api/oauth/organizations/{org}/prepaid/credits`: the prepaid
/// balance.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct PrepaidCredits {
    /// The balance, minor units. `None` where prepaid credits don't apply.
    #[serde(default)]
    pub amount: Option<f64>,
    /// ISO 4217 code.
    #[serde(default)]
    pub currency: Option<String>,
    /// The auto-reload settings.
    #[serde(default)]
    pub auto_reload_settings: Option<AutoReload>,
    /// After how many months unused credits expire, if they do.
    #[serde(default)]
    pub expiry_policy_months: Option<u32>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}

/// Prepaid auto-reload, read-only.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AutoReload {
    /// Whether it is on.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Anything else (thresholds).
    #[serde(flatten)]
    pub extra: Extra,
}

/// `GET /api/oauth/organizations/{org}/overage_spend_limit`: the extra
/// usage (overage) monthly limit.
///
/// Amounts are in minor units.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct OverageSpendLimit {
    /// Whether overage is turned on.
    #[serde(default)]
    pub is_enabled: Option<bool>,
    /// The monthly limit, minor units.
    #[serde(default)]
    pub monthly_credit_limit: Option<f64>,
    /// ISO 4217 code.
    #[serde(default)]
    pub currency: Option<String>,
    /// Spent this month, minor units.
    #[serde(default)]
    pub used_credits: Option<f64>,
    /// Until when overage is off, RFC 3339, if it was turned off.
    #[serde(default)]
    pub disabled_until: Option<String>,
    /// Why it is off, if it is.
    #[serde(default)]
    pub disabled_reason: Option<String>,
    /// Anything else.
    #[serde(flatten)]
    pub extra: Extra,
}
