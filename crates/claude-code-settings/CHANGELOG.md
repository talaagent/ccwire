# Changelog

Changes are recorded here by release-plz when preparing a release.

## [Unreleased]

## [0.1.0](https://github.com/talaagent/ccwire/releases/tag/claude-code-settings-v0.1.0) - 2026-11-13

### Other

- rename ccwire-settings, document every key, standalone-crate API ([#18](https://github.com/talaagent/ccwire/pull/18))

### Added

- `Settings`: every key of Claude Code's settings.json (the published JSON
  schema and the settings reference) as Rust types, each documented with
  what Claude Code does with it, its values, default and scope.
- Lossless round-trips: unknown keys, enum values, hook types and hook
  events are kept and written back.
- `Settings::merge` and `merge_values`: Claude Code's layer merge.
- `Settings::to_json_string` for `--settings`, `to_json_string_pretty`,
  `from_json_str`, `unknown_keys`.
