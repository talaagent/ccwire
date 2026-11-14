---
name: update-claude-code-settings
description: Update the claude-code-settings crate for a new Claude Code release or schema change - add new settings.json keys, enum values, hook types or events, refresh the vendored SchemaStore schema, adjust the layer merge, bump the tested CLI version. Use when the `schema` test fails, the newest-cli workflow reports rejected or unknown keys, a Claude Code changelog mentions new settings, or someone asks to "update/refresh the settings crate/types".
---

# Updating claude-code-settings

The types in `crates/claude-code-settings/src` are written by hand. The
vendored schema is a test oracle, not a codegen input: never generate the
types from it.

## 1. Find what changed

```sh
scripts/refresh-settings-schema          # refetch json.schemastore.org/claude-code-settings.json, bump SCHEMA_FETCHED
cargo test -p claude-code-settings --test schema
```

- `the_fixture_sets_every_schema_property` lists schema properties that
  `tests/fixtures/full.json` doesn't set (so `Settings` lacks them).
- `every_schema_enum_value_is_known` lists enum/const values the Rust enums
  don't name.

The schema lags the docs. Also read the
[settings reference](https://code.claude.com/docs/en/settings-reference),
the [hooks reference](https://code.claude.com/docs/en/hooks) and the Claude
Code changelog for keys, hook events and hook types the schema doesn't have yet.

## 2. Add each key

For every new key, in the type that owns it (`settings.rs`, `permissions.rs`,
`hooks.rs`, `sandbox.rs`, `plugins.rs`, `models.rs`, `enums.rs`):

- A snake_case field with `#[serde(skip_serializing_if = "Option::is_none")]`
  (and a `rename` where serde's camelCase rule doesn't produce the exact key).
  `Option<...>` for every field; only what is set is written.
- A doc comment in the existing shape: one line on what Claude Code does with
  it, then `- **Values**:`, `- **Default**:`, `- **Scope**:` bullets, then
  `Docs: [`key`](https://code.claude.com/docs/en/settings-reference#anchor).`
  Copy the style of `auto_compact_window` in `settings.rs`. `missing_docs` is on.
- New enum values become variants; keep the `Other(String)` catch-all last.
- New nested objects get their own struct with an `extra` map, like the
  existing ones, so unknown keys round-trip.
- Set the key in `tests/fixtures/full.json` (a realistic value, not a
  placeholder).

Removed or renamed keys: keep the old field (documents in the wild still
carry it), say it's deprecated in its doc comment, and add the new one.

## 3. Merge rules

`merge.rs` mirrors Claude Code's layer merge: objects merge per key, arrays
concatenate without repeated scalars, scalars come from the higher layer,
and a few keys are replaced whole (`fallbackModel`, `modelPicker`) or merged
per entry (`extraKnownMarketplaces`). If a new key is an array or object that
Claude Code replaces whole, add it to that list with a test in
`tests/settings.rs`.

## 4. Check against the real CLI

```sh
CLAUDE_BIN=$(command -v claude) cargo test -p claude-code-settings --test cli -- --ignored
```

Offline, no login. It hands `full.json` to `claude --settings`: nothing may
be rejected, and only managed-only keys may be skipped. When it passes on a
newer release, bump `CLAUDE_CODE_VERSION` in `src/lib.rs` (and the version
the README and docs mention).

## 5. Finish

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc -p claude-code-settings --no-deps
```

Don't edit `CHANGELOG.md`: release-plz writes it from the
commit messages, so use a conventional commit (`feat(settings): ...` for new
keys, `fix(settings): ...` for corrections). New public fields are additive
(minor bump); a changed field type is breaking.
