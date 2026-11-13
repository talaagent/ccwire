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
    pub enum PermissionMode {
        /// Ask before each tool that needs permission.
        Default = "default",
        /// Accept file edits without asking.
        AcceptEdits = "acceptEdits",
        /// Plan only: read, don't change anything.
        Plan = "plan",
        /// Deny whatever isn't allowed instead of asking.
        DontAsk = "dontAsk",
        /// Skip every permission check.
        BypassPermissions = "bypassPermissions",
        /// Let the auto-mode classifier decide.
        Auto = "auto",
        /// In the schema; Claude Code 2.1.322 rejects it.
        Delegate = "delegate",
        /// In the schema; Claude Code 2.1.322 rejects it.
        Manual = "manual",
    }
}

string_enum! {
    /// The one value of the `disable*` keys that turn a feature off
    /// (`permissions.disableBypassPermissionsMode`, `disableAutoMode`,
    /// `disableDeepLinkRegistration`).
    pub enum DisableFlag {
        /// Turn it off.
        Disable = "disable",
    }
}

string_enum! {
    /// `browserExternalPageTools`.
    pub enum DisabledFlag {
        /// Turn the tools off.
        Disabled = "disabled",
    }
}

string_enum! {
    /// `autoUpdatesChannel`.
    pub enum UpdatesChannel {
        /// About a week old, skipping releases with major regressions.
        Stable = "stable",
        /// The most recent release.
        Latest = "latest",
        /// Release candidates (accepted by Claude Code 2.1.322, not in the
        /// schema).
        Rc = "rc",
    }
}

string_enum! {
    /// `effortLevel`, `maxEffortLevel` and `modelSettings.*.effortLevel`.
    /// `max` is valid only as a cap.
    pub enum EffortLevel {
        /// Low effort.
        Low = "low",
        /// Medium effort.
        Medium = "medium",
        /// High effort.
        High = "high",
        /// Extra-high effort.
        Xhigh = "xhigh",
        /// No cap (`maxEffortLevel` only).
        Max = "max",
    }
}

string_enum! {
    /// `forceLoginMethod`.
    pub enum LoginMethod {
        /// A claude.ai subscription.
        ClaudeAi = "claudeai",
        /// A Console (API billing) account.
        Console = "console",
        /// A Cloud gateway.
        Gateway = "gateway",
    }
}

string_enum! {
    /// `teammateMode`: how agent-team teammates are shown.
    pub enum TeammateMode {
        /// Pick one.
        Auto = "auto",
        /// In this process.
        InProcess = "in-process",
        /// tmux panes.
        Tmux = "tmux",
        /// iTerm2 panes.
        Iterm2 = "iterm2",
    }
}

string_enum! {
    /// `parentSettingsBehavior` and `managedSourcesBehavior`.
    pub enum SourcesBehavior {
        /// The highest source that sets a key supplies it.
        FirstWins = "first-wins",
        /// Every source contributes its keys.
        Merge = "merge",
    }
}

string_enum! {
    /// `defaultShell` and a command hook's `shell`.
    pub enum Shell {
        /// bash.
        Bash = "bash",
        /// PowerShell.
        Powershell = "powershell",
    }
}

string_enum! {
    /// `tui`.
    pub enum Tui {
        /// The fullscreen renderer.
        Fullscreen = "fullscreen",
        /// The default renderer.
        Default = "default",
    }
}

string_enum! {
    /// `viewMode`.
    pub enum ViewMode {
        /// The default view.
        Default = "default",
        /// Verbose output.
        Verbose = "verbose",
        /// Focus view.
        Focus = "focus",
    }
}

string_enum! {
    /// `editorMode`.
    pub enum EditorMode {
        /// Normal editing.
        Normal = "normal",
        /// Vim keys.
        Vim = "vim",
    }
}

string_enum! {
    /// `preferredNotifChannel`.
    pub enum NotifChannel {
        /// Pick one.
        Auto = "auto",
        /// The terminal bell.
        TerminalBell = "terminal_bell",
        /// iTerm2 notifications.
        Iterm2 = "iterm2",
        /// iTerm2 notifications and the bell.
        Iterm2WithBell = "iterm2_with_bell",
        /// kitty notifications.
        Kitty = "kitty",
        /// Ghostty notifications.
        Ghostty = "ghostty",
        /// No notifications.
        NotificationsDisabled = "notifications_disabled",
    }
}

