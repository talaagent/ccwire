# Changelog

Changes are recorded here by release-plz when preparing a release.

## [Unreleased]

### Added

- `Settings`: every key of Claude Code's settings.json as Rust types, with
  unknown keys kept, the layer merge Claude Code applies, and
  `to_json_string` for `--settings`.
