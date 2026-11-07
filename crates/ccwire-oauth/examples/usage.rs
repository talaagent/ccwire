//! Read usage using an existing OAuth token from `CLAUDE_ACCESS_TOKEN`.
use ccwire_oauth::{Client, Secret};

#[expect(
    clippy::disallowed_methods,
    reason = "the executable entry point starts its Tokio runtime"
)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn core::error::Error>> {
    let token = Secret::new(std::env::var("CLAUDE_ACCESS_TOKEN")?);
    let client = Client::new()?;
    let usage = client.usage(&token).await?;
    for (name, window) in [
        ("five-hour", usage.five_hour),
        ("seven-day", usage.seven_day),
    ] {
        if let Some(window) = window {
            println!(
                "{name}: {:?}% (resets {:?})",
                window.utilization, window.resets_at
            );
        }
    }
    Ok(())
}
