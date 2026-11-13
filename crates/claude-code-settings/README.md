# claude-code-settings

[![crates.io](https://img.shields.io/crates/v/claude-code-settings.svg)](https://crates.io/crates/claude-code-settings)
[![docs.rs](https://docs.rs/claude-code-settings/badge.svg)](https://docs.rs/claude-code-settings)
[![CI](https://github.com/talaagent/ccwire/actions/workflows/ci.yml/badge.svg)](https://github.com/talaagent/ccwire/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/talaagent/ccwire/blob/main/LICENSE)

[API reference](https://docs.rs/claude-code-settings) ·
[Repository](https://github.com/talaagent/ccwire/tree/main/crates/claude-code-settings) ·
[Issues](https://github.com/talaagent/ccwire/issues)

[Claude Code](https://code.claude.com/docs)'s `settings.json` as Rust types.

Claude Code reads its configuration from layered JSON settings files and
from the `--settings` command-line argument. This crate types every key
those documents accept, so a program can build one and pass it to
`claude --settings`, or read settings files, layer them the way Claude Code
does, and write the result, without hand-built JSON.

- `Settings` has a field for every key in Claude Code's published JSON
  schema and its [settings reference](https://code.claude.com/docs/en/settings-reference),
  with nested objects typed: permissions, hooks, the sandbox, marketplaces,
  plugin configuration, model settings and the rest.
- Each field's docs say what Claude Code does with the key, the values it
  accepts, its default, where it may be set, and link to its entry in the
  settings reference.
- Keys, values, hook types and hook events the crate doesn't know are kept
  and written back unchanged, so a file from a newer Claude Code
  round-trips.
- `Settings::merge` applies Claude Code's layer merge.
- serde and serde_json only; no I/O.

This crate is not affiliated with or endorsed by Anthropic.

## Install

```sh
cargo add claude-code-settings
```

Rust 1.85 or newer.

## Quick start

Build a document and pass it to `claude --settings`. Every type is
`#[non_exhaustive]`: start from `Default::default()` and set fields. Only
what is set is written.

```rust
use claude_code_settings::{Hook, HookEvent, HookMatcher, PermissionMode, Permissions, Settings};

fn main() -> Result<(), serde_json::Error> {
    let mut permissions = Permissions::default();
    permissions.allow = Some(vec!["Bash(git diff *)".into(), "Read".into()]);
    permissions.deny = Some(vec!["Bash(rm *)".into()]);
    permissions.default_mode = Some(PermissionMode::DontAsk);

    let mut settings = Settings::default();
    settings.permissions = Some(permissions);
    settings.hooks.get_or_insert_default().insert(
        HookEvent::PreToolUse,
        vec![HookMatcher::matching("Bash", [Hook::command("./scripts/check-command")])],
    );

    let arg = settings.to_json_string()?;
    // std::process::Command::new("claude").args(["--settings", &arg, "-p", "..."]);
    assert!(arg.contains(r#""defaultMode":"dontAsk""#));
    Ok(())
}
```

Read a settings file, change it, and write it back. Unknown keys and values
survive:

```rust
use claude_code_settings::Settings;

fn main() -> Result<(), serde_json::Error> {
    let file = r#"{"model": "opus", "aKeyFromANewerVersion": {"enabled": true}}"#;
    let mut settings = Settings::from_json_str(file)?;
    assert_eq!(settings.unknown_keys(), ["aKeyFromANewerVersion"]);

    settings.model = Some("sonnet".into());
    let written = settings.to_json_string_pretty()?;
    assert!(written.contains("aKeyFromANewerVersion"));
    Ok(())
}
```

## How settings layer

Claude Code reads settings from several sources. Highest precedence first:

1. **Managed settings**: what an organization deploys. Nothing below
   overrides a managed key.
2. **Command line**: `--settings <file-or-json>`, for one session.
3. **Local project settings**: `.claude/settings.local.json`.
4. **Shared project settings**: `.claude/settings.json`.
5. **User settings**: `~/.claude/settings.json`.

It merges them instead of picking one:

- a scalar (string, number, Boolean) comes from the highest source that
  sets it;
- objects merge key by key (`env`, `permissions`, `sandbox`, ...);
- lists concatenate, so `permissions.allow`/`ask`/`deny` and each hook
  event's matcher groups add up across sources; `fallbackModel` and
  `modelPicker` are taken whole from the highest source.

A `deny` rule from any source beats an `allow` rule from any other: Claude
Code evaluates `deny`, then `ask`, then `allow`. Some keys are honored only
from some sources (managed-only keys, keys a project can't set); each
field's **Scope** line says which, and the merge doesn't enforce it. Claude
Code watches its settings files and reloads most keys in a running session
when a file changes; a few, such as `model`, are read only at start.

`Settings::merge` applies one layer over another. Call it on the lowest
layer with each higher one in turn:

```rust
use claude_code_settings::Settings;

fn main() -> Result<(), serde_json::Error> {
    let user = Settings::from_json_str(r#"{"model": "opus", "permissions": {"allow": ["Read"]}}"#)?;
    let project = Settings::from_json_str(
        r#"{"model": "sonnet", "permissions": {"allow": ["Bash(cargo test *)"]}}"#,
    )?;

    let mut effective = user;
    effective.merge(project)?;
    assert_eq!(effective.model.as_deref(), Some("sonnet"));
    let allow = effective.permissions.unwrap_or_default().allow.unwrap_or_default();
    assert_eq!(allow, ["Read", "Bash(cargo test *)"]);
    Ok(())
}
```

`merge_values` does the same on `serde_json::Value`s.

## Compatibility

| | |
| --- | --- |
| Claude Code | 2.1.322 (`CLAUDE_CODE_VERSION`) |
| JSON schema | fetched 2026-11-13 (`SCHEMA_FETCHED`) |
| Rust | 1.85 or newer |

The types follow two sources: the published JSON schema
(<https://json.schemastore.org/claude-code-settings.json>, a copy is in
`schema/` in the repository) and Claude Code's settings reference, for the
keys the schema doesn't list yet. A document that sets every key is
checked against the Claude Code release above: it loads, and the only keys
skipped are the ones a `--settings` document may not set (managed-only
keys).

Releases:

- New keys and values are additive: the types are `#[non_exhaustive]`, so
  they ship as patch releases. An older release of this crate keeps a newer
  key in `extra`, so nothing is lost meanwhile.
- A change that breaks the API (a key's type changes, a key is removed)
  bumps the minor version while the crate is 0.x.
- The changelog is `CHANGELOG.md`; release-plz writes it and checks semver
  with cargo-semver-checks.

## Keeping up with Claude Code

In the repository:

- `scripts/refresh-settings-schema` fetches the schema again and records
  the date.
- The `schema` test (`cargo test -p claude-code-settings`) then fails,
  listing every property by JSON path (such as
  `sandbox.network.newOption`, or `$defs/hookCommand.newField` for a hook
  handler field) and every enum value the schema has that the types and
  `tests/fixtures/full.json` don't. Add each to its type and to the fixture.
- `tests/cli.rs` hands the fixture to a real `claude --settings`, offline,
  and fails on any key it rejects:
  `CLAUDE_BIN=$(command -v claude) cargo test -p claude-code-settings --test cli -- --ignored`.
  A weekly workflow runs it against the newest Claude Code.

## Design notes

- Every key is an `Option`, written only when set. One key keeps `null`
  apart from absent: `teammateDefaultModel`, where `null` follows the lead's
  model.
- A value of the wrong type is an error, not dropped: Claude Code rejects
  the same document.
- `env` is a plain map: its keys are environment variables, not settings.
  `managedMcpServers` entries and an `mcp_tool` hook's `input` stay JSON
  objects, as the schema leaves them open.
- Marketplace and plugin sources are one struct, `Source`: `source` names
  the kind and the other fields are those the kind takes, so a new kind
  still parses.
- Numbers the schema calls `number` are `f64`, so `10` is written back as
  `10.0`, the same JSON number.
- Keys Claude Code accepts that neither the schema nor the settings
  reference names, and the settings reference's "global config" keys,
  which belong in `~/.claude.json` rather than `settings.json`, are left in
  `extra`.

## License

Licensed under the Apache License, Version 2.0
([LICENSE](https://github.com/talaagent/ccwire/blob/main/LICENSE)).
