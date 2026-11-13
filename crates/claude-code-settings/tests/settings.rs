//! Round trips, unknown keys and the layer merge.

#![allow(clippy::unwrap_used, reason = "test bodies")]

use claude_code_settings::{
    AttributionSetting, ChannelPluginEntry, CommandHook, CompactWindow, EffortLevel, Hook,
    HookEvent, HookMatcher, ManagedMcpServers, PermissionMode, Settings, SpinnerTip, merge_values,
};
use serde_json::{Value, json};

const FULL: &str = include_str!("fixtures/full.json");
const REAL_WORLD: &str = include_str!("fixtures/real-world.json");

/// Every number as an `f64`: a field typed `f64` writes `10` back as
/// `10.0`, the same JSON number.
fn normalized(v: Value) -> Value {
    match v {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(a) => a.into_iter().map(normalized).collect(),
        Value::Object(o) => o.into_iter().map(|(k, v)| (k, normalized(v))).collect(),
        other => other,
    }
}

/// Parses `json`, checks every key is typed, and that it writes back the
/// same document.
fn round_trip(json: &str) -> Settings {
    let settings = Settings::from_json_str(json).unwrap();
    assert_eq!(settings.unknown_keys(), Vec::<String>::new());
    let written: Value = serde_json::from_str(&settings.to_json_string().unwrap()).unwrap();
    let original: Value = serde_json::from_str(json).unwrap();
    assert_eq!(normalized(written), normalized(original));
    settings
}

fn settings(v: Value) -> Settings {
    serde_json::from_value(v).unwrap()
}

#[test]
fn full_fixture_round_trips_with_every_key_typed() {
    let s = round_trip(FULL);
    assert_eq!(s.model.as_deref(), Some("sonnet"));
    let perms = s.permissions.unwrap();
    assert_eq!(perms.default_mode, Some(PermissionMode::DontAsk));
    let hooks = s.hooks.unwrap();
    assert_eq!(hooks.len(), 33);
    let pre = &hooks[&HookEvent::PreToolUse][0];
    assert_eq!(pre.matcher.as_deref(), Some("Bash|mcp__.*"));
    let kinds: Vec<_> = pre.hooks.iter().map(|h| h.kind().unwrap()).collect();
    assert_eq!(kinds, ["command", "prompt", "agent", "http", "mcp_tool"]);
    let Hook::Command(cmd) = &pre.hooks[0] else {
        panic!("not a command hook: {:?}", pre.hooks[0]);
    };
    assert_eq!(cmd.timeout, Some(10.0));
    assert_eq!(cmd.condition.as_deref(), Some("Bash(git *)"));
    assert_eq!(s.teammate_default_model, Some(None));
    let opus = &s.model_settings.unwrap()["claude-opus-5-5"];
    assert_eq!(opus.effort_level, Some(EffortLevel::High));
    assert_eq!(
        opus.auto_compact_window,
        Some(CompactWindow::Named("auto".into()))
    );
}

#[test]
fn real_world_documents_round_trip() {
    let base = round_trip(REAL_WORLD);
    assert_eq!(base.output_style.as_deref(), Some("Concise"));
    // Keys a program that runs Claude Code adds per session.
    round_trip(
        &json!({
            "autoMemoryDirectory": "/tmp/memory",
            "includeGitInstructions": false,
            "autoCompactWindow": 200_000,
            "autoCompactEnabled": false,
            "allowManagedHooksOnly": true,
            "disableAllHooks": false,
            "env": {"CLAUDE_AUTOCOMPACT_PCT_OVERRIDE": "80", "GOAL": "x"},
            "permissions": {"disableBypassPermissionsMode": "disable"},
            "hooks": {
                "PreToolUse": [{
                    "matcher": "Agent|Task|Bash|WebSearch|WebFetch|mcp__.*",
                    "hooks": [{"type": "command", "command": "guard --pre-tool-use", "timeout": 10}],
                }],
                "Stop": [{"hooks": [{"type": "command", "command": "guard --stop", "timeout": 10}]}],
            },
        })
        .to_string(),
    );
}

