//! [`Secret`]: a token that never prints.

use alloc::string::String;
use core::fmt;

use serde::{Deserialize, Deserializer};
use zeroize::Zeroize;

/// An access or refresh token.
///
/// Its [`Debug`](fmt::Debug) and [`Display`](fmt::Display) print
/// `<redacted>`, it has no `Serialize`, and its own buffer is zeroed when it
/// is dropped. [`Secret::expose`] is the only way to read it.
///
/// Zeroing is best effort, not a guarantee that no copy is left in memory.
/// Each clone is a separate buffer, zeroed when that clone drops. The
/// client zeroes the response bodies it reads, but not the copies made
/// outside it: the request body handed to reqwest (and its TLS and HTTP/2
/// buffers), reqwest's received chunks, the `String` a caller passes to
/// [`Secret::new`], and whatever a caller does with [`Secret::expose`].
///
/// Equality (`==`) is a plain string comparison, not constant-time: fine
/// for the compare-and-swap a refresh needs, not for checking a token an
/// untrusted party sent.
///
/// ```
/// use ccwire_oauth::Secret;
///
/// let token = Secret::new("not-a-real-token");
/// assert_eq!(format!("{token} {token:?}"), "<redacted> Secret(<redacted>)");
/// assert_eq!(token.expose(), "not-a-real-token");
/// ```
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps a token.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The token itself. Send it, compare it or store it; don't log it.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether the token is the empty string.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for Secret {
    fn from(token: String) -> Self {
        Self(token)
    }
}

impl From<&str> for Secret {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<'de> Deserialize<'de> for Secret {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_prints() {
        let s = Secret::new("abc123");
        assert_eq!(format!("{s}"), "<redacted>");
        assert_eq!(format!("{s:?}"), "Secret(<redacted>)");
        assert_eq!(format!("{:?}", Some(s.clone())), "Some(Secret(<redacted>))");
        assert_eq!(s.expose(), "abc123");
    }

    #[test]
    fn deserializes_from_a_string() {
        let s: Secret = serde_json::from_str(r#""tok""#).unwrap();
        assert_eq!(s.expose(), "tok");
        assert!(serde_json::from_str::<Secret>("1").is_err());
    }
}
