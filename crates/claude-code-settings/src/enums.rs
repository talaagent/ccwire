//! The string enums. Each has an `Other` variant that keeps a value this
//! crate doesn't know (one a newer Claude Code added) as it was written.

use alloc::borrow::ToOwned;
use alloc::string::String;
use core::fmt;

/// Defines a string enum: its variants, their JSON spellings and an
/// `Other(String)` for the rest, with `as_str`, `is_known`, `From<&str>`,
/// `Display` and serde as a plain string.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $($(#[$vmeta:meta])* $variant:ident = $value:literal,)*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub enum $name {
            $($(#[$vmeta])* $variant,)*
            /// A value this crate doesn't know, as written.
            Other(String),
        }

        impl $name {
            /// The value as Claude Code spells it.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $value,)*
                    Self::Other(s) => s,
                }
            }

            /// Whether this is a value this crate knows (not `Other`).
            pub fn is_known(&self) -> bool {
                !matches!(self, Self::Other(_))
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                match s {
                    $($value => Self::$variant,)*
                    other => Self::Other(other.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                match Self::from(s.as_str()) {
                    Self::Other(_) => Self::Other(s),
                    known => known,
                }
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                String::deserialize(d).map(Self::from)
            }
        }

        impl $crate::walk::Key for $name {
            fn key(&self) -> &str {
                self.as_str()
            }

            fn is_known(&self) -> bool {
                $name::is_known(self)
            }
        }
    };
}

string_enum! {
    /// `permissions.defaultMode`: the permission mode a session starts in.
    ///
    /// Docs: [`permissions.defaultMode`](https://code.claude.com/docs/en/settings-reference#permissions-defaultmode).
    pub enum PermissionMode {
        /// Claude Code runs only reads without asking.
        Default = "default",
        /// Claude Code also runs file edits and common filesystem commands such as `mkdir` and
        /// `mv` without asking.
        AcceptEdits = "acceptEdits",
        /// Claude Code reads and plans but blocks edits until you approve a plan.
        Plan = "plan",
        /// Claude Code auto-denies every call that would otherwise prompt, though a
        /// [mod](https://code.claude.com/docs/en/plugins/mods/overview) you installed can still
        /// approve [some of those
        /// calls](https://code.claude.com/docs/en/permissions#precedence-among-rules-hooks-and-mods).
        /// File reads inside your working directories, other actions that need no approval, and
        /// pre-approved tools still run, apart from reads from [network
        /// paths](https://code.claude.com/docs/en/permissions#network-paths).
        DontAsk = "dontAsk",
        /// Claude Code runs everything without asking.
        BypassPermissions = "bypassPermissions",
        /// Claude Code runs without routine prompts; before actions such as shell commands and
        /// network requests run, a background classifier checks that they align with your
        /// request.
        Auto = "auto",
        /// In the schema; Claude Code 2.1.322 rejects it.
        Delegate = "delegate",
        /// An alias for `"default"`.
        Manual = "manual",
    }
}

string_enum! {
    /// The one value of the `disable*` keys that turn a feature off
    /// (`permissions.disableBypassPermissionsMode`, `disableAutoMode`,
    /// `disableDeepLinkRegistration`).
    ///
    /// Docs: [`permissions.disableBypassPermissionsMode`](https://code.claude.com/docs/en/settings-reference#permissions-disablebypasspermissionsmode).
    pub enum DisableFlag {
        /// Turn it off.
        Disable = "disable",
    }
}

string_enum! {
    /// `browserExternalPageTools`.
    ///
    /// Docs: [`browserExternalPageTools`](https://code.claude.com/docs/en/settings-reference#browserexternalpagetools).
    pub enum DisabledFlag {
        /// Turn the tools off.
        Disabled = "disabled",
    }
}

string_enum! {
    /// `autoUpdatesChannel`.
    ///
    /// Docs: [`autoUpdatesChannel`](https://code.claude.com/docs/en/settings-reference#autoupdateschannel).
    pub enum UpdatesChannel {
        /// Updates follow a version that is typically about one week old and skips releases
        /// with major regressions.
        Stable = "stable",
        /// Updates follow the most recent release.
        Latest = "latest",
        /// Release candidates (accepted by Claude Code 2.1.322, not in the
        /// schema).
        Rc = "rc",
    }
}

