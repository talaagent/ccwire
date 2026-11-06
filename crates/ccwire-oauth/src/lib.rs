//! A typed async client for the Claude account endpoints an OAuth login can
//! call: usage, profile, token validation, roles, prepaid credits and the
//! overage limit, and the refresh-token grant.
//!
//! Everything is read-only. Nothing here spends money, changes a setting
//! or mints a key; the only `POST`s are the token check
//! ([`Client::validate`]) and the refresh grant ([`Client::refresh`]).
//!
//! This crate is not affiliated with or endorsed by Anthropic. The
//! endpoints are undocumented: they are the ones Claude Code itself calls.
//! Their shapes can change without notice, so nearly every field is an
//! `Option` that reads as `None` when it is missing or of an unexpected
//! type, and each type keeps whatever else the response carried in its
//! `extra` map.
//!
//! - [`Client`]: one method per endpoint. It holds no token: each call
//!   takes the one it acts with.
//! - [`Secret`]: a token. It never prints and its own buffer is zeroed on
//!   drop (copies made along the way may not be: see [`Secret`]).
//! - [`Error`]: [`Error::Unauthorized`] (re-login), [`Error::Forbidden`]
//!   (a good token without the scope or role),
//!   [`Error::RateLimited`] (with `Retry-After`), [`Error::Http`],
//!   [`Error::Decode`] (with a scrubbed body snippet), and the rest.
//!
//! ```no_run
//! # async fn run() -> Result<(), ccwire_oauth::Error> {
//! use ccwire_oauth::{Client, RefreshRequest, Secret};
//!
//! let client = Client::new()?;
//! let stored = Secret::new("stored-refresh-token");
//! // Don't cancel a refresh once it's sent (see `Client::refresh`).
//! let tokens = client.refresh(&RefreshRequest::new(stored)).await?;
//! // Store tokens.refresh_token only if the stored one is still the one
//! // posted (compare-and-swap); it has been spent either way.
//! let profile = client.profile(&tokens.access_token).await?;
//! if let Some(org) = &profile.organization {
//!     let limit = client.overage_spend_limit(&tokens.access_token, &org.uuid).await?;
//!     println!("{:?} {:?}", limit.is_enabled, limit.used_credits);
//! }
//! # Ok(()) }
//! ```
//!
//! # License
//!
//! Licensed under the Apache License, Version 2.0 (`LICENSE`).

#![warn(missing_docs)]

extern crate alloc;

mod client;
mod error;
mod lenient;
mod secret;
mod token;
mod types;

pub use client::{
    Client, ClientBuilder, DEFAULT_BASE_URL, DEFAULT_TIMEOUT, DEFAULT_TOKEN_URL,
    DEFAULT_USER_AGENT, MAX_BODY, OAUTH_BETA,
};
pub use error::Error;
pub use secret::Secret;
pub use token::{CLAUDE_CODE_CLIENT_ID, DEFAULT_SCOPES, RefreshRequest, TokenSet};
pub use types::{
    Account, AutoReload, Extra, ExtraUsage, Limit, LimitModel, LimitScope, Organization,
    OverageSpendLimit, Plan, PrepaidCredits, Profile, Roles, Usage, Window,
};

/// The README's example, compiled as a doctest.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
