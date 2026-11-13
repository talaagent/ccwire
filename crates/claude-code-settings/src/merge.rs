//! The layer merge: [`merge_values`].

use alloc::vec::Vec;

use serde_json::Value;

/// Layers the settings document `higher` over `lower`, in place, the way
/// Claude Code merges settings sources (checked against 2.1.322's
/// merge, a lodash `mergeWith`):
///
/// - Objects merge key by key, recursively: a key only one side sets
///   keeps its value (so `env`, `permissions`, `sandbox` and `hooks`
///   combine).
/// - Arrays concatenate, `lower`'s entries first, and repeated scalars
///   are dropped (so `permissions.allow`/`deny`/`ask` and each hook
///   event's matcher groups combine). Two keys are whole values instead:
///   `fallbackModel` (an ordered chain) and `modelPicker`, where
///   `higher`'s wins.
/// - `extraKnownMarketplaces` merges one level deep: a marketplace
///   `higher` names replaces `lower`'s whole.
/// - Anything else (scalars, `null`, a type change): `higher` wins.
///
/// Merging decides what each key holds, not what it means: a `deny` rule
/// beats an `allow` rule however the layers were ordered, and Claude Code
/// reads some keys only from some sources (managed-only keys, keys it
/// ignores in project settings), which this doesn't model.
pub fn merge_values(lower: &mut Value, higher: Value) {
    merge_at(lower, higher, None);
}

fn merge_at(lower: &mut Value, higher: Value, key: Option<&str>) {
    if key == Some("modelPicker") {
        *lower = higher;
        return;
    }
    match (lower, higher) {
        (Value::Array(lo), Value::Array(hi)) => {
            if key == Some("fallbackModel") {
                *lo = hi;
                return;
            }
            lo.extend(hi);
            dedup_scalars(lo);
        }
        (Value::Object(lo), Value::Object(hi)) if key == Some("extraKnownMarketplaces") => {
            lo.extend(hi);
        }
        (Value::Object(lo), Value::Object(hi)) => {
            for (k, v) in hi {
                match lo.get_mut(&k) {
                    Some(slot) => merge_at(slot, v, Some(&k)),
                    None => {
                        lo.insert(k, v);
                    }
                }
            }
        }
        (lo, hi) => *lo = hi,
    }
}

/// Drops a scalar that repeats an earlier one (lodash `uniq`, which keeps
/// every object and array: they are compared by reference).
fn dedup_scalars(values: &mut Vec<Value>) {
    let mut seen: Vec<Value> = Vec::new();
    values.retain(|v| {
        if matches!(v, Value::Array(_) | Value::Object(_)) {
            return true;
        }
        if seen.contains(v) {
            return false;
        }
        seen.push(v.clone());
        true
    });
}
