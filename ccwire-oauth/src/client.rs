//! [`Client`] and [`ClientBuilder`].

use alloc::string::String;
use core::time::Duration;
use std::time::SystemTime;

use reqwest::header::{self, HeaderValue};
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use url::{Host, Url};

use crate::error::{error_fields, scrub_serde, snippet};
use crate::token::{GrantBody, GrantResponse};
use crate::{
    Error, OverageSpendLimit, PrepaidCredits, Profile, RefreshRequest, Roles, Secret, TokenSet,
    Usage,
};

/// The API's base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
/// The OAuth token endpoint.
pub const DEFAULT_TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
/// The `anthropic-beta` value the OAuth endpoints require.
pub const OAUTH_BETA: &str = "oauth-2025-04-20";
/// The default request timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
/// The default `User-Agent`: `ccwire-oauth/<version>`.
pub const DEFAULT_USER_AGENT: &str = concat!("ccwire-oauth/", env!("CARGO_PKG_VERSION"));
/// The largest response body read; a larger one is [`Error::TooLarge`].
pub const MAX_BODY: usize = 1 << 20;

/// A client for the account endpoints.
///
/// Every call takes the token it acts with; the client holds none. It is
/// cheap to clone (the connection pool is shared).
///
/// ```no_run
/// # async fn run() -> Result<(), ccwire_oauth::Error> {
/// use ccwire_oauth::{Client, Secret};
///
/// let client = Client::new()?;
/// let token = Secret::new(std::env::var("ACCESS_TOKEN").unwrap_or_default());
/// let usage = client.usage(&token).await?;
/// if let Some(w) = usage.five_hour {
///     println!("5h: {:?}% (resets {:?})", w.utilization, w.resets_at);
/// }
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: Url,
    token_url: Url,
}