#[test]
fn unknown_keys_values_and_hooks_round_trip_and_are_listed() {
    let doc = json!({
        "futureKey": {"a": 1},
        "permissions": {"allow": ["Read"], "defaultMode": "someNewMode", "newRule": true},
        "sandbox": {"network": {"newKnob": 3}},
        "hooks": {
            "NewEvent": [{"hooks": [{"type": "command", "command": "true"}]}],
            "Stop": [{"matcher": "x", "newGroupKey": 1, "hooks": [
                {"type": "websocket", "url": "ws://localhost"},
                {"type": "command", "command": "true", "newHookKey": "y"},
            ]}],
        },
        "theme": "my-theme",
        "teammateDefaultModel": "sonnet",
    });
    let s = settings(doc.clone());
    assert_eq!(serde_json::to_value(&s).unwrap(), doc);
    assert_eq!(
        s.permissions.as_ref().unwrap().default_mode,
        Some(PermissionMode::Other("someNewMode".into()))
    );
    assert!(!s.theme.as_ref().unwrap().is_known());
    assert_eq!(s.teammate_default_model, Some(Some("sonnet".into())));
    let mut unknown = s.unknown_keys();
    unknown.sort();
    assert_eq!(
        unknown,
        [
            "futureKey",
            "hooks.NewEvent",
            "hooks.Stop[0].hooks[0].type",
            "hooks.Stop[0].hooks[1].newHookKey",
            "hooks.Stop[0].newGroupKey",
            "permissions.newRule",
            "sandbox.network.newKnob",
        ]
    );
}

#[test]
fn absent_and_null_stay_apart() {
    assert_eq!(settings(json!({})).teammate_default_model, None);
    let null = settings(json!({"teammateDefaultModel": null}));
    assert_eq!(null.teammate_default_model, Some(None));
    assert_eq!(
        null.to_json_string().unwrap(),
        r#"{"teammateDefaultModel":null}"#
    );
    assert_eq!(Settings::default().to_json_string().unwrap(), "{}");
}

#[test]
fn a_wrong_type_is_an_error() {
    let err = Settings::from_json_str(r#"{"cleanupPeriodDays":"x"}"#).unwrap_err();
    assert!(err.to_string().contains("invalid type"), "{err}");
    let err = Settings::from_json_str(
        r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","timeout":"x"}]}]}}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid type"), "{err}");
}

#[test]
fn built_settings_serialize_as_claude_code_spells_them() {
    let mut s = Settings::default();
    s.include_co_authored_by = Some(false);
    s.force_login_org_uuid = Some(claude_code_settings::OneOrMany::One("org".into()));
    let mut check = CommandHook::new("check").with_timeout(10.0);
    check.run_async = Some(true);
    s.hooks.get_or_insert_default().insert(
        HookEvent::PreToolUse,
        vec![HookMatcher::matching("Bash", [Hook::Command(check)])],
    );
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        json!({
            "includeCoAuthoredBy": false,
            "forceLoginOrgUUID": "org",
            "hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [
                {"type": "command", "command": "check", "async": true, "timeout": 10.0},
            ]}]},
        })
    );
}

#[test]
fn merge_scalars_take_the_higher_layer() {
    let mut s = settings(json!({"model": "opus", "verbose": true, "effortLevel": "low"}));
    s.merge(settings(json!({"model": "sonnet", "effortLevel": "high"})))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        json!({"model": "sonnet", "verbose": true, "effortLevel": "high"})
    );
}

#[test]
fn merge_lists_concatenate_without_repeats() {
    let mut s = settings(json!({
        "permissions": {"allow": ["Read", "Bash(ls:*)"], "deny": ["Bash(rm:*)"], "defaultMode": "plan"},
        "sandbox": {"excludedCommands": ["git"]},
        "env": {"A": "1", "B": "1"},
    }));
    s.merge(settings(json!({
        "permissions": {"allow": ["Bash(ls:*)", "Edit"], "ask": ["Bash(git push:*)"], "defaultMode": "dontAsk"},
        "sandbox": {"excludedCommands": ["docker"], "enabled": true},
        "env": {"B": "2"},
    })))
    .unwrap();
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        json!({
            "permissions": {
                "allow": ["Read", "Bash(ls:*)", "Edit"],
                "deny": ["Bash(rm:*)"],
                "ask": ["Bash(git push:*)"],
                "defaultMode": "dontAsk",
            },
            "sandbox": {"excludedCommands": ["git", "docker"], "enabled": true},
            "env": {"A": "1", "B": "2"},
        })
    );
}

#[test]
fn merge_hooks_combine_per_event() {
    let group = |c: &str| json!({"hooks": [{"type": "command", "command": c}]});
    let mut s = settings(json!({"hooks": {"Stop": [group("a")], "SessionStart": [group("s")]}}));
    // The same group in both layers runs twice: objects aren't deduplicated.
    s.merge(settings(
        json!({"hooks": {"Stop": [group("b"), group("a")]}}),
    ))
    .unwrap();
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        json!({"hooks": {"SessionStart": [group("s")], "Stop": [group("a"), group("b"), group("a")]}})
    );
}