string_enum! {
    /// `effortLevel`, `maxEffortLevel` and `modelSettings.*.effortLevel`.
    /// `max` is valid only as a cap.
    ///
    /// Docs: [`effortLevel`](https://code.claude.com/docs/en/settings-reference#effortlevel).
    pub enum EffortLevel {
        /// The least reasoning, for short, scoped, latency-sensitive tasks that aren't
        /// intelligence-sensitive.
        Low = "low",
        /// Reduces token usage for cost-sensitive work that can trade off some intelligence.
        Medium = "medium",
        /// Balances token usage and intelligence.
        High = "high",
        /// Deeper reasoning at higher token spend.
        Xhigh = "xhigh",
        /// No cap (`maxEffortLevel` only).
        Max = "max",
    }
}

string_enum! {
    /// `forceLoginMethod`.
    ///
    /// Docs: [`forceLoginMethod`](https://code.claude.com/docs/en/settings-reference#forceloginmethod).
    pub enum LoginMethod {
        /// Claude.ai sign-in.
        ClaudeAi = "claudeai",
        /// Claude Console sign-in.
        Console = "console",
        /// Claude Code sends people to a cloud gateway instead of a first-party login.
        Gateway = "gateway",
    }
}

string_enum! {
    /// `teammateMode`: how agent-team teammates are shown.
    ///
    /// Docs: [`teammateMode`](https://code.claude.com/docs/en/settings-reference#teammatemode).
    pub enum TeammateMode {
        /// Split panes when you're running inside tmux, or inside iTerm2 with `it2` on your
        /// `PATH` or tmux installed; in-process otherwise.
        Auto = "auto",
        /// Teammates run inside your main terminal pane.
        InProcess = "in-process",
        /// Split panes using tmux or iTerm2, detected from your terminal.
        Tmux = "tmux",
        /// ITerm2 native split panes through the `it2` CLI.
        Iterm2 = "iterm2",
    }
}

string_enum! {
    /// `parentSettingsBehavior` and `managedSourcesBehavior`.
    ///
    /// Docs: [`parentSettingsBehavior`](https://code.claude.com/docs/en/settings-reference#parentsettingsbehavior).
    pub enum SourcesBehavior {
        /// Claude Code drops the host-supplied settings when an admin-deployed managed tier is
        /// present.
        FirstWins = "first-wins",
        /// Claude Code applies the host-supplied settings under the admin tier through a
        /// restrictive-only filter.
        Merge = "merge",
    }
}

string_enum! {
    /// `defaultShell` and a command hook's `shell`.
    ///
    /// Docs: [`defaultShell`](https://code.claude.com/docs/en/settings-reference#defaultshell).
    pub enum Shell {
        /// Claude Code runs your `!` commands in Bash.
        Bash = "bash",
        /// Claude Code runs your `!` commands in PowerShell.
        Powershell = "powershell",
    }
}

string_enum! {
    /// `tui`.
    ///
    /// Docs: [`tui`](https://code.claude.com/docs/en/settings-reference#tui).
    pub enum Tui {
        /// The flicker-free alt-screen renderer with virtualized scrollback.
        Fullscreen = "fullscreen",
        /// The classic main-screen renderer.
        Default = "default",
    }
}

string_enum! {
    /// `viewMode`.
    ///
    /// Docs: [`viewMode`](https://code.claude.com/docs/en/settings-reference#viewmode).
    pub enum ViewMode {
        /// The normal transcript with truncated tool output.
        Default = "default",
        /// The transcript with full tool output.
        Verbose = "verbose",
        /// Only your last prompt, a one-line summary of tool calls with edit diffstats, and the
        /// final response. Focus view needs the [fullscreen
        /// renderer](https://code.claude.com/docs/en/settings-reference#tui).
        Focus = "focus",
    }
}