/// Builds a [`Client`].
///
/// URLs must be `https`, except on a loopback host (a local mock server).
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    base_url: String,
    token_url: String,
    user_agent: String,
    timeout: Duration,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.into(),
            token_url: DEFAULT_TOKEN_URL.into(),
            user_agent: DEFAULT_USER_AGENT.into(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl ClientBuilder {
    /// The API's base URL. Default [`DEFAULT_BASE_URL`].
    #[must_use]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// The token endpoint's URL. Default [`DEFAULT_TOKEN_URL`].
    #[must_use]
    pub fn token_url(mut self, url: impl Into<String>) -> Self {
        self.token_url = url.into();
        self
    }

    /// The `User-Agent`. Default [`DEFAULT_USER_AGENT`].
    #[must_use]
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// The whole-request timeout. Default [`DEFAULT_TIMEOUT`].
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// The client.
    ///
    /// # Errors
    /// [`Error::Config`]: a URL that doesn't parse or isn't `https` (or
    /// loopback), or a `User-Agent` that isn't a valid header value.
    /// [`Error::Transport`]: the TLS backend couldn't start.
    pub fn build(self) -> Result<Client, Error> {
        let base_url = parse_url(&self.base_url, "base URL")?;
        let token_url = parse_url(&self.token_url, "token URL")?;
        let user_agent = HeaderValue::from_str(&self.user_agent)
            .map_err(|_| Error::Config("user agent is not a valid header value".into()))?;
        let http = reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(self.timeout)
            // Never follow a redirect with a bearer token on it.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| Error::Transport(e.without_url()))?;
        Ok(Client {
            http,
            base_url,
            token_url,
        })
    }
}

fn parse_url(s: &str, what: &str) -> Result<Url, Error> {
    let url = Url::parse(s).map_err(|e| Error::Config(alloc::format!("{what}: {e}")))?;
    let loopback = match url.host() {
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        Some(Host::Domain(d)) => d == "localhost",
        None => false,
    };
    match url.scheme() {
        "https" if url.host().is_some() => Ok(url),
        "http" if loopback => Ok(url),
        _ => Err(Error::Config(alloc::format!(
            "{what} must be https (or http on a loopback host)"
        ))),
    }
}

impl Client {
    /// A client with the defaults.
    ///
    /// # Errors
    /// See [`ClientBuilder::build`].
    pub fn new() -> Result<Self, Error> {
        Self::builder().build()
    }

    /// A [`ClientBuilder`] with the defaults.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// `GET /api/oauth/usage`: the 5-hour and 7-day windows and extra
    /// usage.
    ///
    /// # Errors
    /// [`Error`]; a token without the profile scope is
    /// [`Error::Unauthorized`].
    pub async fn usage(&self, token: &Secret) -> Result<Usage, Error> {
        self.get_json(token, &["api", "oauth", "usage"]).await
    }

    /// `GET /api/oauth/profile`: the account and its organization.
    ///
    /// # Errors
    /// [`Error`]; an inference-only token is [`Error::Unauthorized`].
    pub async fn profile(&self, token: &Secret) -> Result<Profile, Error> {
        self.get_json(token, &["api", "oauth", "profile"]).await
    }

    /// `POST /api/oauth/validate`: whether the token is good, without
    /// reading anything. `Ok(())` if it is.
    ///
    /// The endpoint accepts tokens with `user:profile` (a subscription
    /// login) and refuses inference-only ones.
    ///
    /// # Errors
    /// [`Error::Unauthorized`] for a bad, expired or revoked token, or one
    /// without an accepted scope; [`Error`] otherwise.
    pub async fn validate(&self, token: &Secret) -> Result<(), Error> {
        let req = self
            .request(Method::POST, token, &["api", "oauth", "validate"])?
            .header(header::CONTENT_TYPE, "application/json")
            .body("null");
        let (status, _body) = send(req).await?;
        debug_assert!(status.is_success());
        Ok(())
    }

    /// `GET /api/oauth/claude_cli/roles`: the user's roles in the token's
    /// organization.
    ///
    /// # Errors
    /// [`Error`].
    pub async fn roles(&self, token: &Secret) -> Result<Roles, Error> {
        self.get_json(token, &["api", "oauth", "claude_cli", "roles"])
            .await
    }

    /// `GET /api/oauth/organizations/{org}/prepaid/credits`: the prepaid
    /// credit balance. `org` is [`Organization::uuid`](crate::Organization::uuid).
    ///
    /// # Errors
    /// [`Error`].
    pub async fn prepaid_credits(
        &self,
        token: &Secret,
        org: &str,
    ) -> Result<PrepaidCredits, Error> {
        self.get_json(
            token,
            &["api", "oauth", "organizations", org, "prepaid", "credits"],
        )
        .await
    }

    /// `GET /api/oauth/organizations/{org}/overage_spend_limit`: the extra
    /// usage limit and what's spent against it. Read-only: this crate never
    /// changes it.
    ///
    /// # Errors
    /// [`Error`].
    pub async fn overage_spend_limit(
        &self,
        token: &Secret,
        org: &str,
    ) -> Result<OverageSpendLimit, Error> {
        self.get_json(
            token,
            &["api", "oauth", "organizations", org, "overage_spend_limit"],
        )
        .await
    }

    /// Posts a refresh-token grant to the token endpoint and returns the
    /// new tokens.
    ///
    /// A plain call: it stores nothing and takes no lock. The posted
    /// refresh token is spent once this succeeds, so the caller stores the
    /// result with a compare-and-swap (see [`TokenSet`]).
    ///
    /// # Errors
    /// [`Error::Unauthorized`] with code `invalid_grant` for a revoked or
    /// already-rotated refresh token (a new login is needed); [`Error`]
    /// otherwise.
    pub async fn refresh(&self, req: &RefreshRequest) -> Result<TokenSet, Error> {
        let body = GrantBody {
            grant_type: "refresh_token",
            refresh_token: req.refresh_token.expose(),
            client_id: &req.client_id,
            scope: req.scope_string(),
        };
        // Moved into the request as is: no copy of the token is left here.
        let body = serde_json::to_vec(&body).map_err(|e| Error::Config(scrub_serde(&e)))?;
        let http = self
            .http
            .post(self.token_url.clone())
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json")
            .body(body);
        let (status, bytes) = send(http).await?;
        let resp: GrantResponse = decode(status, &bytes)?;
        Ok(TokenSet::from_response(
            resp,
            &req.refresh_token,
            SystemTime::now(),
        ))
    }

    fn url(&self, segments: &[&str]) -> Result<Url, Error> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|()| Error::Config("base URL can't take a path".into()))?
            .pop_if_empty()
            .extend(segments);
        Ok(url)
    }

    fn request(
        &self,
        method: Method,
        token: &Secret,
        segments: &[&str],
    ) -> Result<RequestBuilder, Error> {
        Ok(self
            .http
            .request(method, self.url(segments)?)
            .bearer_auth(token.expose())
            .header("anthropic-beta", OAUTH_BETA)
            .header(header::ACCEPT, "application/json"))
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        token: &Secret,
        segments: &[&str],
    ) -> Result<T, Error> {
        let (status, bytes) = send(self.request(Method::GET, token, segments)?).await?;
        decode(status, &bytes)
    }
}

