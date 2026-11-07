# ccwire-oauth

[![crates.io](https://img.shields.io/crates/v/ccwire-oauth.svg)](https://crates.io/crates/ccwire-oauth)
[![docs.rs](https://docs.rs/ccwire-oauth/badge.svg)](https://docs.rs/ccwire-oauth)
[![CI](https://github.com/talaagent/ccwire/actions/workflows/ci.yml/badge.svg)](https://github.com/talaagent/ccwire/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/talaagent/ccwire/blob/main/LICENSE)

[API reference](https://docs.rs/ccwire-oauth) ·
[Repository](https://github.com/talaagent/ccwire) ·
[Issues](https://github.com/talaagent/ccwire/issues)

A typed async client for the read-only Claude account endpoints an OAuth
login (a Claude Code subscription login) can call:

- usage: the 5-hour and 7-day windows, extra usage, the meters
- profile: the account and its organization
- validate a token
- roles in the organization
- prepaid credits and the overage spend limit (read-only)
- the refresh grant, as a plain function: refresh token in, new tokens out

It writes nothing: no billing, payment or API-key endpoint, nothing that
changes the account. Token storage and locking are the caller's; see
`TokenSet` for the compare-and-swap a refresh needs.

Tokens are `Secret`s: redacted in `Debug` and `Display`, not
serializable, their own buffer zeroed on drop (best effort: see `Secret`
for the copies that aren't). Errors never carry a token or the account's
personal data.

This crate is not affiliated with or endorsed by Anthropic. The endpoints
are not publicly documented and may change: unknown fields land in each
type's `extra` map, and a field that is missing, `null` or of an
unexpected type reads as `None` instead of failing the call.

## Install

```sh
cargo add ccwire-oauth
```

While the initial crates.io publication is pending, install from Git:

```toml
[dependencies]
ccwire-oauth = { git = "https://github.com/talaagent/ccwire" }
```

Rust 1.85 or newer and a Tokio runtime are required. Supply an existing
Claude OAuth access token; this crate does not implement browser login or
load credentials from disk.

## Read usage

```rust,no_run
async fn usage() -> Result<(), Box<dyn std::error::Error>> {
    use ccwire_oauth::{Client, Secret};

    let token = Secret::new(std::env::var("CLAUDE_ACCESS_TOKEN")?);
    let usage = Client::new()?.usage(&token).await?;
    if let Some(window) = usage.five_hour {
        println!("five-hour utilization: {:?}%", window.utilization);
    }
    Ok(())
}
```

Run the [complete example](https://github.com/talaagent/ccwire/blob/main/crates/ccwire-oauth/examples/usage.rs)
with `cargo run -p ccwire-oauth --example usage` after setting
`CLAUDE_ACCESS_TOKEN` in your environment.

## Refresh an expired token

```rust,no_run
use ccwire_oauth::{Client, Error, RefreshRequest, Secret};

async fn run() -> Result<(), Error> {
    let client = Client::new()?;
    let access = Secret::new(std::env::var("ACCESS_TOKEN").unwrap_or_default());

    match client.usage(&access).await {
        Ok(usage) => {
            if let Some(w) = usage.five_hour {
                println!("5h: {:?}% (resets {:?})", w.utilization, w.resets_at);
            }
        }
        // Expired: refresh. Run it to the end; a dropped refresh can lose
        // the rotated login, so refresh_and_store runs it in its own task.
        Err(e) if e.is_unauthorized() => {
            let req = RefreshRequest::new(Secret::new("stored-refresh-token"));
            let tokens = client
                .refresh_and_store(req, |tokens| async move {
                    // Compare-and-swap tokens.refresh_token into your store.
                    tokens
                })
                .await?;
            let usage = client.usage(&tokens.access_token).await?;
            println!("{:?}", usage.seven_day);
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
```

## License

Apache-2.0. See [LICENSE](https://github.com/talaagent/ccwire/blob/main/LICENSE).

For development and release instructions, see
[CONTRIBUTING.md](https://github.com/talaagent/ccwire/blob/main/CONTRIBUTING.md).