string_enum! {
    /// `editorMode`.
    ///
    /// Docs: [`editorMode`](https://code.claude.com/docs/en/settings-reference#editormode).
    pub enum EditorMode {
        /// Standard key bindings in the prompt input.
        Normal = "normal",
        /// Vim-style editing with NORMAL, INSERT, and VISUAL modes.
        Vim = "vim",
    }
}

string_enum! {
    /// `preferredNotifChannel`.
    ///
    /// Docs: [`preferredNotifChannel`](https://code.claude.com/docs/en/settings-reference#preferrednotifchannel).
    pub enum NotifChannel {
        /// Claude Code sends a desktop notification when it detects iTerm2, Ghostty, or Kitty,
        /// rings the bell in Terminal.app only when its audible bell is off, and does nothing
        /// elsewhere.
        Auto = "auto",
        /// Claude Code rings the bell character in any terminal.
        TerminalBell = "terminal_bell",
        /// Claude Code sends an iTerm2 desktop notification.
        Iterm2 = "iterm2",
        /// Claude Code sends an iTerm2 desktop notification and rings the bell.
        Iterm2WithBell = "iterm2_with_bell",
        /// Claude Code sends a Kitty desktop notification.
        Kitty = "kitty",
        /// Claude Code sends a Ghostty desktop notification.
        Ghostty = "ghostty",
        /// Claude Code sends no notification.
        NotificationsDisabled = "notifications_disabled",
    }
}

string_enum! {
    /// `theme`. A custom theme's name is `Other`.
    ///
    /// Docs: [`theme`](https://code.claude.com/docs/en/settings-reference#theme).
    pub enum Theme {
        /// Matches your terminal's light or dark background.
        Auto = "auto",
        /// The dark theme.
        Dark = "dark",
        /// The light theme.
        Light = "light",
        /// The dark theme with colorblind-friendly colors.
        DarkDaltonized = "dark-daltonized",
        /// The light theme with colorblind-friendly colors.
        LightDaltonized = "light-daltonized",
        /// The dark theme using only your terminal's ANSI color palette.
        DarkAnsi = "dark-ansi",
        /// The light theme using only your terminal's ANSI color palette.
        LightAnsi = "light-ansi",
    }
}

string_enum! {
    /// `askUserQuestionTimeout` and `dialogExpiry`.
    ///
    /// Docs: [`askUserQuestionTimeout`](https://code.claude.com/docs/en/settings-reference#askuserquestiontimeout).
    pub enum DialogTimeout {
        /// 60 seconds.
        Seconds60 = "60s",
        /// 5 minutes.
        Minutes5 = "5m",
        /// 10 minutes.
        Minutes10 = "10m",
        /// No deadline.
        Never = "never",
    }
}

string_enum! {
    /// `workflowSizeGuideline`.
    ///
    /// Docs: [`workflowSizeGuideline`](https://code.claude.com/docs/en/settings-reference#workflowsizeguideline).
    pub enum WorkflowSize {
        /// No guideline, so Claude sizes the workflow to the task.
        Unrestricted = "unrestricted",
        /// Claude aims for fewer than 5 agents.
        Small = "small",
        /// Claude aims for fewer than 10 agents.
        Medium = "medium",
        /// Claude aims for fewer than 50 agents.
        Large = "large",
    }
}

string_enum! {
    /// `diffTool`.
    ///
    /// Docs: [`diffTool`](https://code.claude.com/docs/en/settings-reference#difftool).
    pub enum DiffTool {
        /// Claude Code opens the diff in the IDE's diff viewer when a VS Code or JetBrains IDE
        /// is connected.
        Auto = "auto",
        /// Claude Code keeps the diff in the terminal.
        Terminal = "terminal",
    }
}

string_enum! {
    /// A value of `skillOverrides`.
    ///
    /// Docs: [`skillOverrides`](https://code.claude.com/docs/en/settings-reference#skilloverrides).
    pub enum SkillOverride {
        /// Claude sees the skill and you can type `/name`.
        On = "on",
        /// Claude sees the skill by name without its description.
        NameOnly = "name-only",
        /// Claude doesn't see the skill, but you can still type `/name`.
        UserInvocableOnly = "user-invocable-only",
        /// Claude doesn't see the skill and `/name` is hidden from autocomplete.
        Off = "off",
    }
}

