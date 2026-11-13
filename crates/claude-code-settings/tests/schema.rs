//! The schema copy (`schema/claude-code-settings.json`) against the
//! types: every property it has is set in `fixtures/full.json`, which
//! `settings.rs` parses with no unknown key left over; and every enum value
//! it lists is a known variant. After `scripts/refresh-settings-schema`,
//! these tests list what the new schema added.

#![allow(clippy::unwrap_used, reason = "test bodies")]

use claude_code_settings::{
    BgIsolation, CredentialMode, CustomizationKind, DialogTimeout, DiffTool, DisableFlag,
    DisabledFlag, EditorMode, EffortLevel, HookEvent, LoginMethod, NotifChannel, PermissionMode,
    Platform, Settings, Shell, SkillOverride, SourceKind, SourcesBehavior, SpinnerVerbsMode,
    TeammateMode, Theme, Tui, UpdatesChannel, ViewMode, VoiceMode, WorkflowSize, WorktreeBaseRef,
};
use serde_json::{Map, Value};

const SCHEMA: &str = include_str!("../schema/claude-code-settings.json");
const FULL: &str = include_str!("fixtures/full.json");

/// A walk of the schema and the fixture together. A `$def` is walked
/// once, with the fixture's values from every place that uses it (so a
/// hook type needs setting in one event, not in each).
struct Walk<'a> {
    defs: &'a Map<String, Value>,
    uses: Vec<(&'a str, Vec<&'a Value>)>,
    missing: Vec<String>,
}

impl<'a> Walk<'a> {
    fn walk_refs(&mut self) {
        let mut done = 0;
        while done < self.uses.len() {
            let (name, values) = self.uses[done].clone();
            self.walk(&self.defs[name], &values, &format!("$defs/{name}"));
            done += 1;
        }
    }

    /// Records each property `schema` declares that none of `values` (the
    /// fixture's values at `path`) sets.
    fn walk(&mut self, schema: &'a Value, values: &[&'a Value], path: &str) {
        if let Some(r) = schema.get("$ref").and_then(Value::as_str) {
            let name = r.trim_start_matches("#/$defs/");
            match self.uses.iter_mut().find(|(n, _)| *n == name) {
                Some((_, seen)) => seen.extend(values),
                None => self.uses.push((name, values.to_vec())),
            }
            return;
        }
        for key in ["anyOf", "oneOf"] {
            for (i, branch) in schema
                .get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                let Some(props) = branch.get("properties").and_then(Value::as_object) else {
                    let items: Vec<&Value> =
                        values.iter().copied().filter(|v| v.is_array()).collect();
                    self.walk(branch, &items, path);
                    continue;
                };
                let matching: Vec<&Value> = values
                    .iter()
                    .copied()
                    .filter(|v| matches_branch(props, v))
                    .collect();
                if matching.is_empty() {
                    self.missing
                        .push(format!("{path} ({key} branch {i}: {})", keys(props)));
                } else {
                    self.walk(branch, &matching, path);
                }
            }
        }
        if let Some(items_schema) = schema.get("items") {
            let items: Vec<&Value> = values
                .iter()
                .filter_map(|v| v.as_array())
                .flatten()
                .collect();
            if !items.is_empty() {
                self.walk(items_schema, &items, &format!("{path}[]"));
            }
        }
        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            for (key, prop) in props {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                let set: Vec<&Value> = values.iter().filter_map(|v| v.get(key)).collect();
                if set.is_empty() {
                    self.missing.push(child);
                } else {
                    self.walk(prop, &set, &child);
                }
            }
        }
        if let Some(more @ Value::Object(_)) = schema.get("additionalProperties") {
            let declared = schema.get("properties").and_then(Value::as_object);
            let set: Vec<&Value> = values
                .iter()
                .filter_map(|v| v.as_object())
                .flat_map(|o| o.iter())
                .filter(|(k, _)| !declared.is_some_and(|d| d.contains_key(*k)))
                .map(|(_, v)| v)
                .collect();
            self.walk(more, &set, &format!("{path}.*"));
        }
    }
}

/// Whether `value` is an object of this branch: its `const` properties
/// match, or (with none) it sets one of the branch's properties.
fn matches_branch(props: &Map<String, Value>, value: &Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    let consts: Vec<_> = props
        .iter()
        .filter_map(|(k, p)| Some((k, p.get("const")?)))
        .collect();
    if consts.is_empty() {
        props.keys().any(|k| obj.contains_key(k))
    } else {
        consts.iter().all(|(k, c)| obj.get(*k) == Some(*c))
    }
}

fn keys(props: &Map<String, Value>) -> String {
    props.keys().cloned().collect::<Vec<_>>().join(", ")
}

