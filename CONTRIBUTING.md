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

First-time setup:

1. Publish each crate once using an authorized crates.io account. Run
   `cargo publish -p ccwire --dry-run` and
   `cargo publish -p ccwire-oauth --dry-run`, then repeat without `--dry-run`.
   Configure credentials with `cargo login`; never commit a registry token.
2. For **each** crate, configure a crates.io trusted publisher with GitHub owner
   `talaagent`, repository `ccwire`, workflow `release.yml`, and environment
   `release`. The first publication requires a token because trusted publishing
   cannot create a new crate.
3. Allow GitHub Actions to create pull requests in the repository's Actions
   settings (organization policy must also permit this). Set repository variable
   `RELEASE_PR=true`. Keep a GitHub label named `release`.

After setup, pushes to `main` update a release PR. Review its version bumps and
changelogs, wait for CI, then merge it. The publish job exchanges GitHub OIDC
credentials for a short-lived crates.io token, publishes pending versions, and
creates tags and GitHub releases. Ordinary pushes do not publish with
`release_always = false`. Tags are `v<version>` for `ccwire` and
`ccwire-oauth-v<version>` for `ccwire-oauth`.

A missing crate or registry API failure fails the publish job; a green workflow
must not hide a missing initial publication. Check both package pages and docs.rs
builds after release:

- [ccwire](https://crates.io/crates/ccwire) · [API docs](https://docs.rs/ccwire)
- [ccwire-oauth](https://crates.io/crates/ccwire-oauth) · [API docs](https://docs.rs/ccwire-oauth)

Once initial publication is confirmed, remove the temporary Git-install fallback
from both READMEs. docs.rs builds all features, including the session API.

Contributions are licensed under the repository's Apache-2.0 license.
