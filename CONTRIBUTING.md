# Contributing

Bug reports, documentation improvements, protocol fixtures, and focused pull
requests are welcome. Open an [issue](https://github.com/talaagent/ccwire/issues)
for a substantial API change before implementing it. Include your Rust version,
Claude Code version, enabled features, and a minimal reproduction in bug reports.
Remove tokens, prompts, personal data, and account identifiers from logs.

## Development

Install Rust 1.85 or newer. CI also checks the minimum supported Rust version;
use the toolchain pinned in `.github/workflows/ci.yml` for matching CI locally.
An optional `nix develop` shell provides Rust, Clippy, rustfmt, and cargo-deny.

The workspace contains:

- `crates/ccwire`: protocol types, CLI options, framing, and optional async sessions.
- `crates/ccwire-oauth`: the Claude account client and token types.

Run from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked
```

`cargo test` includes doctests. Keep public API examples compilable, and use
`no_run` for examples that need credentials, a process, or network access.
The README examples are compiled as doctests too. Check both default and all
features: the protocol layer must remain usable without Tokio.

With the optional tools installed, also run `cargo deny check` and
`nix flake check`. CI runs these checks on pull requests.

## Protocol changes

Keep unknown variants and extra JSON fields intact. Preserve the difference
between missing and null fields, and retain the turn-ending behavior of malformed
`result` frames. Add regression coverage for behavioral changes.

See [the corpus guide](crates/ccwire/tests/corpus/README.md) for recorded stream-json
fixtures and scrubbing. OAuth fixtures live in `crates/ccwire-oauth/tests/fixtures`;
see [their guide](crates/ccwire-oauth/tests/fixtures/README.md). Tests use recorded
fixtures and a fake CLI or local HTTP server; normal tests need no account or
network credentials. Run live examples explicitly, using your own account.

## Releases (maintainers)

Each crate has its own version and changelog. Release automation is configured
in `release-plz.toml` and `.github/workflows/release.yml`.

Release automation uses **release-plz** for both release PRs and publishing.
Configure these repository Actions secrets:

- `RELEASE_PLZ_TOKEN`: a GitHub App token or PAT with repository contents,
  pull-request, and workflow write permissions. This lets release PRs update
  workflow files and trigger CI without relying on organization permissions
  for the built-in `GITHUB_TOKEN`.
- `CARGO_REGISTRY_TOKEN`: a crates.io publishing token. This is required for
  first-time publication and is configured for this repository.

Pushes to `main` update a release PR with version bumps and changelogs. Review
and merge it after CI passes. `release_always = false` publishes only merged
release PRs; release-plz publishes crates in dependency order, creates tags,
and creates GitHub releases. Tags are `v<version>` for `ccwire` and
`ccwire-oauth-v<version>` for `ccwire-oauth`.

For tokenless registry publication, configure each crate's trusted publisher
with owner `talaagent`, repository `ccwire`, workflow `release.yml`, environment
`release`, then remove the registry secret. Release-plz obtains OIDC credentials
itself. A new crate still needs a publishing token for its first upload.

Check the package and docs.rs pages after releasing:

- [ccwire](https://crates.io/crates/ccwire) · [API docs](https://docs.rs/ccwire)
- [ccwire-oauth](https://crates.io/crates/ccwire-oauth) · [API docs](https://docs.rs/ccwire-oauth)


Contributions are licensed under the repository's Apache-2.0 license.