string_enum! {
    /// `spinnerVerbs.mode`.
    ///
    /// Docs: [`spinnerVerbs`](https://code.claude.com/docs/en/settings-reference#spinnerverbs).
    pub enum SpinnerVerbsMode {
        /// Claude Code adds your verbs to the built-in set.
        Append = "append",
        /// Claude Code shows only your verbs.
        Replace = "replace",
    }
}

string_enum! {
    /// `worktree.baseRef`.
    ///
    /// Docs: [`worktree.baseRef`](https://code.claude.com/docs/en/settings-reference#worktree-baseref).
    pub enum WorktreeBaseRef {
        /// New worktrees branch from `origin/<default-branch>`.
        Fresh = "fresh",
        /// New worktrees branch from your current local `HEAD`, including unpushed commits.
        Head = "head",
    }
}

string_enum! {
    /// `worktree.bgIsolation`.
    ///
    /// Docs: [`worktree.bgIsolation`](https://code.claude.com/docs/en/settings-reference#worktree-bgisolation).
    pub enum BgIsolation {
        /// Claude Code blocks `Edit` and `Write` in the main checkout until the session calls
        /// `EnterWorktree`.
        Worktree = "worktree",
        /// Background jobs edit the working copy directly.
        None = "none",
    }
}

string_enum! {
    /// `sandbox.enabledPlatforms`.
    pub enum Platform {
        /// macOS.
        Macos = "macos",
        /// Linux.
        Linux = "linux",
        /// WSL.
        Wsl = "wsl",
        /// Windows.
        Windows = "windows",
    }
}

string_enum! {
    /// `sandbox.credentials.*.mode`.
    ///
    /// Docs: [`sandbox.credentials.files`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-files).
    pub enum CredentialMode {
        /// Hide it from sandboxed commands.
        Deny = "deny",
        /// Show sandboxed commands a placeholder.
        Mask = "mask",
    }
}

string_enum! {
    /// `voice.mode`.
    ///
    /// Docs: [`voice`](https://code.claude.com/docs/en/settings-reference#voice).
    pub enum VoiceMode {
        /// You hold the dictation key while speaking and release it to stop.
        Hold = "hold",
        /// You tap the key once to start recording and again to send.
        Tap = "tap",
    }
}

string_enum! {
    /// An entry of `strictPluginOnlyCustomization`.
    ///
    /// Docs: [`strictPluginOnlyCustomization`](https://code.claude.com/docs/en/settings-reference#strictpluginonlycustomization).
    pub enum CustomizationKind {
        /// Skills.
        Skills = "skills",
        /// Agents.
        Agents = "agents",
        /// Hooks.
        Hooks = "hooks",
        /// MCP servers.
        Mcp = "mcp",
    }
}

string_enum! {
    /// `availableModelsMatch`.
    ///
    /// Docs: [`availableModelsMatch`](https://code.claude.com/docs/en/settings-reference#availablemodelsmatch).
    pub enum ModelMatch {
        /// A model ID entry permits its version and any model ID that extends it with another
        /// segment.
        Prefix = "prefix",
        /// A model ID entry permits only the version it names, so `"claude-opus-5"` permits
        /// Opus 5 but not `claude-opus-5-5`. An entry such as `"claude-opus-4-5"` also permits
        /// its dated Anthropic model ID `claude-opus-4-5-20251101`. A family alias such as
        /// `"opus"` still permits the whole family, and `best`, `opusplan`, and `default`
        /// entries are ignored.
        Exact = "exact",
    }
}

string_enum! {
    /// `promptCacheTtl` and `subagentPromptCacheTtl`.
    ///
    /// Docs: [`promptCacheTtl`](https://code.claude.com/docs/en/settings-reference#promptcachettl).
    pub enum CacheTtl {
        /// The cache holds for five minutes.
        Minutes5 = "5m",
        /// The cache holds for an hour.
        Hour1 = "1h",
    }
}

