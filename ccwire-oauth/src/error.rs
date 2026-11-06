//! [`Error`]: why a call failed, never with a token in it.

use alloc::string::{String, ToString};
use core::time::Duration;

use serde_json::Value;

/// Why a call failed.
///
/// No variant holds a token: response bodies are scrubbed (see
/// [`Error::Decode`]) and transport errors drop the URL.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The token was refused: HTTP 401 or 403, or (from the token endpoint)
    /// an `invalid_grant`, which means the refresh token was revoked or has
    /// already been rotated away. The login needs a new token.
    #[error("unauthorized (HTTP {status}{})", fmt_code(code.as_deref()))]
    Unauthorized {
        /// The HTTP status.
        status: u16,
        /// The API's error code, if the body had one (`invalid_grant`,
        /// `authentication_error`, `permission_error`, ...).
        code: Option<String>,
    },
    /// HTTP 429.
    #[error("rate limited{}", fmt_retry(*retry_after))]
    RateLimited {
        /// The `Retry-After` header, if it was there and readable (seconds
        /// or an HTTP date).
        retry_after: Option<Duration>,
    },
    /// Any other non-success status.
    #[error("HTTP {status}{}", fmt_code(code.as_deref()))]
    Http {
        /// The HTTP status.
        status: u16,
        /// The API's error code, if the body had one.
        code: Option<String>,
        /// The API's error message, if the body had one, scrubbed and cut
        /// short.
        message: Option<String>,
    },
    /// A success status whose body isn't the expected JSON.
    #[error("unexpected response (HTTP {status}): {message}")]
    Decode {
        /// The HTTP status.
        status: u16,
        /// What didn't match, with any quoted value in it replaced.
        message: String,
        /// The start of the body, with token-like values replaced.
        snippet: String,
    },
    /// The request didn't complete: connect, TLS, a timeout. The error
    /// carries no URL.
    #[error("transport: {0}")]
    Transport(#[source] reqwest::Error),
    /// The response body was larger than the client accepts.
    #[error("response body over {limit} bytes")]
    TooLarge {
        /// The limit, in bytes.
        limit: usize,
    },
    /// The client was misconfigured (a bad base URL, a header value).
    #[error("config: {0}")]
    Config(String),
}

impl Error {
    /// Whether the login itself is bad (re-login needed), as opposed to a
    /// passing failure.
    pub fn is_unauthorized(&self) -> bool {
        matches!(self, Self::Unauthorized { .. })
    }

    /// Whether a later retry may succeed: rate limiting, a 5xx, a transport
    /// failure.
    pub fn is_transient(&self) -> bool {
        match self {
            Self::RateLimited { .. } | Self::Transport(_) => true,
            Self::Http { status, .. } => *status >= 500,
            _ => false,
        }
    }
}

fn fmt_code(code: Option<&str>) -> String {
    code.map(|c| alloc::format!(", {}", scrub_text(c, 64)))
        .unwrap_or_default()
}

fn fmt_retry(retry_after: Option<Duration>) -> String {
    retry_after
        .map(|d| alloc::format!(", retry after {}s", d.as_secs()))
        .unwrap_or_default()
}

/// The most of a body kept in [`Error::Decode`]'s snippet.
pub(crate) const SNIPPET_LEN: usize = 256;
const MESSAGE_LEN: usize = 200;
const REDACTED: &str = "<redacted>";

/// Object keys whose values are replaced in snippets, matched as
/// lowercase substrings.
const SECRET_KEYS: &[&str] = &[
    "token",
    "secret",
    "password",
    "key",
    "code",
    "authorization",
    "cookie",
    "credential",
];

/// `{"error": "code", "error_description": "..."}` (OAuth) or
/// `{"type": "error", "error": {"type": "...", "message": "..."}}` (the
/// API): the code and the message, scrubbed.
pub(crate) fn error_fields(body: &[u8]) -> (Option<String>, Option<String>) {
    let Ok(Value::Object(o)) = serde_json::from_slice::<Value>(body) else {
        return (None, None);
    };
    let s = |v: Option<&Value>, n| v.and_then(Value::as_str).map(|s| scrub_text(s, n));
    match o.get("error") {
        Some(Value::String(c)) => (
            Some(scrub_text(c, 64)),
            s(o.get("error_description"), MESSAGE_LEN),
        ),
        Some(Value::Object(e)) => (s(e.get("type"), 64), s(e.get("message"), MESSAGE_LEN)),
        _ => (None, None),
    }
}