#[test]
fn merge_whole_value_keys() {
    let mut s = settings(json!({
        "fallbackModel": ["sonnet", "haiku"],
        "modelPicker": {"options": [{"model": "opus"}], "replaceBuiltInOptions": true},
        "extraKnownMarketplaces": {
            "a": {"source": {"source": "github", "repo": "x/a", "ref": "main"}},
            "b": {"source": {"source": "npm", "package": "b"}},
        },
    }));
    s.merge(settings(json!({
        "fallbackModel": ["haiku"],
        "modelPicker": {"options": [{"model": "sonnet"}]},
        "extraKnownMarketplaces": {"a": {"source": {"source": "github", "repo": "y/a"}}},
    })))
    .unwrap();
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        json!({
            "fallbackModel": ["haiku"],
            "modelPicker": {"options": [{"model": "sonnet"}]},
            "extraKnownMarketplaces": {
                "a": {"source": {"source": "github", "repo": "y/a"}},
                "b": {"source": {"source": "npm", "package": "b"}},
            },
        })
    );
}

#[test]
fn merge_keeps_unknown_keys_and_takes_null_and_type_changes() {
    let mut s = settings(
        json!({"future": {"a": [1], "b": 1}, "teammateDefaultModel": "sonnet", "forceLoginOrgUUID": "x"}),
    );
    s.merge(settings(json!({"future": {"a": [1, 2]}, "teammateDefaultModel": null, "forceLoginOrgUUID": ["y", "z"]})))
    .unwrap();
    assert_eq!(
        serde_json::to_value(&s).unwrap(),
        json!({"future": {"a": [1, 2], "b": 1}, "teammateDefaultModel": null, "forceLoginOrgUUID": ["y", "z"]})
    );
}

#[test]
fn merge_values_layers_raw_documents() {
    let mut lower = json!({"permissions": {"allow": ["Read"]}, "model": "opus"});
    merge_values(
        &mut lower,
        json!({"permissions": {"allow": ["Read", "Edit"]}}),
    );
    assert_eq!(
        lower,
        json!({"permissions": {"allow": ["Read", "Edit"]}, "model": "opus"})
    );
}

#[test]
fn alternative_shapes_round_trip() {
    let doc = json!({
        "attribution": false,
        "allowedChannelPlugins": ["telegram@official", {"marketplace": "m", "plugin": "p"}],
        "managedMcpServers": {"docs": {"type": "http", "url": "https://mcp.example.com"}},
        "spinnerTipsOverride": {"tips": ["plain", {"id": "a", "text": "b", "priority": -2}]},
        "strictPluginOnlyCustomization": true,
        "forceLoginOrgUUID": "org",
        "enabledPlugins": {"a@m": true, "b@m": ["1.0.0"]},
        "modelSettings": {"m": {"autoCompactWindow": 300_000}},
    });
    let s = settings(doc.clone());
    assert_eq!(s.unknown_keys(), Vec::<String>::new());
    assert_eq!(serde_json::to_value(&s).unwrap(), doc);
    assert_eq!(s.attribution, Some(AttributionSetting::Flag(false)));
    assert!(matches!(
        s.allowed_channel_plugins.as_deref(),
        Some([ChannelPluginEntry::Id(_), ChannelPluginEntry::Plugin(_)])
    ));
    assert!(matches!(
        s.managed_mcp_servers,
        Some(ManagedMcpServers::ByName(_))
    ));
    let tips = s.spinner_tips_override.unwrap().tips.unwrap();
    assert!(matches!(
        tips.as_slice(),
        [SpinnerTip::Text(_), SpinnerTip::Entry(_)]
    ));
    assert_eq!(
        s.model_settings.unwrap()["m"].auto_compact_window,
        Some(CompactWindow::Tokens(300_000))
    );
}

#[test]
fn version_constants() {
    assert_eq!(claude_code_settings::CLAUDE_CODE_VERSION, "2.1.322");
    let schema_readme = include_str!("../schema/README.md");
    assert!(
        schema_readme.contains(&format!(
            "Fetched: {}",
            claude_code_settings::SCHEMA_FETCHED
        )),
        "SCHEMA_FETCHED doesn't match schema/README.md: run scripts/refresh-settings-schema and update it"
    );
}