string_enum! {
    /// `keybindingFlavor`.
    ///
    /// Docs: [`keybindingFlavor`](https://code.claude.com/docs/en/settings-reference#keybindingflavor).
    pub enum KeybindingFlavor {
        /// Claude Code's own bindings.
        Classic = "classic",
        /// readline bindings.
        Readline = "readline",
    }
}

string_enum! {
    /// `timeFormat`. A strftime pattern (any value with a `%`) is `Other`.
    ///
    /// Docs: [`timeFormat`](https://code.claude.com/docs/en/settings-reference#timeformat).
    pub enum TimeFormat {
        /// The same as unset; each time keeps its built-in format, which follows your locale on
        /// the turn duration message.
        Auto = "auto",
        /// A 12-hour clock.
        Hour12 = "12-hour",
        /// A 24-hour clock.
        Hour24 = "24-hour",
        /// A 24-hour clock in UTC with `Z` after the minutes, such as `18:05Z`; Claude Code
        /// ignores [`timeZone`](https://code.claude.com/docs/en/settings-reference#timezone)
        /// for this preset.
        Hour24Utc = "24-hour-utc",
    }
}

string_enum! {
    /// `crossSessionInbound`.
    ///
    /// Docs: [`crossSessionInbound`](https://code.claude.com/docs/en/settings-reference#crosssessioninbound).
    pub enum InboundMode {
        /// Claude Code delivers the message to Claude.
        Accept = "accept",
        /// Claude Code shows a notice for the message without delivering it.
        Hold = "hold",
        /// Claude Code drops the message.
        Refuse = "refuse",
    }
}

string_enum! {
    /// `feedbackDrafts`.
    ///
    /// Docs: [`feedbackDrafts`](https://code.claude.com/docs/en/settings-reference#feedbackdrafts).
    pub enum FeedbackDrafts {
        /// Claude Code shows a card above the prompt when Claude queues a draft, up to [three
        /// cards in a
        /// session](https://code.claude.com/docs/en/tools-reference#what-you-see-when-claude-drafts)
        /// by default.
        Notify = "notify",
        /// Claude drafts without a card. You see the count of queued drafts in the prompt
        /// footer and review them in `/feedback`.
        Quiet = "quiet",
        /// Claude Code removes the SendFeedback tool, so Claude can't queue drafts.
        Off = "off",
    }
}

string_enum! {
    /// An entry of `allowedProviders`.
    ///
    /// Docs: [`allowedProviders`](https://code.claude.com/docs/en/settings-reference#allowedproviders).
    pub enum Provider {
        /// The Anthropic API on Anthropic's own host, through a claude.ai or Console sign-in or
        /// an API key. Pair it with
        /// [`forceLoginMethod`](https://code.claude.com/docs/en/settings-reference#forceloginmethod)
        /// or
        /// [`forceLoginOrgUUID`](https://code.claude.com/docs/en/settings-reference#forceloginorguuid)
        /// to also restrict the sign-in.
        Anthropic = "anthropic",
        /// [Amazon Bedrock](https://code.claude.com/docs/en/amazon-bedrock).
        Bedrock = "bedrock",
        /// [Google Cloud's Agent Platform](https://code.claude.com/docs/en/google-vertex-ai),
        /// formerly Vertex AI.
        Vertex = "vertex",
        /// [Microsoft Foundry](https://code.claude.com/docs/en/microsoft-foundry).
        Foundry = "foundry",
        /// [Claude Platform on AWS](https://code.claude.com/docs/en/claude-platform-on-aws).
        AnthropicAws = "anthropicAws",
        /// The Amazon Bedrock [Mantle
        /// endpoint](https://code.claude.com/docs/en/amazon-bedrock#use-the-mantle-endpoint). A
        /// session that [runs Mantle alongside the Invoke
        /// API](https://code.claude.com/docs/en/amazon-bedrock#run-mantle-alongside-the-invoke-api)
        /// uses both providers, so list `"bedrock"` and `"mantle"` together for it.
        Mantle = "mantle",
        /// The Anthropic API or a cloud provider's API sent to another host, such as an [LLM
        /// gateway](https://code.claude.com/docs/en/llm-gateway) named by `ANTHROPIC_BASE_URL`
        /// or by a provider's endpoint variable such as `ANTHROPIC_BEDROCK_BASE_URL`. Claude
        /// Code admits it only for the exact value a managed
        /// [`env`](https://code.claude.com/docs/en/settings-reference#env) block pins.
        CustomEndpoint = "customEndpoint",
        /// A [Cloud gateway](https://code.claude.com/docs/en/claude-apps-gateway) sign-in.
        Gateway = "gateway",
    }
}

