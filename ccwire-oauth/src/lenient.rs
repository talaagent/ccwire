//! Field deserializers that never fail: a value of an unexpected type reads
//! as `None` (or an empty list) instead of failing the whole response.
//!
//! Each reads the field as a [`Value`] first, then converts it, so only the
//! field's own value is dropped. Use them with `#[serde(default,
//! deserialize_with = "...")]` so a missing field is `None` too.

use alloc::vec::Vec;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// Any `T`; `None` if the value isn't one.
pub(crate) fn opt<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    Ok(T::deserialize(Value::deserialize(d)?).ok())
}

/// A finite number, or a string holding one (`"12.50"`).
pub(crate) fn num<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    Ok(match Value::deserialize(d)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|f| f.is_finite()))
}

/// A whole non-negative count that fits a `u32` (`6`, `6.0`, `"6"`).
pub(crate) fn count<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    Ok(whole(Value::deserialize(d)?).and_then(|n| u32::try_from(n).ok()))
}

/// Seconds: a non-negative number (a fraction is cut) or a string holding
/// one. Saturates at `u64::MAX`.
pub(crate) fn seconds<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u64>, D::Error> {
    let v = Value::deserialize(d)?;
    if let Some(n) = whole(v.clone()) {
        return Ok(Some(n));
    }
    let f = match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "checked non-negative and finite; `as` saturates"
    )]
    Ok(f.filter(|f| f.is_finite() && *f >= 0.0).map(|f| f as u64))
}

/// A list; `null` or a non-list is empty, and an element that doesn't read
/// as a `T` is skipped.
pub(crate) fn list<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    Ok(match Value::deserialize(d)? {
        Value::Array(a) => a
            .into_iter()
            .filter_map(|v| T::deserialize(v).ok())
            .collect(),
        _ => Vec::new(),
    })
}

/// A whole non-negative number, from an integer, an integral float or a
/// string holding either.
fn whole(v: Value) -> Option<u64> {
    let f = match v {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                return Some(u);
            }
            n.as_f64()?
        }
        Value::String(s) => {
            let s = s.trim();
            if let Ok(u) = s.parse::<u64>() {
                return Some(u);
            }
            s.parse::<f64>().ok()?
        }
        _ => return None,
    };
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "checked whole, non-negative and below 2^64"
    )]
    (f.is_finite() && f >= 0.0 && f.fract() == 0.0 && f < u64::MAX as f64).then_some(f as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize, Debug, Default)]
    #[serde(default)]
    struct T {
        #[serde(deserialize_with = "num")]
        n: Option<f64>,
        #[serde(deserialize_with = "count")]
        c: Option<u32>,
        #[serde(deserialize_with = "seconds")]
        s: Option<u64>,
        #[serde(deserialize_with = "opt")]
        b: Option<bool>,
        #[serde(deserialize_with = "list")]
        l: Vec<u8>,
    }

    fn t(s: &str) -> T {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn wrong_types_are_none() {
        let v = t(r#"{"n":"x","c":-1,"s":"soon","b":"yes","l":null}"#);
        assert_eq!((v.n, v.c, v.s, v.b), (None, None, None, None));
        assert_eq!(v.l, [] as [u8; 0]);
        let v = t(r#"{"n":"12.50","c":6.0,"s":"30","b":true,"l":[1,"x",300,2]}"#);
        assert_eq!(
            (v.n, v.c, v.s, v.b),
            (Some(12.5), Some(6), Some(30), Some(true))
        );
        assert_eq!(v.l, [1, 2]);
        let v = t(r#"{"c":1.5,"s":1.5}"#);
        assert_eq!((v.c, v.s), (None, Some(1)));
        let v = t(r#"{"c":4294967296,"s":1e300}"#);
        assert_eq!((v.c, v.s), (None, Some(u64::MAX)));
        let v = t("{}");
        assert_eq!((v.n, v.c, v.s, v.b), (None, None, None, None));
    }
}
