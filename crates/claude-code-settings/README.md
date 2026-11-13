# ccwire-settings

[![crates.io](https://img.shields.io/crates/v/ccwire-settings.svg)](https://crates.io/crates/ccwire-settings)
[![docs.rs](https://docs.rs/ccwire-settings/badge.svg)](https://docs.rs/ccwire-settings)
[![CI](https://github.com/talaagent/ccwire/actions/workflows/ci.yml/badge.svg)](https://github.com/talaagent/ccwire/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/talaagent/ccwire/blob/main/LICENSE)

[API reference](https://docs.rs/ccwire-settings) ·
[Repository](https://github.com/talaagent/ccwire) ·
[Issues](https://github.com/talaagent/ccwire/issues)

Claude Code's settings.json as Rust types: build a `--settings` document
instead of a hand-made JSON map, or read settings files, layer them the way
Claude Code does, and write the result.

- `Settings`: every key Claude Code's settings accept, with the nested
  objects typed (`permissions`, `hooks`, `sandbox`, marketplaces, plugin
  configs, `modelSettings`, ...).
- `Settings::to_json_string()`: the document for `claude --settings`.
- `Settings::merge(higher)`: layer one document over another, as Claude
  Code merges its settings sources. `merge_values` does the same for raw
  `serde_json::Value`s.
- `Settings::unknown_keys()`: the keys a document holds that this crate
  doesn't model.

serde and serde_json only; no I/O.

This crate is not affiliated with or endorsed by Anthropic.

## Install

```sh
cargo add ccwire-settings
```

## Build a `--settings` document

```rust
use ccwire_settings::{Hook, HookEvent, HookMatcher, PermissionMode, Permissions, Settings};

let mut settings = Settings {
    include_git_instructions: Some(false),
    permissions: Some(Permissions {
        allow: Some(vec!["Bash(git diff:*)".into()]),
        default_mode: Some(PermissionMode::DontAsk),
        ..Permissions::default()
    }),
    ..Settings::default()
};
settings.hooks.get_or_insert_default().insert(
    HookEvent::PreToolUse,
    vec![HookMatcher::matching("Bash", [Hook::command("my-guard")])],
);
let arg = settings.to_json_string();
// Command::new("claude").args(["--settings", &arg, ...])
```

## Layer settings files

```rust
use ccwire_settings::Settings;

let user = r#"{"permissions":{"allow":["Read"]},"model":"opus"}"#;
let project = r#"{"permissions":{"deny":["Bash(rm:*)"]},"model":"sonnet"}"#;
let mut merged = Settings::from_json_str(user).expect("valid settings");
merged.merge(Settings::from_json_str(project).expect("valid settings"));
assert_eq!(merged.model.as_deref(), Some("sonnet"));
let perms = merged.permissions.unwrap();
assert_eq!(perms.allow.unwrap(), ["Read"]);
assert_eq!(perms.deny.unwrap(), ["Bash(rm:*)"]);
```

The merge follows Claude Code's (checked against 2.1.322): objects merge
key by key, arrays concatenate (lower layer first, repeated strings
dropped), and anything else is the higher layer's. `fallbackModel` and
`modelPicker` are taken whole, and `extraKnownMarketplaces` per
marketplace. It doesn't model which sources Claude Code reads a key from
(managed-only keys, keys project settings can't set), and a `deny` rule
beats an `allow` rule whatever the layer order.

## Sources and checks

The keys come from two sources, both checked in:

- the published JSON schema, <https://json.schemastore.org/claude-code-settings.json>,
  copied to `schema/` with its fetch date. `scripts/refresh-settings-schema`
  re-fetches it, and the `schema` test lists every property (top level and
  nested) and every enum value it has that the types don't;
- Claude Code's settings reference, <https://code.claude.com/docs/en/settings-reference>,
  for the keys the schema doesn't have yet (`autoCompactWindow`,
  `modelSettings`, `modelPicker`, `allowedProviders`, ...).

`tests/fixtures/full.json` sets every one of them. The tests parse it with
no unknown key left over and write it back unchanged; an opt-in test
(`tests/cli.rs`, also run weekly against the newest CLI) hands it to a real
`claude` through `--settings` and checks nothing is rejected. With
Claude Code 2.1.322 every key loads; the only ones skipped are the three a
`--settings` document may not set (`managedMcpServers`, `deniedModels`,
`availableModelsMatch`: managed settings only).

## Decisions

- **Every key is an `Option`**, skipped when `None`; nothing has a default
  value of its own, so a document says only what it sets. One key keeps
  `null` apart from absent: `teammateDefaultModel` (`Option<Option<String>>`),
  where `null` means "follow the lead's model".
- **Unknown keys round-trip.** Every object has a `#[serde(flatten)] extra`
  map, enums have `Other(String)`, hooks of an unknown `type` are
  `Hook::Other` with the whole object, and `hooks` is a map keyed by
  `HookEvent` (whose `Other` holds a new event). A host file written for a
  newer Claude Code survives a parse, a merge and a write.
- **A value of the wrong type is an error**, not dropped: Claude Code
  rejects the same document.
- **Structs aren't `#[non_exhaustive]`**, so they can be built with
  `..Default::default()`. A new key is a new field: a minor release while
  the crate is 0.x. Enums are `#[non_exhaustive]`.
- **Numbers** the schema calls `number` are `f64` (a hook's `timeout`,
  `statusLine.padding`, rates); `integer`s are `u64` (`u16` for ports).
  An `f64` field writes `10` back as `10.0`, the same JSON number.
- **Marketplace and plugin sources are one struct**, `Source`: `source`
  names the kind (`SourceKind`) and the other fields are those the kind
  takes. The schema has three slightly different unions of nine kinds;
  one struct covers them all and still parses a kind added later.
- **Untyped on purpose:**
  - `env` is a `BTreeMap<String, String>`: its keys are environment
    variables (the schema lists ~400), not settings.
  - `managedMcpServers` entries are JSON objects: MCP server configs, which
    the schema leaves open; the key is managed-only.
  - `mcp_tool` hooks' `input` is a JSON object: the tool's arguments.
- **Left in `extra`:** keys the CLI accepts but neither the schema nor the
  settings reference names (internal or unreleased ones, such as
  `autoDreamEnabled` or `buildGate`), and the settings reference's "Global
  config" keys (`copyOnSelect`, `defaultToAgentsView`, ...), which live in
  `~/.claude.json`, not settings.json. Removed keys that the schema still
  lists (`teammateDefaultModel`, `permissionExplainerEnabled`) or that the
  docs still document (`taskOutputMaxChars`) are typed.
- **Enum values the schema has and Claude Code 2.1.322 rejects** are kept
  (`PermissionMode::Delegate`, `Manual`), and values it accepts beyond the
  schema are added (`UpdatesChannel::Rc`, `EffortLevel::Max` for caps).

## License

Licensed under the Apache License, Version 2.0 (`LICENSE`).
