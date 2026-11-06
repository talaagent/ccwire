use std::ffi::OsString;

use serde_json::{Map, Value, json};

use super::{InputFormat, Options, OutputFormat, PermissionMode, SessionStart};

/// `-p --output-format stream-json --verbose`: every default argv.
const BASE: [&str; 4] = ["-p", "--output-format", "stream-json", "--verbose"];

fn argv(o: &Options) -> Vec<String> {
    o.to_argv()
        .into_iter()
        .map(|a| a.into_string().expect("UTF-8 argument"))
        .collect()
}

/// `BASE` then `rest`.
fn with_base(rest: &[&str]) -> Vec<String> {
    BASE.iter().chain(rest).map(|s| (*s).to_owned()).collect()
}

/// `o`'s argv is `BASE` then `rest`.
#[track_caller]
fn check(o: &Options, rest: &[&str]) {
    assert_eq!(argv(o), with_base(rest));
}

#[test]
fn defaults() {
    check(&Options::new(), &[]);
    assert_eq!(Options::new(), Options::default());
}

// ------------------------------------------------------------ formats

#[test]
fn input_format() {
    let o = Options::new().input_format(InputFormat::StreamJson);
    assert_eq!(
        argv(&o),
        [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose"
        ]
    );
    let o = Options::new().input_format(InputFormat::Text);
    assert_eq!(argv(&o)[1..3], ["--input-format", "text"]);
}

#[test]
fn output_format_and_verbose() {
    let text = Options::new().output_format(OutputFormat::Text);
    assert_eq!(argv(&text), ["-p", "--output-format", "text"]);
    let json = Options::new().output_format(OutputFormat::Json);
    assert_eq!(argv(&json), ["-p", "--output-format", "json"]);
    assert_eq!(
        argv(&text.verbose(true)),
        ["-p", "--output-format", "text", "--verbose"]
    );
    assert_eq!(
        argv(&Options::new().verbose(false)),
        ["-p", "--output-format", "stream-json"]
    );
}

#[test]
fn stream_switches() {
    check(
        &Options::new().include_partial_messages(true),
        &["--include-partial-messages"],
    );
    check(
        &Options::new().include_hook_events(true),
        &["--include-hook-events"],
    );
    check(&Options::new().include_partial_messages(false), &[]);
}

// ------------------------------------------------------------ model

#[test]
fn model_fallback_effort() {
    check(&Options::new().model("sonnet"), &["--model", "sonnet"]);
    check(
        &Options::new().fallback_model("haiku"),
        &["--fallback-model", "haiku"],
    );
    check(&Options::new().effort("high"), &["--effort", "high"]);
    // The last call wins.
    check(
        &Options::new().model("opus").model("claude-sonnet-4-5"),
        &["--model", "claude-sonnet-4-5"],
    );
}

// ------------------------------------------------------------ permissions

#[test]
fn permission_modes() {
    for (mode, s) in [
        (PermissionMode::Default, "default"),
        (PermissionMode::AcceptEdits, "acceptEdits"),
        (PermissionMode::Auto, "auto"),
        (PermissionMode::BypassPermissions, "bypassPermissions"),
        (PermissionMode::Manual, "manual"),
        (PermissionMode::Plan, "plan"),
        (PermissionMode::DontAsk, "dontAsk"),
        (PermissionMode::Other("yolo".into()), "yolo"),
    ] {
        assert_eq!(PermissionMode::from(s), mode);
        assert_eq!(mode.to_string(), s);
        check(
            &Options::new().permission_mode(mode),
            &["--permission-mode", s],
        );
    }
    // A string goes through as is.
    check(
        &Options::new().permission_mode("dontAsk".to_owned()),
        &["--permission-mode", "dontAsk"],
    );
}

#[test]
fn permission_prompts_and_tool() {
    check(&Options::new().restricted(true), &["--restricted"]);
    check(
        &Options::new().permission_prompts("none"),
        &["--permission-prompts", "none"],
    );
    check(
        &Options::new()
            .permission_prompt_tool("stdio")
            .permission_prompts("host"),
        &[
            "--permission-prompts",
            "host",
            "--permission-prompt-tool",
            "stdio",
        ],
    );
    check(
        &Options::new().permission_prompt_tool("mcp__perm__ask"),
        &["--permission-prompt-tool", "mcp__perm__ask"],
    );
}

// ------------------------------------------------------------ tools