string_enum! {
    /// `spellcheck.checker`.
    ///
    /// Docs: [`spellcheck`](https://code.claude.com/docs/en/settings-reference#spellcheck).
    pub enum SpellChecker {
        /// The first one found on `PATH`.
        Auto = "auto",
        /// aspell.
        Aspell = "aspell",
        /// hunspell.
        Hunspell = "hunspell",
        /// ispell.
        Ispell = "ispell",
    }
}

string_enum! {
    /// A marketplace or plugin source's `source`.
    ///
    /// Docs: [`extraKnownMarketplaces`](https://code.claude.com/docs/en/settings-reference#extraknownmarketplaces).
    pub enum SourceKind {
        /// A URL to a marketplace file.
        Url = "url",
        /// Any marketplace on matching hosts.
        HostPattern = "hostPattern",
        /// Any marketplace at matching paths.
        PathPattern = "pathPattern",
        /// A GitHub repository.
        Github = "github",
        /// A git repository.
        Git = "git",
        /// A subdirectory of a git repository (plugins).
        GitSubdir = "git-subdir",
        /// An npm package.
        Npm = "npm",
        /// A local marketplace file.
        File = "file",
        /// A local directory.
        Directory = "directory",
        /// A marketplace declared inline in settings.
        Settings = "settings",
        /// The skills directory.
        SkillsDir = "skills-dir",
    }
}

