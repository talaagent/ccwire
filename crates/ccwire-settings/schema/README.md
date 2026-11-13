# Claude Code settings schema

`claude-code-settings.json` is the published JSON schema for Claude
Code's settings.json, from <https://json.schemastore.org/claude-code-settings.json>.

Fetched: 2026-11-13

`scripts/refresh-settings-schema` (at the repository root) fetches it
again and updates the date. The `schema` test then lists every property
the schema has that `tests/fixtures/full.json` doesn't set; add each to
`Settings` (or the nested type) and to the fixture.