/// A serde error's text with every `"..."` replaced: `serde_json` quotes the
/// offending string value, which could be a token.
pub(crate) fn scrub_serde(e: &serde_json::Error) -> String {
    let text = e.to_string();
    let mut out = String::with_capacity(text.len());
    let mut parts = text.split('"');
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    for (i, part) in parts.enumerate() {
        out.push('"');
        out.push_str(if i % 2 == 0 { REDACTED } else { part });
    }
    out
}

/// The start of a body, for [`Error::Decode`]. JSON keeps its shape with
/// the values of token-like keys (and anything that looks like a token)
/// replaced; anything else is cut and its token-like words replaced.
pub(crate) fn snippet(body: &[u8]) -> String {
    match serde_json::from_slice::<Value>(body) {
        Ok(mut v) => {
            scrub_value(&mut v);
            truncate(&v.to_string(), SNIPPET_LEN)
        }
        Err(_) => scrub_text(&String::from_utf8_lossy(body), SNIPPET_LEN),
    }
}

fn scrub_value(v: &mut Value) {
    match v {
        Value::Object(o) => {
            for (k, val) in o.iter_mut() {
                let k = k.to_ascii_lowercase();
                if SECRET_KEYS.iter().any(|s| k.contains(s)) && !val.is_null() {
                    *val = Value::String(REDACTED.into());
                } else {
                    scrub_value(val);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(scrub_value),
        Value::String(s) if looks_like_token(s) => *s = REDACTED.into(),
        _ => {}
    }
}

/// Free text, cut to `max` chars, with token-like words replaced.
pub(crate) fn scrub_text(text: &str, max: usize) -> String {
    let words: alloc::vec::Vec<&str> = text
        .split_inclusive(char::is_whitespace)
        .map(|w| {
            if looks_like_token(w.trim()) {
                REDACTED
            } else {
                w
            }
        })
        .collect();
    truncate(&words.concat(), max)
}

/// An Anthropic token (`sk-ant-...`), a bearer header, or a long run of
/// token characters with no spaces.
fn looks_like_token(s: &str) -> bool {
    let s = s.trim_matches(|c: char| matches!(c, '"' | '\'' | ',' | ';'));
    s.contains("sk-ant-")
        || s.to_ascii_lowercase().starts_with("bearer")
        || (s.len() >= 32
            && s.chars().all(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '+' | '/' | '=')
            }))
}

fn truncate(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => {
            let mut out = s[..i].to_string();
            out.push('…');
            out
        }
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snippet_redacts_token_keys_and_values() {
        let body = br#"{"access_token":"sk-ant-oat01-fake","refresh_token":"r","expires_in":"soon","note":"sk-ant-ort01-other","nested":{"api_key":"k"},"ok":"short"}"#;
        let s = snippet(body);
        assert!(!s.contains("sk-ant"), "{s}");
        assert!(!s.contains("\"r\""), "{s}");
        assert!(s.contains("\"expires_in\":\"soon\""), "{s}");
        assert!(s.contains("\"ok\":\"short\""), "{s}");
    }

    #[test]
    fn snippet_of_text_is_cut_and_scrubbed() {
        let body = alloc::format!("<html>sk-ant-oat01-fake {}</html>", "x".repeat(400));
        let s = snippet(body.as_bytes());
        assert!(!s.contains("sk-ant"), "{s}");
        assert!(s.chars().count() <= SNIPPET_LEN + 1);
    }

    #[test]
    fn serde_messages_lose_quoted_values() {
        #[derive(Debug, serde::Deserialize)]
        #[allow(dead_code)]
        struct T {
            n: u64,
        }
        let e = serde_json::from_str::<T>(r#"{"n":"sk-ant-oat01-fake"}"#).unwrap_err();
        let m = scrub_serde(&e);
        assert!(!m.contains("sk-ant"), "{m}");
        assert!(m.contains("invalid type"), "{m}");
    }

    #[test]
    fn error_fields_both_shapes() {
        assert_eq!(
            error_fields(
                br#"{"error":"invalid_grant","error_description":"Refresh token not found"}"#
            ),
            (
                Some("invalid_grant".into()),
                Some("Refresh token not found".into())
            )
        );
        assert_eq!(
            error_fields(
                br#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#
            ),
            (Some("rate_limit_error".into()), Some("slow down".into()))
        );
        assert_eq!(error_fields(b"nope"), (None, None));
    }

    #[test]
    fn display() {
        let e = Error::Unauthorized {
            status: 400,
            code: Some("invalid_grant".into()),
        };
        assert_eq!(e.to_string(), "unauthorized (HTTP 400, invalid_grant)");
        let e = Error::RateLimited {
            retry_after: Some(Duration::from_secs(30)),
        };
        assert_eq!(e.to_string(), "rate limited, retry after 30s");
        assert!(e.is_transient());
    }
}