string_enum! {
    /// A hook event: a key of `hooks`.
    ///
    /// Docs: [settings reference](https://code.claude.com/docs/en/hooks#hook-events).
    pub enum HookEvent {
        /// Runs after Claude creates tool parameters and before processing the tool call.
        ///
        /// Docs: [`PreToolUse`](https://code.claude.com/docs/en/hooks#pretooluse).
        PreToolUse = "PreToolUse",
        /// Runs immediately after a tool completes successfully.
        ///
        /// Docs: [`PostToolUse`](https://code.claude.com/docs/en/hooks#posttooluse).
        PostToolUse = "PostToolUse",
        /// Runs when a tool that started executing fails: the tool threw an error, or an MCP
        /// tool returned an error result.
        ///
        /// Docs: [`PostToolUseFailure`](https://code.claude.com/docs/en/hooks#posttoolusefailure).
        PostToolUseFailure = "PostToolUseFailure",
        /// Runs once after every tool call in a batch has resolved, before Claude Code sends
        /// the next request to the model.
        ///
        /// Docs: [`PostToolBatch`](https://code.claude.com/docs/en/hooks#posttoolbatch).
        PostToolBatch = "PostToolBatch",
        /// Runs when Claude Code is about to ask you for permission to use a tool.
        ///
        /// Docs: [`PermissionRequest`](https://code.claude.com/docs/en/hooks#permissionrequest).
        PermissionRequest = "PermissionRequest",
        /// Runs when [auto
        /// mode](https://code.claude.com/docs/en/permission-modes#eliminate-prompts-with-auto-mode)
        /// denies a tool call, including when it denies without a classifier verdict because [a
        /// safety check separate from auto mode refused the classifier's own
        /// request](https://code.claude.com/docs/en/errors#auto-mode-cannot-determine-the-safety-of-an-action)
        /// or its response didn't parse.
        ///
        /// Docs: [`PermissionDenied`](https://code.claude.com/docs/en/hooks#permissiondenied).
        PermissionDenied = "PermissionDenied",
        /// Runs when Claude Code sends notifications.
        ///
        /// Docs: [`Notification`](https://code.claude.com/docs/en/hooks#notification).
        Notification = "Notification",
        /// Runs when a prompt is submitted, before Claude processes it.
        ///
        /// Docs: [`UserPromptSubmit`](https://code.claude.com/docs/en/hooks#userpromptsubmit).
        UserPromptSubmit = "UserPromptSubmit",
        /// Runs when a user-typed command expands into a prompt before reaching Claude.
        ///
        /// Docs: [`UserPromptExpansion`](https://code.claude.com/docs/en/hooks#userpromptexpansion).
        UserPromptExpansion = "UserPromptExpansion",
        /// Runs when the main Claude Code agent has finished responding.
        ///
        /// Docs: [`Stop`](https://code.claude.com/docs/en/hooks#stop).
        Stop = "Stop",
        /// Runs instead of [Stop](https://code.claude.com/docs/en/hooks#stop) when the turn
        /// ends due to an API error.
        ///
        /// Docs: [`StopFailure`](https://code.claude.com/docs/en/hooks#stopfailure).
        StopFailure = "StopFailure",
        /// Runs when Claude spawns a subagent with the Agent tool, when Claude [resumes a
        /// subagent](https://code.claude.com/docs/en/sub-agents#resume-subagents), and each
        /// time an in-process [agent team](https://code.claude.com/docs/en/agent-teams)
        /// teammate handles a new message.
        ///
        /// Docs: [`SubagentStart`](https://code.claude.com/docs/en/hooks#subagentstart).
        SubagentStart = "SubagentStart",
        /// Runs when a Claude Code subagent has finished responding.
        ///
        /// Docs: [`SubagentStop`](https://code.claude.com/docs/en/hooks#subagentstop).
        SubagentStop = "SubagentStop",
        /// Runs before Claude Code is about to run a compact operation.
        ///
        /// Docs: [`PreCompact`](https://code.claude.com/docs/en/hooks#precompact).
        PreCompact = "PreCompact",
        /// Runs after Claude Code completes a compact operation.
        ///
        /// Docs: [`PostCompact`](https://code.claude.com/docs/en/hooks#postcompact).
        PostCompact = "PostCompact",
        /// Runs when an MCP server requests user input mid-task.
        ///
        /// Docs: [`Elicitation`](https://code.claude.com/docs/en/hooks#elicitation).
        Elicitation = "Elicitation",
        /// Runs after a user responds to an MCP elicitation.
        ///
        /// Docs: [`ElicitationResult`](https://code.claude.com/docs/en/hooks#elicitationresult).
        ElicitationResult = "ElicitationResult",
        /// Runs when an [agent team](https://code.claude.com/docs/en/agent-teams) teammate is
        /// about to go idle after finishing its turn.
        ///
        /// Docs: [`TeammateIdle`](https://code.claude.com/docs/en/hooks#teammateidle).
        TeammateIdle = "TeammateIdle",
        /// Runs when a task is being created via the `TaskCreate` tool.
        ///
        /// Docs: [`TaskCreated`](https://code.claude.com/docs/en/hooks#taskcreated).
        TaskCreated = "TaskCreated",
        /// Runs when a task is being marked as completed.
        ///
        /// Docs: [`TaskCompleted`](https://code.claude.com/docs/en/hooks#taskcompleted).
        TaskCompleted = "TaskCompleted",
        /// Fires only when you launch Claude Code with `--init-only`, or with `--init` or
        /// `--maintenance` in [non-interactive mode](https://code.claude.com/docs/en/headless)
        /// with the `-p` flag.
        ///
        /// Docs: [`Setup`](https://code.claude.com/docs/en/hooks#setup).
        Setup = "Setup",
        /// Fires when a `CLAUDE.md` or `.claude/rules/*.md` file is loaded into context.
        ///
        /// Docs: [`InstructionsLoaded`](https://code.claude.com/docs/en/hooks#instructionsloaded).
        InstructionsLoaded = "InstructionsLoaded",
        /// Runs when a shell command in the main conversation changes the working directory,
        /// for example when Claude executes a `cd` command.
        ///
        /// Docs: [`CwdChanged`](https://code.claude.com/docs/en/hooks#cwdchanged).
        CwdChanged = "CwdChanged",
        /// Runs when a watched file changes on disk.
        ///
        /// Docs: [`FileChanged`](https://code.claude.com/docs/en/hooks#filechanged).
        FileChanged = "FileChanged",
        /// Runs when a configuration file changes during a session.
        ///
        /// Docs: [`ConfigChange`](https://code.claude.com/docs/en/hooks#configchange).
        ConfigChange = "ConfigChange",
        /// Runs when Claude Code creates a worktree in cases such as these:
        ///
        /// Docs: [`WorktreeCreate`](https://code.claude.com/docs/en/hooks#worktreecreate).
        WorktreeCreate = "WorktreeCreate",
        /// Runs when Claude Code cleans up a worktree that your
        /// [`WorktreeCreate`](https://code.claude.com/docs/en/hooks#worktreecreate) hook
        /// created.
        ///
        /// Docs: [`WorktreeRemove`](https://code.claude.com/docs/en/hooks#worktreeremove).
        WorktreeRemove = "WorktreeRemove",
        /// Runs when Claude Code starts a new session or resumes an existing session.
        ///
        /// Docs: [`SessionStart`](https://code.claude.com/docs/en/hooks#sessionstart).
        SessionStart = "SessionStart",
        /// Runs when a Claude Code session ends.
        ///
        /// Docs: [`SessionEnd`](https://code.claude.com/docs/en/hooks#sessionend).
        SessionEnd = "SessionEnd",
        /// Runs before Claude Code applies a model switch that you or a client requested.
        ///
        /// Docs: [`PreModelSwitch`](https://code.claude.com/docs/en/hooks#premodelswitch).
        PreModelSwitch = "PreModelSwitch",
        /// Runs after the session's model changes.
        ///
        /// Docs: [`PostModelSwitch`](https://code.claude.com/docs/en/hooks#postmodelswitch).
        PostModelSwitch = "PostModelSwitch",
        /// Runs while an assistant message streams to the screen.
        ///
        /// Docs: [`MessageDisplay`](https://code.claude.com/docs/en/hooks#messagedisplay).
        MessageDisplay = "MessageDisplay",
        /// Runs after you add a working directory mid-session with the `/add-dir` command, or
        /// after an SDK client adds one with the `register_repo_root` control request.
        ///
        /// Docs: [`DirectoryAdded`](https://code.claude.com/docs/en/hooks#directoryadded).
        DirectoryAdded = "DirectoryAdded",
    }
}