/// Sends, reads the body (capped), and turns a non-success status into its
/// [`Error`].
async fn send(req: RequestBuilder) -> Result<(StatusCode, alloc::vec::Vec<u8>), Error> {
    let resp = req
        .send()
        .await
        .map_err(|e| Error::Transport(e.without_url()))?;
    let status = resp.status();
    let retry_after = retry_after(&resp);
    let body = read_body(resp).await?;
    if status.is_success() {
        return Ok((status, body));
    }
    let (code, message) = error_fields(&body);
    let s = status.as_u16();
    Err(match s {
        401 | 403 => Error::Unauthorized { status: s, code },
        400 if code.as_deref() == Some("invalid_grant") => Error::Unauthorized { status: s, code },
        429 => Error::RateLimited { retry_after },
        _ => Error::Http {
            status: s,
            code,
            message,
        },
    })
}

async fn read_body(mut resp: Response) -> Result<alloc::vec::Vec<u8>, Error> {
    let mut body = alloc::vec::Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| Error::Transport(e.without_url()))?
    {
        if body.len() + chunk.len() > MAX_BODY {
            return Err(Error::TooLarge { limit: MAX_BODY });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn decode<T: DeserializeOwned>(status: StatusCode, body: &[u8]) -> Result<T, Error> {
    serde_json::from_slice(body).map_err(|e| Error::Decode {
        status: status.as_u16(),
        message: scrub_serde(&e),
        snippet: snippet(body),
    })
}

/// `Retry-After`: delta-seconds or an HTTP date.
fn retry_after(resp: &Response) -> Option<Duration> {
    let v = resp
        .headers()
        .get(header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();
    if let Ok(secs) = v.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let at = httpdate::parse_http_date(v).ok()?;
    Some(
        at.duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_must_be_https_or_loopback() {
        assert!(parse_url("https://api.example.com", "u").is_ok());
        assert!(parse_url("http://127.0.0.1:8080", "u").is_ok());
        assert!(parse_url("http://localhost:1", "u").is_ok());
        assert!(parse_url("http://[::1]:1", "u").is_ok());
        assert!(parse_url("http://api.example.com", "u").is_err());
        assert!(parse_url("ftp://127.0.0.1", "u").is_err());
        assert!(parse_url("not a url", "u").is_err());
    }

    #[test]
    fn paths_are_joined_and_escaped() {
        let c = Client::builder()
            .base_url("https://api.example.com/prefix/")
            .build()
            .unwrap();
        assert_eq!(
            c.url(&["api", "oauth", "organizations", "a/b?c", "prepaid"])
                .unwrap()
                .as_str(),
            "https://api.example.com/prefix/api/oauth/organizations/a%2Fb%3Fc/prepaid"
        );
    }

    #[test]
    fn default_user_agent() {
        assert!(DEFAULT_USER_AGENT.starts_with("ccwire-oauth/"));
        assert!(Client::builder().user_agent("bad\nua").build().is_err());
    }
}