string_enum! {
    /// `theme`. A custom theme's name is `Other`.
    pub enum Theme {
        /// Follow the terminal.
        Auto = "auto",
        /// Dark.
        Dark = "dark",
        /// Light.
        Light = "light",
        /// Dark, colorblind-friendly.
        DarkDaltonized = "dark-daltonized",
        /// Light, colorblind-friendly.
        LightDaltonized = "light-daltonized",
        /// Dark, ANSI colors only.
        DarkAnsi = "dark-ansi",
        /// Light, ANSI colors only.
        LightAnsi = "light-ansi",
    }
}

string_enum! {
    /// `askUserQuestionTimeout` and `dialogExpiry`.
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
    pub enum WorkflowSize {
        /// No guideline.
        Unrestricted = "unrestricted",
        /// Small workflows.
        Small = "small",
        /// Medium workflows.
        Medium = "medium",
        /// Large workflows.
        Large = "large",
    }
}

string_enum! {
    /// `diffTool`.
    pub enum DiffTool {
        /// Pick one.
        Auto = "auto",
        /// The terminal.
        Terminal = "terminal",
    }
}

string_enum! {
    /// A value of `skillOverrides`.
    pub enum SkillOverride {
        /// Listed and invocable.
        On = "on",
        /// Listed by name only.
        NameOnly = "name-only",
        /// Invocable by the user only.
        UserInvocableOnly = "user-invocable-only",
        /// Off.
        Off = "off",
    }
}

string_enum! {
    /// `spinnerVerbs.mode`.
    pub enum SpinnerVerbsMode {
        /// Add to the built-in verbs.
        Append = "append",
        /// Replace them.
        Replace = "replace",
    }
}

string_enum! {
    /// `worktree.baseRef`.
    pub enum WorktreeBaseRef {
        /// A fresh checkout of the default branch.
        Fresh = "fresh",
        /// The current `HEAD`.
        Head = "head",
    }
}