string_enum! {
    /// A hook's `onFailure`.
    ///
    /// Docs: [`hook.command.onFailure`](https://code.claude.com/docs/en/hooks#command-hook-fields).
    pub enum HookOnFailure {
        /// The action goes ahead.
        Continue = "continue",
        /// The action is blocked.
        Block = "block",
    }
}

string_enum! {
    /// A credential's `onExtractNoMatch`.
    ///
    /// Docs: [`sandbox.credentials.files.onExtractNoMatch`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files).
    pub enum ExtractNoMatch {
        /// Leave it unmasked.
        Warn = "warn",
        /// Hide it.
        Deny = "deny",
        /// Stop sandbox setup.
        Error = "error",
    }
}

string_enum! {
    /// A credential's `decode`.
    ///
    /// Docs: [`sandbox.credentials.files.decode`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files).
    pub enum TokenDecode {
        /// JSON Web Tokens.
        Jwt = "jwt",
    }
}

string_enum! {
    /// A value of `sandbox.credentials.sigv4`.
    ///
    /// Docs: [`sandbox.credentials.sigv4`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-sigv4).
    pub enum Sigv4Action {
        /// The proxy fails the request.
        Deny = "deny",
        /// The proxy forwards the request signed with the masked placeholder, so the tool
        /// receives AWS's own rejection.
        Passthrough = "passthrough",
    }
}