#[test]
fn mcp_config() {
    check(
        &Options::new().mcp_config(r#"{"mcpServers":{}}"#),
        &["--mcp-config", r#"{"mcpServers":{}}"#],
    );
    check(
        &Options::new()
            .mcp_config("/etc/a.json")
            .mcp_config("/etc/b.json")
            .strict_mcp_config(true),
        &[
            "--strict-mcp-config",
            "--mcp-config",
            "/etc/a.json",
            "--mcp-config",
            "/etc/b.json",
        ],
    );
}

#[test]
fn tool_lists() {
    check(
        &Options::new().tools(["Read", "Grep"]),
        &["--tools", "Read,Grep"],
    );
    // No built-in tools at all: an empty value, not no flag.
    check(
        &Options::new().tools(Vec::<String>::new()),
        &["--tools", ""],
    );
    check(
        &Options::new()
            .allowed_tools(["Read", "Bash(git status:*)"])
            .allowed_tools(["mcp__t__x"]),
        &["--allowedTools", "Read,Bash(git status:*),mcp__t__x"],
    );
    check(
        &Options::new().disallowed_tools(["Agent", "Task"]),
        &["--disallowedTools", "Agent,Task"],
    );
    // Empty allow and deny lists are left out.
    check(
        &Options::new()
            .allowed_tools(Vec::<String>::new())
            .disallowed_tools(Vec::<String>::new()),
        &[],
    );
    check(
        &Options::new().disable_slash_commands(true),
        &["--disable-slash-commands"],
    );
}

#[test]
fn add_dirs() {
    check(
        &Options::new().add_dir("/data/a").add_dir("/data/my dir"),
        &["--add-dir", "/data/a", "--add-dir", "/data/my dir"],
    );
}

// ------------------------------------------------------------ prompt

#[test]
fn prompts_and_name() {
    check(
        &Options::new().system_prompt("You are terse."),
        &["--system-prompt", "You are terse."],
    );
    check(
        &Options::new().append_system_prompt("Line one.\nLine \"two\"."),
        &["--append-system-prompt", "Line one.\nLine \"two\"."],
    );
    check(&Options::new().name("room: Ana"), &["--name", "room: Ana"]);
}

// ------------------------------------------------------------ settings

#[test]
fn setting_sources() {
    check(
        &Options::new().setting_sources(["user", "project"]),
        &["--setting-sources", "user,project"],
    );
    // None at all: an empty value.
    check(
        &Options::new().setting_sources(Vec::<String>::new()),
        &["--setting-sources", ""],
    );
}

#[test]
fn settings_one_key() {
    let o = Options::new().setting("autoMemoryDirectory", "/m");
    check(&o, &["--settings", r#"{"autoMemoryDirectory":"/m"}"#]);
}

/// The `--settings` JSON of `o`, parsed (key order depends on `serde_json`'s
/// features, so several keys compare as values).
fn settings_of(o: &Options) -> Value {
    let a = argv(o);
    let at = a
        .iter()
        .position(|x| x == "--settings")
        .expect("--settings");
    assert_eq!(
        a.iter().filter(|x| *x == "--settings").count(),
        1,
        "one --settings"
    );
    serde_json::from_str(&a[at + 1]).expect("JSON")
}

#[test]
fn settings_merge() {
    let mut m = Map::new();
    m.insert("a".into(), json!(1));
    m.insert("env".into(), json!({"X": "1"}));
    let o = Options::new()
        .setting("a", 0)
        .settings(m)
        .setting_env("Y", "2")
        .setting_env("X", "3")
        .setting("hooks", json!({"Stop": []}));
    assert_eq!(
        settings_of(&o),
        json!({"a": 1, "env": {"X": "3", "Y": "2"}, "hooks": {"Stop": []}})
    );
    // An `env` that isn't an object is replaced.
    let o = Options::new().setting("env", "junk").setting_env("K", "v");
    assert_eq!(settings_of(&o), json!({"env": {"K": "v"}}));
    assert_eq!(
        Options::new().setting("k", true).settings_json().as_deref(),
        Some(r#"{"k":true}"#)
    );
    assert_eq!(Options::new().settings_json(), None);
}

#[test]
fn settings_file() {
    check(
        &Options::new().settings_file("/etc/claude/s.json"),
        &["--settings", "/etc/claude/s.json"],
    );
    // Keys set win: the CLI takes one --settings.
    check(
        &Options::new()
            .settings_file("/etc/claude/s.json")
            .setting("k", 1),
        &["--settings", r#"{"k":1}"#],
    );
}

// ------------------------------------------------------------ session

const SID: &str = "0b5f0d6e-5d3a-4c55-9a3f-1f2e3d4c5b6a";

#[test]
fn session_start() {
    check(
        &Options::new().resume(SID),
        &["--resume=0b5f0d6e-5d3a-4c55-9a3f-1f2e3d4c5b6a"],
    );
    check(
        &Options::new().session_id(SID),
        &["--session-id=0b5f0d6e-5d3a-4c55-9a3f-1f2e3d4c5b6a"],
    );
    check(&Options::new().continue_last(), &["--continue"]);
    // One session: the last call wins.
    check(&Options::new().resume(SID).continue_last(), &["--continue"]);
    check(
        &Options::new().continue_last().session(SessionStart::New),
        &[],
    );
    check(
        &Options::new().session(SessionStart::Resume("s1".into())),
        &["--resume=s1"],
    );
}

#[test]
fn fork_session() {
    check(
        &Options::new().resume("s1").fork_session(true),
        &["--resume=s1", "--fork-session"],
    );
    check(
        &Options::new().continue_last().fork_session(true),
        &["--continue", "--fork-session"],
    );
    // Nothing to fork on a new session.
    check(&Options::new().fork_session(true), &[]);
    check(
        &Options::new().session_id("s1").fork_session(true),
        &["--session-id=s1"],
    );
    check(
        &Options::new().no_session_persistence(true),
        &["--no-session-persistence"],
    );
}

#[test]
fn fork_session_id() {
    check(
        &Options::new().resume("s1").fork_session_id("s2"),
        &["--resume=s1", "--fork-session", "--session-id=s2"],
    );
    check(
        &Options::new().fork_session_id("s2").continue_last(),
        &["--continue", "--fork-session", "--session-id=s2"],
    );
    // Turning the fork off drops its id; turning it on again doesn't bring
    // the id back.
    check(
        &Options::new()
            .resume("s1")
            .fork_session_id("s2")
            .fork_session(false)
            .fork_session(true),
        &["--resume=s1", "--fork-session"],
    );
    // Nothing to fork on a new session.
    check(&Options::new().fork_session_id("s2"), &[]);
    check(
        &Options::new().session_id("s1").fork_session_id("s2"),
        &["--session-id=s1"],
    );
}

// ------------------------------------------------------------ limits

#[test]
fn limits() {
    check(&Options::new().max_turns(3), &["--max-turns", "3"]);
    check(
        &Options::new().max_budget_usd(1.5),
        &["--max-budget-usd", "1.50"],
    );
    check(
        &Options::new().max_budget_usd(2.0),
        &["--max-budget-usd", "2.00"],
    );
    check(
        &Options::new().max_budget_usd(0.1 + 0.2),
        &["--max-budget-usd", "0.30"],
    );
    check(
        &Options::new().max_budget_usd(1e-9),
        &["--max-budget-usd", "0.00"],
    );
    check(
        &Options::new().max_budget_usd(0.25).max_turns(0),
        &["--max-turns", "0", "--max-budget-usd", "0.25"],
    );
}

#[test]
fn bad_budgets_panic() {
    for usd in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01] {
        let r = std::panic::catch_unwind(|| Options::new().max_budget_usd(usd));
        assert!(r.is_err(), "{usd} accepted");
    }
}

// ------------------------------------------------------------ extra

#[test]
fn extra_args_last() {
    check(
        &Options::new()
            .extra_args(["--debug", "api"])
            .model("sonnet")
            .extra_args([OsString::from("--x")]),
        &["--model", "sonnet", "--debug", "api", "--x"],
    );
}

#[test]
fn prompt_arg_after_double_dash() {
    // After every flag and extra arg, behind `--`: no variadic flag takes
    // it, and a leading dash or an empty prompt stays a prompt.
    check(
        &Options::new()
            .prompt_arg("hi there")
            .add_dir("/d")
            .allowed_tools(["Read"])
            .extra_args(["--debug"]),
        &[
            "--allowedTools",
            "Read",
            "--add-dir",
            "/d",
            "--debug",
            "--",
            "hi there",
        ],
    );
    check(&Options::new().prompt_arg("-x"), &["--", "-x"]);
    check(&Options::new().prompt_arg(""), &["--", ""]);
    // The last call wins.
    check(
        &Options::new().prompt_arg("a").prompt_arg("b"),
        &["--", "b"],
    );
}

// ------------------------------------------------------------ odd values

#[test]
fn leading_dash_values_are_joined() {
    check(
        &Options::new().append_system_prompt("- be terse"),
        &["--append-system-prompt=- be terse"],
    );
    check(&Options::new().model("-x"), &["--model=-x"]);
    check(&Options::new().name("--help"), &["--name=--help"]);
    check(&Options::new().add_dir("-dir"), &["--add-dir=-dir"]);
    check(&Options::new().tools(["-Read"]), &["--tools=-Read"]);
    check(&Options::new().resume("-s"), &["--resume=-s"]);
    // A dash later in the value is fine.
    check(&Options::new().model("a-b"), &["--model", "a-b"]);
}

#[test]
fn empty_and_spaced_values() {
    check(&Options::new().system_prompt(""), &["--system-prompt", ""]);
    check(&Options::new().name(""), &["--name", ""]);
    check(&Options::new().resume(""), &["--resume="]);
    check(
        &Options::new().append_system_prompt("  two  spaces 'and' $HOME `x`"),
        &["--append-system-prompt", "  two  spaces 'and' $HOME `x`"],
    );
    check(&Options::new().model("a=b"), &["--model", "a=b"]);
}

#[cfg(unix)]
#[test]
fn non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt;
    let dir = OsString::from_vec(b"/data/\xff".to_vec());
    let a = Options::new().add_dir(dir.clone()).to_argv();
    assert_eq!(a[a.len() - 2..], [OsString::from("--add-dir"), dir]);
}

// ------------------------------------------------------------ combinations

#[test]
fn everything_in_order() {
    let o = Options::new()
        .prompt_arg("prompt")
        .extra_args(["--extra"])
        .max_budget_usd(4.5)
        .max_turns(10)
        .no_session_persistence(true)
        .fork_session_id("s2")
        .resume("s1")
        .setting("k", "v")
        .setting_sources(["user"])
        .name("n")
        .append_system_prompt("ap")
        .system_prompt("sp")
        .add_dir("/d")
        .disable_slash_commands(true)
        .disallowed_tools(["Agent"])
        .allowed_tools(["Read"])
        .tools(["Read"])
        .mcp_config("{}")
        .strict_mcp_config(true)
        .permission_prompt_tool("stdio")
        .permission_prompts("host")
        .permission_mode(PermissionMode::Plan)
        .restricted(true)
        .effort("low")
        .fallback_model("haiku")
        .model("sonnet")
        .include_hook_events(true)
        .include_partial_messages(true)
        .input_format(InputFormat::StreamJson);
    assert_eq!(
        argv(&o),
        [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--include-hook-events",
            "--model",
            "sonnet",
            "--fallback-model",
            "haiku",
            "--effort",
            "low",
            "--restricted",
            "--permission-mode",
            "plan",
            "--permission-prompts",
            "host",
            "--permission-prompt-tool",
            "stdio",
            "--strict-mcp-config",
            "--mcp-config",
            "{}",
            "--tools",
            "Read",
            "--allowedTools",
            "Read",
            "--disallowedTools",
            "Agent",
            "--disable-slash-commands",
            "--add-dir",
            "/d",
            "--system-prompt",
            "sp",
            "--append-system-prompt",
            "ap",
            "--name",
            "n",
            "--setting-sources",
            "user",
            "--settings",
            r#"{"k":"v"}"#,
            "--resume=s1",
            "--fork-session",
            "--session-id=s2",
            "--no-session-persistence",
            "--max-turns",
            "10",
            "--max-budget-usd",
            "4.50",
            "--extra",
            "--",
            "prompt",
        ]
    );
}

/// A sandboxed, resumable session: restricted, only its own MCP server,
/// an allowlist, a settings env.
#[test]
fn sandboxed_session() {
    let o = Options::new()
        .input_format(InputFormat::StreamJson)
        .include_partial_messages(true)
        .model("sonnet")
        .restricted(true)
        .permission_mode(PermissionMode::DontAsk)
        .permission_prompts("none")
        .strict_mcp_config(true)
        .mcp_config(r#"{"mcpServers":{}}"#)
        .tools(["Read", "Grep"])
        .allowed_tools(["Read", "Grep"])
        .disallowed_tools(["Bash"])
        .disable_slash_commands(true)
        .add_dir("/work")
        .append_system_prompt("Be brief.")
        .name("guest: x")
        .setting_env("FOO", "1")
        .resume("s1");
    assert_eq!(
        argv(&o),
        [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--model",
            "sonnet",
            "--restricted",
            "--permission-mode",
            "dontAsk",
            "--permission-prompts",
            "none",
            "--strict-mcp-config",
            "--mcp-config",
            r#"{"mcpServers":{}}"#,
            "--tools",
            "Read,Grep",
            "--allowedTools",
            "Read,Grep",
            "--disallowedTools",
            "Bash",
            "--disable-slash-commands",
            "--add-dir",
            "/work",
            "--append-system-prompt",
            "Be brief.",
            "--name",
            "guest: x",
            "--settings",
            r#"{"env":{"FOO":"1"}}"#,
            "--resume=s1",
        ]
    );
}

/// A one-shot, tool-less text answer that loads nothing.
#[test]
fn lean_one_shot() {
    let o = Options::new()
        .output_format(OutputFormat::Text)
        .model("haiku")
        .tools(Vec::<String>::new())
        .strict_mcp_config(true)
        .setting_sources(Vec::<String>::new())
        .disable_slash_commands(true)
        .system_prompt("Summarize.")
        .no_session_persistence(true);
    assert_eq!(
        argv(&o),
        [
            "-p",
            "--output-format",
            "text",
            "--model",
            "haiku",
            "--strict-mcp-config",
            "--tools",
            "",
            "--disable-slash-commands",
            "--system-prompt",
            "Summarize.",
            "--setting-sources",
            "",
            "--no-session-persistence",
        ]
    );
}

/// A new session with a chosen id, a budget and host-answered permission
/// prompts.
#[test]
fn delegated_session() {
    let o = Options::new()
        .input_format(InputFormat::StreamJson)
        .model("opus")
        .permission_mode("bypassPermissions")
        .permission_prompts("host")
        .permission_prompt_tool("stdio")
        .session_id(SID)
        .max_budget_usd(2.5)
        .name("worker: build");
    assert_eq!(
        argv(&o),
        [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--model",
            "opus",
            "--permission-mode",
            "bypassPermissions",
            "--permission-prompts",
            "host",
            "--permission-prompt-tool",
            "stdio",
            "--name",
            "worker: build",
            "--session-id=0b5f0d6e-5d3a-4c55-9a3f-1f2e3d4c5b6a",
            "--max-budget-usd",
            "2.50",
        ]
    );
}

#[test]
fn to_argv_is_pure() {
    let o = Options::new().model("m").setting_env("A", "1");
    assert_eq!(o.to_argv(), o.to_argv());
    assert_eq!(o.clone().to_argv(), o.to_argv());
}

/// Every known `claude` flag the builder covers (those `claude --help`
/// 2.1.303 lists, plus the hidden `--permission-prompt-tool`): each one
/// renders.
#[test]
fn every_known_flag_renders() {
    let all = Options::new()
        .input_format(InputFormat::StreamJson)
        .include_partial_messages(true)
        .model("m")
        .effort("high")
        .restricted(true)
        .permission_mode(PermissionMode::DontAsk)
        .permission_prompts("host")
        .permission_prompt_tool("stdio")
        .strict_mcp_config(true)
        .mcp_config("{}")
        .tools(["Read"])
        .allowed_tools(["Read"])
        .disallowed_tools(["Agent"])
        .disable_slash_commands(true)
        .add_dir("/d")
        .system_prompt("s")
        .append_system_prompt("a")
        .name("n")
        .setting_sources(["user"])
        .setting("k", 1)
        .resume("r")
        .fork_session(true)
        .no_session_persistence(true)
        .max_budget_usd(1.0);
    let with_id = Options::new().session_id("i");
    let text = Options::new().output_format(OutputFormat::Text);
    let rendered: Vec<String> = [all, with_id, text]
        .iter()
        .flat_map(argv)
        .map(|a| a.split('=').next().unwrap_or_default().to_owned())
        .collect();
    for flag in [
        "-p",
        "--add-dir",
        "--allowedTools",
        "--append-system-prompt",
        "--disable-slash-commands",
        "--disallowedTools",
        "--effort",
        "--fork-session",
        "--include-partial-messages",
        "--input-format",
        "--max-budget-usd",
        "--mcp-config",
        "--model",
        "--name",
        "--no-session-persistence",
        "--output-format",
        "--permission-mode",
        "--permission-prompt-tool",
        "--permission-prompts",
        "--restricted",
        "--resume",
        "--session-id",
        "--setting-sources",
        "--settings",
        "--strict-mcp-config",
        "--system-prompt",
        "--tools",
        "--verbose",
    ] {
        assert!(rendered.iter().any(|a| a == flag), "{flag} not rendered");
    }
}