string_enum! {
    /// `worktree.bgIsolation`.
    pub enum BgIsolation {
        /// Background agents get a worktree.
        Worktree = "worktree",
        /// No isolation.
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
    pub enum CredentialMode {
        /// Hide it from sandboxed commands.
        Deny = "deny",
        /// Show sandboxed commands a placeholder.
        Mask = "mask",
    }
}

string_enum! {
    /// `voice.mode`.
    pub enum VoiceMode {
        /// Hold to talk.
        Hold = "hold",
        /// Tap to start and stop.
        Tap = "tap",
    }
}

string_enum! {
    /// An entry of `strictPluginOnlyCustomization`.
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
    pub enum ModelMatch {
        /// An entry permits model IDs that extend it.
        Prefix = "prefix",
        /// An entry permits only the version it names.
        Exact = "exact",
    }
}

string_enum! {
    /// `promptCacheTtl` and `subagentPromptCacheTtl`.
    pub enum CacheTtl {
        /// Five minutes.
        Minutes5 = "5m",
        /// One hour.
        Hour1 = "1h",
    }
}

string_enum! {
    /// `keybindingFlavor`.
    pub enum KeybindingFlavor {
        /// Claude Code's own bindings.
        Classic = "classic",
        /// readline bindings.
        Readline = "readline",
    }
}

string_enum! {
    /// `timeFormat`. A strftime pattern (any value with a `%`) is `Other`.
    pub enum TimeFormat {
        /// Each time's built-in format.
        Auto = "auto",
        /// A 12-hour clock.
        Hour12 = "12-hour",
        /// A 24-hour clock.
        Hour24 = "24-hour",
        /// A 24-hour clock in UTC (`18:05Z`).
        Hour24Utc = "24-hour-utc",
    }
}

string_enum! {
    /// `crossSessionInbound`.
    pub enum InboundMode {
        /// Deliver messages from other sessions.
        Accept = "accept",
        /// Hold them for review.
        Hold = "hold",
        /// Drop them.
        Refuse = "refuse",
    }
}

string_enum! {
    /// `feedbackDrafts`.
    pub enum FeedbackDrafts {
        /// Show a card when Claude queues a draft.
        Notify = "notify",
        /// Draft without a card.
        Quiet = "quiet",
        /// No drafts.
        Off = "off",
    }
}

string_enum! {
    /// An entry of `allowedProviders`.
    pub enum Provider {
        /// The Anthropic API.
        Anthropic = "anthropic",
        /// Amazon Bedrock.
        Bedrock = "bedrock",
        /// Google Cloud's Agent Platform (Vertex AI).
        Vertex = "vertex",
        /// Microsoft Foundry.
        Foundry = "foundry",
        /// Claude Platform on AWS.
        AnthropicAws = "anthropicAws",
        /// The Bedrock Mantle endpoint.
        Mantle = "mantle",
        /// An API on another host (an LLM gateway).
        CustomEndpoint = "customEndpoint",
        /// A Cloud gateway sign-in.
        Gateway = "gateway",
    }
}

string_enum! {
    /// `spellcheck.checker`.
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
    pub enum HookEvent {
        /// Before a tool runs.
        PreToolUse = "PreToolUse",
        /// After a tool succeeds.
        PostToolUse = "PostToolUse",
        /// After a tool fails.
        PostToolUseFailure = "PostToolUseFailure",
        /// After a batch of tool calls.
        PostToolBatch = "PostToolBatch",
        /// When a permission dialog would show.
        PermissionRequest = "PermissionRequest",
        /// When the auto-mode classifier denies a tool call.
        PermissionDenied = "PermissionDenied",
        /// On a notification.
        Notification = "Notification",
        /// When the user submits a prompt.
        UserPromptSubmit = "UserPromptSubmit",
        /// When a typed command expands into a prompt.
        UserPromptExpansion = "UserPromptExpansion",
        /// When the agent finishes responding (not on interrupt).
        Stop = "Stop",
        /// When a turn ends on an API error.
        StopFailure = "StopFailure",
        /// When a subagent starts.
        SubagentStart = "SubagentStart",
        /// When a subagent stops.
        SubagentStop = "SubagentStop",
        /// Before compaction.
        PreCompact = "PreCompact",
        /// After compaction.
        PostCompact = "PostCompact",
        /// When an MCP server asks for input.
        Elicitation = "Elicitation",
        /// When the user answers an elicitation.
        ElicitationResult = "ElicitationResult",
        /// When an agent-team teammate is about to go idle.
        TeammateIdle = "TeammateIdle",
        /// When a task is created.
        TaskCreated = "TaskCreated",
        /// When a task completes.
        TaskCompleted = "TaskCompleted",
        /// On repository initialization (`--init`) or maintenance (undocumented).
        Setup = "Setup",
        /// When an instructions file loads.
        InstructionsLoaded = "InstructionsLoaded",
        /// When the working directory changes.
        CwdChanged = "CwdChanged",
        /// When a watched file changes.
        FileChanged = "FileChanged",
        /// When settings or skill files change during a session.
        ConfigChange = "ConfigChange",
        /// When a worktree is created.
        WorktreeCreate = "WorktreeCreate",
        /// When a worktree is removed.
        WorktreeRemove = "WorktreeRemove",
        /// When a session starts.
        SessionStart = "SessionStart",
        /// When a session ends.
        SessionEnd = "SessionEnd",
        /// Before the model switches.
        PreModelSwitch = "PreModelSwitch",
        /// After the model switches.
        PostModelSwitch = "PostModelSwitch",
        /// While assistant text is displayed.
        MessageDisplay = "MessageDisplay",
        /// After `/add-dir` registers a working directory (undocumented).
        DirectoryAdded = "DirectoryAdded",
    }
}

string_enum! {
    /// A hook's `onFailure`.
    pub enum HookOnFailure {
        /// The action goes ahead.
        Continue = "continue",
        /// The action is blocked.
        Block = "block",
    }
}

string_enum! {
    /// A credential's `onExtractNoMatch`.
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
    pub enum TokenDecode {
        /// JSON Web Tokens.
        Jwt = "jwt",
    }
}

string_enum! {
    /// A value of `sandbox.credentials.sigv4`.
    pub enum Sigv4Action {
        /// Fail the request.
        Deny = "deny",
        /// Forward it signed with the placeholder.
        Passthrough = "passthrough",
    }
}
