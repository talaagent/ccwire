//! [`Field`]: an optional key that tells absent from `null`.

use core::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An optional key of a frame: absent, `null`, or a value.
///
/// The CLI sends some keys as `null` and leaves others out; keeping the
/// difference lets a frame round-trip unchanged. Use it with
/// `#[serde(default, skip_serializing_if = "Field::is_absent")]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Field<T> {
    /// The key isn't there.
    #[default]
    Absent,
    /// The key is `null`.
    Null,
    /// The key has a value.
    Value(T),
}

impl<T> Field<T> {
    /// Whether the key is absent.
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    /// Whether the key is `null`.
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// The value, if there is one (absent and `null` read as `None`).
    pub const fn get(&self) -> Option<&T> {
        match self {
            Self::Value(v) => Some(v),
            Self::Absent | Self::Null => None,
        }
    }

    /// The value, if there is one, by value.
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Value(v) => Some(v),
            Self::Absent | Self::Null => None,
        }
    }

    /// The value through [`Deref`] (`Field<String>` gives an `&str`).
    pub fn as_deref(&self) -> Option<&T::Target>
    where
        T: Deref,
    {
        self.get().map(Deref::deref)
    }
}

impl<T> From<T> for Field<T> {
    fn from(v: T) -> Self {
        Self::Value(v)
    }
}

impl<T> From<Option<T>> for Field<T> {
    /// `None` is absent (leave the key out); use [`Field::Null`] for `null`.
    fn from(v: Option<T>) -> Self {
        v.map_or(Self::Absent, Self::Value)
    }
}

impl<T: Serialize> Serialize for Field<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(v) => s.serialize_some(v),
            Self::Absent | Self::Null => s.serialize_none(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Field<T> {
    /// Only called when the key is there (absent is the `default`).
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Option::<T>::deserialize(d)?.map_or(Self::Null, Self::Value))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Field;

    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct S {
        #[serde(default, skip_serializing_if = "Field::is_absent")]
        a: Field<String>,
    }

    #[test]
    fn absent_null_value_round_trip() {
        for v in [json!({}), json!({"a": null}), json!({"a": "x"})] {
            let s: S = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(serde_json::to_value(&s).unwrap(), v);
        }
        let s: S = serde_json::from_value(json!({"a": "x"})).unwrap();
        assert_eq!(s.a.as_deref(), Some("x"));
        assert!(serde_json::from_value::<S>(json!({"a": 1})).is_err());
    }
}
