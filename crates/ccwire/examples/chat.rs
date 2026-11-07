//! Run with `cargo run -p ccwire --example chat --features runtime -- "Hello"`.
//! Requires `claude` on PATH and an authenticated Claude account.
use core::time::Duration;
use std::{io, process::Stdio};

use ccwire::{
    Options,
    options::InputFormat,
    protocol::{Frame, Message},
    session::Session,
};
use tokio::process::Command;

#[expect(
    clippy::disallowed_methods,
    reason = "the executable entry point starts its Tokio runtime"
)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn core::error::Error>> {
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Say hello.".into());
    let options = Options::new()
        .input_format(InputFormat::StreamJson)
        .tools(core::iter::empty::<&str>())
        .max_turns(1)
        .no_session_persistence(true);
    let mut child = Command::new("claude")
        .args(options.to_argv())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing stdout"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("missing stdin"))?;

    // Bound the whole exchange, including shutdown, if the CLI stops responding.
    tokio::time::timeout(Duration::from_secs(120), async {
        let mut session = Session::new(stdout, stdin);
        session.send(prompt).await?;
        while let Some(frame) = session.next().await {
            let frame = frame?;
            let ends_turn = frame.ends_turn();
            match frame.split() {
                Frame::Message(Message::Assistant(message)) => {
                    for block in message.content() {
                        if let Some(text) = block.as_text() {
                            println!("{text}");
                        }
                    }
                }
                Frame::Message(Message::Result(result)) if !result.is_success() => {
                    return Err(io::Error::other(format!("Claude turn failed: {result:?}")).into());
                }
                _ => {}
            }
            if ends_turn {
                break;
            }
        }
        session.close().await?;
        let status = child.wait().await?;
        if !status.success() {
            return Err(io::Error::other(format!("claude exited with {status}")).into());
        }
        Ok::<(), Box<dyn core::error::Error>>(())
    })
    .await??;
    Ok(())
}
