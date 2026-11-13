//! The real `claude` loads `fixtures/full.json` as written by `Settings`:
//! no key is rejected, and the only ones skipped are managed-only keys.
//! Opt-in (CI has no `claude`); offline, no login needed:
//!
//! ```sh
//! CLAUDE_BIN=$(command -v claude) cargo test -p ccwire-settings --test cli -- --ignored
//! ```

#![allow(clippy::unwrap_used, reason = "test bodies")]
#![allow(
    clippy::disallowed_methods,
    reason = "a test polls the child with thread::sleep"
)]

use core::time::Duration;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

use ccwire_settings::Settings;

const FULL: &str = include_str!("fixtures/full.json");

/// What `claude -p` prints (stdout and stderr) for a run with only
/// `settings` (`--setting-sources ""`), in an empty home.
fn run(settings: &str) -> String {
    let bin = std::env::var("CLAUDE_BIN").unwrap_or_else(|_| "claude".into());
    let home: PathBuf =
        std::env::temp_dir().join(format!("ccwire-settings-cli-{}", std::process::id()));
    let cwd = home.join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    let mut child = Command::new(bin)
        .args([
            "-p",
            "--model",
            "haiku",
            "--no-session-persistence",
            "--output-format",
            "text",
        ])
        .args([
            "--system-prompt",
            "s",
            "--tools",
            "",
            "--setting-sources",
            "",
        ])
        .args(["--settings", settings, "x"])
        .current_dir(&cwd)
        .env("HOME", &home)
        .env("CLAUDE_CONFIG_DIR", home.join(".claude"))
        .env_remove("CLAUDE_CODE_OAUTH_TOKEN")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("ANTHROPIC_AUTH_TOKEN")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(120) {
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let out = child.wait_with_output().unwrap();
    std::fs::remove_dir_all(&home).unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

#[test]
#[ignore = "needs a claude binary: CLAUDE_BIN=... cargo test -p ccwire-settings --test cli -- --ignored"]
fn claude_loads_every_key() {
    // The check itself: a value of the wrong type doesn't load.
    let bad = run(r#"{"cleanupPeriodDays":"x"}"#);
    assert!(bad.contains("did not load"), "a bad document loaded: {bad}");

    let settings = Settings::from_json_str(FULL).unwrap();
    let out = run(&settings.to_json_string());
    assert!(!out.contains("did not load"), "{out}");
    assert!(!out.lines().any(|l| l.starts_with("error: ")), "{out}");
    let skipped: Vec<&str> = out
        .lines()
        .filter(|l| l.trim_start().starts_with("--settings"))
        .filter(|l| !l.contains("is only honored from managed settings"))
        .collect();
    assert!(
        skipped.is_empty(),
        "entries skipped:\n{}",
        skipped.join("\n")
    );
}