#[test]
fn the_fixture_sets_every_schema_property() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let full: Value = serde_json::from_str(FULL).unwrap();
    let mut walk = Walk {
        defs: schema["$defs"].as_object().unwrap(),
        uses: Vec::new(),
        missing: Vec::new(),
    };
    let mut top = schema.clone();
    // `env`'s properties are environment variables, not settings: a map.
    top["properties"]["env"]
        .as_object_mut()
        .unwrap()
        .remove("properties");
    walk.walk(&top, &[&full], "");
    walk.walk_refs();
    assert!(
        walk.missing.is_empty(),
        "schema properties missing from tests/fixtures/full.json (add them to the types too):\n{}",
        walk.missing.join("\n")
    );
    assert_eq!(
        Settings::from_json_str(FULL).unwrap().unknown_keys(),
        Vec::<String>::new()
    );
}

/// The schema's `enum` list at `pointer`.
fn values(schema: &Value, pointer: &str) -> Vec<String> {
    let node = schema
        .pointer(pointer)
        .unwrap_or_else(|| panic!("no {pointer} in the schema"));
    let list = node
        .get("enum")
        .or_else(|| node.pointer("/items/enum"))
        .or_else(|| node.pointer("/additionalProperties/enum"))
        .or_else(|| node.pointer("/anyOf/0/enum"))
        .or_else(|| node.pointer("/anyOf/1/items/enum"))
        .unwrap_or_else(|| panic!("no enum at {pointer}"));
    list.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

/// A schema enum's pointer, and whether a value parses to a known variant.
type Check = (&'static str, fn(&str) -> bool);

macro_rules! known {
    ($($pointer:literal => $ty:ty,)*) => {
        &[$(($pointer, |v| <$ty>::from(v).is_known()),)*]
    };
}

const CHECKS: &[Check] = known! {
    "/properties/autoUpdatesChannel" => UpdatesChannel,
    "/properties/permissions/properties/defaultMode" => PermissionMode,
    "/properties/permissions/properties/disableBypassPermissionsMode" => DisableFlag,
    "/properties/permissions/properties/disableAutoMode" => DisableFlag,
    "/properties/disableAutoMode" => DisableFlag,
    "/properties/disableDeepLinkRegistration" => DisableFlag,
    "/properties/browserExternalPageTools" => DisabledFlag,
    "/properties/effortLevel" => EffortLevel,
    "/properties/forceLoginMethod" => LoginMethod,
    "/properties/teammateMode" => TeammateMode,
    "/properties/parentSettingsBehavior" => SourcesBehavior,
    "/properties/defaultShell" => Shell,
    "/$defs/hookCommand/anyOf/0/properties/shell" => Shell,
    "/properties/tui" => Tui,
    "/properties/viewMode" => ViewMode,
    "/properties/editorMode" => EditorMode,
    "/properties/preferredNotifChannel" => NotifChannel,
    "/properties/theme" => Theme,
    "/properties/askUserQuestionTimeout" => DialogTimeout,
    "/properties/workflowSizeGuideline" => WorkflowSize,
    "/properties/diffTool" => DiffTool,
    "/properties/skillOverrides" => SkillOverride,
    "/properties/spinnerVerbs/properties/mode" => SpinnerVerbsMode,
    "/properties/worktree/properties/baseRef" => WorktreeBaseRef,
    "/properties/worktree/properties/bgIsolation" => BgIsolation,
    "/properties/sandbox/properties/enabledPlatforms" => Platform,
    "/properties/sandbox/properties/credentials/properties/files/items/properties/mode" => CredentialMode,
    "/properties/voice/properties/mode" => VoiceMode,
    "/properties/strictPluginOnlyCustomization" => CustomizationKind,
};

#[test]
fn every_schema_enum_value_is_known() {
    let schema: Value = serde_json::from_str(SCHEMA).unwrap();
    let mut unknown = Vec::new();
    for (pointer, known) in CHECKS {
        for v in values(&schema, pointer) {
            if !known(&v) {
                unknown.push(format!("{pointer}: {v}"));
            }
        }
    }
    for event in schema["properties"]["hooks"]["properties"]
        .as_object()
        .unwrap()
        .keys()
    {
        if !HookEvent::from(event.as_str()).is_known() {
            unknown.push(format!("hooks: {event}"));
        }
    }
    let mut sources = Vec::new();
    collect_consts(&schema, "source", &mut sources);
    for s in sources {
        if !SourceKind::from(s.as_str()).is_known() {
            unknown.push(format!("source: {s}"));
        }
    }
    assert!(
        unknown.is_empty(),
        "schema enum values with no variant:\n{}",
        unknown.join("\n")
    );
}

/// Every `const` of a property named `name`, anywhere in `schema`.
fn collect_consts(schema: &Value, name: &str, out: &mut Vec<String>) {
    match schema {
        Value::Object(o) => {
            if let Some(c) = o
                .get("properties")
                .and_then(|p| p.get(name))
                .and_then(|p| p.get("const"))
            {
                out.push(c.as_str().unwrap().to_owned());
            }
            o.values().for_each(|v| collect_consts(v, name, out));
        }
        Value::Array(a) => a.iter().for_each(|v| collect_consts(v, name, out)),
        _ => {}
    }
}
