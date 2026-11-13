//! The walk behind [`Settings::unknown_keys`](crate::Settings::unknown_keys):
//! every struct reports the keys in its `extra` map and walks its typed
//! children.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// A map key: a plain string, or a string enum whose `Other` is unknown.
pub(crate) trait Key {
    fn key(&self) -> &str;
    fn is_known(&self) -> bool;
}

impl Key for String {
    fn key(&self) -> &str {
        self
    }

    fn is_known(&self) -> bool {
        true
    }
}

/// Collects the dotted paths of the keys a value holds but doesn't type.
pub(crate) trait Unknown {
    fn unknown(&self, path: &str, out: &mut Vec<String>);
}

pub(crate) fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.into()
    } else {
        format!("{path}.{key}")
    }
}

impl<T: Unknown> Unknown for Option<T> {
    fn unknown(&self, path: &str, out: &mut Vec<String>) {
        if let Some(v) = self {
            v.unknown(path, out);
        }
    }
}

impl<T: Unknown> Unknown for Vec<T> {
    fn unknown(&self, path: &str, out: &mut Vec<String>) {
        for (i, v) in self.iter().enumerate() {
            v.unknown(&format!("{path}[{i}]"), out);
        }
    }
}

impl<K: Key, V: Unknown> Unknown for BTreeMap<K, V> {
    fn unknown(&self, path: &str, out: &mut Vec<String>) {
        for (k, v) in self {
            let path = join(path, k.key());
            if !k.is_known() {
                out.push(path.clone());
            }
            v.unknown(&path, out);
        }
    }
}

/// Implements [`Unknown`] for a struct with an `extra` map: its keys, then
/// the listed typed children (`field: "jsonKey"`).
macro_rules! walk {
    ($ty:ty { $($field:ident: $key:literal),* $(,)? }) => {
        impl $crate::walk::Unknown for $ty {
            fn unknown(&self, path: &str, out: &mut ::alloc::vec::Vec<::alloc::string::String>) {
                for k in self.extra.keys() {
                    out.push($crate::walk::join(path, k));
                }
                $($crate::walk::Unknown::unknown(
                    &self.$field,
                    &$crate::walk::join(path, $key),
                    out,
                );)*
            }
        }
    };
}
pub(crate) use walk;
