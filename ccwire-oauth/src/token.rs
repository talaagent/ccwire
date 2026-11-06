//! The refresh grant's types.

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::time::Duration;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::Secret;

/// Claude Code's public OAuth client id, the one its logins are issued to.
/// A refresh must name the client the token was issued to.
pub const CLAUDE_CODE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";

/// The scopes a refresh asks for when the caller names none: Claude Code's
/// defaults for a subscription login.
pub const DEFAULT_SCOPES: &[&str] = &[
    "user:profile",
    "user:inference",
    "user:sessions:claude_code",
    "user:mcp_servers",
    "user:file_upload",
    "user:plugins",
];

/// What to refresh: a refresh token, plus the client and scopes to post
/// with it.
///
/// ```
/// use ccwire_oauth::{RefreshRequest, Secret};
///
/// let req = RefreshRequest::new(Secret::new("stored-refresh-token"))
///     .scopes(["user:profile", "user:inference"]);
/// assert_eq!(req.scope_string(), "user:profile user:inference");
/// ```
#[derive(Debug, Clone)]
pub struct RefreshRequest {
    /// The refresh token.
    pub refresh_token: Secret,
    /// The OAuth client id. Default [`CLAUDE_CODE_CLIENT_ID`].
    pub client_id: String,
    /// The scopes to ask for. Default [`DEFAULT_SCOPES`]; pass the stored
    /// login's scopes when it has them.
    pub scopes: Vec<String>,
}

impl RefreshRequest {
    /// A request for `refresh_token` with the default client and scopes.
    pub fn new(refresh_token: Secret) -> Self {
        Self {
            refresh_token,
            client_id: CLAUDE_CODE_CLIENT_ID.to_owned(),
            scopes: DEFAULT_SCOPES.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    /// Sets the client id.
    #[must_use]
    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
    }

    /// Sets the scopes. An empty list keeps the defaults.
    #[must_use]
    pub fn scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let scopes: Vec<String> = scopes.into_iter().map(Into::into).collect();
        if !scopes.is_empty() {
            self.scopes = scopes;
        }
        self
    }

    /// The scopes as the token endpoint takes them: space-separated.
    pub fn scope_string(&self) -> String {
        self.scopes.join(" ")
    }
}

/// The JSON body posted to the token endpoint.
#[derive(Serialize)]
pub(crate) struct GrantBody<'a> {
    pub grant_type: &'static str,
    pub refresh_token: &'a str,
    pub client_id: &'a str,
    pub scope: String,
}

/// The token endpoint's success response, as sent.
#[derive(Deserialize)]
pub(crate) struct GrantResponse {
    pub access_token: Secret,
    #[serde(default)]
    pub refresh_token: Option<Secret>,
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token_expires_in: Option<u64>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
}

/// A refreshed login: what to store in place of the old tokens.
///
/// The refresh token rotates: the one posted is spent. Store
/// [`TokenSet::refresh_token`] only if the stored one is still the one you
/// posted (compare-and-swap); if another process already stored a newer
/// pair, keep that one instead.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TokenSet {
    /// The new access token.
    pub access_token: Secret,
    /// The refresh token to store: the new one, or the posted one if the
    /// response didn't rotate it (see [`TokenSet::rotated`]).
    pub refresh_token: Secret,
    /// Whether the response carried a new refresh token.
    pub rotated: bool,
    /// The access token's lifetime, as the response gave it.
    pub expires_in: Duration,
    /// When the access token expires: the response's time plus
    /// [`TokenSet::expires_in`].
    pub expires_at: SystemTime,
    /// When the new refresh token expires, if the response said.
    pub refresh_token_expires_at: Option<SystemTime>,
    /// The granted scopes.
    pub scopes: Vec<String>,
    /// The token type (`Bearer`), if the response said.
    pub token_type: Option<String>,
}

impl TokenSet {
    pub(crate) fn from_response(r: GrantResponse, posted: &Secret, now: SystemTime) -> Self {
        let expires_in = Duration::from_secs(r.expires_in);
        let (refresh_token, rotated) = match r.refresh_token {
            Some(t) if !t.is_empty() => (t, true),
            _ => (posted.clone(), false),
        };
        Self {
            access_token: r.access_token,
            refresh_token,
            rotated,
            expires_in,
            expires_at: now + expires_in,
            refresh_token_expires_at: r
                .refresh_token_expires_in
                .map(|s| now + Duration::from_secs(s)),
            scopes: r
                .scope
                .unwrap_or_default()
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect(),
            token_type: r.token_type,
        }
    }
}
