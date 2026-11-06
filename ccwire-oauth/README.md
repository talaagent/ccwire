# ccwire-oauth

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

## Example

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

Apache-2.0. See [LICENSE](LICENSE).
