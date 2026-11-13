//! `sandbox`: the sandboxed Bash environment.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::Extra;
use crate::enums::{CredentialMode, ExtractNoMatch, Platform, Sigv4Action, TokenDecode};
use crate::walk::walk;

/// `sandbox`.
///
/// Isolate the Bash commands Claude runs from your filesystem and network with
/// [sandboxing](https://code.claude.com/docs/en/sandboxing). Turn the sandbox on with
/// `enabled`, then narrow or widen what sandboxed commands can touch with the `filesystem`,
/// `network`, and `credentials` sub-objects.
///
/// Docs: [`sandbox`](https://code.claude.com/docs/en/settings-reference#sandbox).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Sandbox {
    /// Turn on [sandboxing](https://code.claude.com/docs/en/sandboxing) for Bash commands. When
    /// you pick a mode in the `/sandbox` panel, Claude Code writes this key to
    /// `.claude/settings.local.json` for the current project; set it in
    /// `~/.claude/settings.json` to sandbox every project.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code sandboxes Bash commands
    ///   - `false`: Bash commands run unsandboxed
    /// - **Default**: `false`
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`enabled`](https://code.claude.com/docs/en/settings-reference#sandbox-enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Let Claude Code run sandboxed Bash commands without a permission prompt. Commands that
    /// can't run in the sandbox still go through the regular permission flow, and `deny` rules
    /// and content-scoped `ask` rules such as `Bash(git push *)` still apply; a bare `Bash` ask
    /// rule is skipped for sandboxed commands. Set it to `false` to send sandboxed commands
    /// through the regular permission flow too, which the `/sandbox` **Mode** tab calls regular
    /// permissions mode.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code runs sandboxed Bash commands without a permission prompt, except
    ///     in the cases that [Auto-allow
    ///     mode](https://code.claude.com/docs/en/sandboxing#auto-allow-mode) describes;
    ///     `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` turns auto-allow off
    ///   - `false`: sandboxed commands go through the regular permission flow, so your allow
    ///     rules and permission mode decide. The `/sandbox` **Mode** tab calls this regular
    ///     permissions mode
    /// - **Default**: `true`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`autoAllowBashIfSandboxed`](https://code.claude.com/docs/en/settings-reference#sandbox-autoallowbashifsandboxed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_allow_bash_if_sandboxed: Option<bool>,
    /// Name commands that Claude Code runs outside the sandbox, such as tools that don't work
    /// under it. Each entry uses the same syntax as the content of a `Bash(...)` [permission
    /// rule](https://code.claude.com/docs/en/permissions#permission-rule-syntax): an exact
    /// command, a prefix such as `docker *`, or a wildcard pattern. A pattern with no wildcard
    /// is an exact match, so `docker` matches only `docker` with no arguments.
    ///
    /// - **Values**: array of command patterns
    /// - **Default**: unset, so no command is excluded
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`excludedCommands`](https://code.claude.com/docs/en/settings-reference#sandbox-excludedcommands).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_commands: Option<Vec<String>>,
    /// Let Claude retry a command outside the sandbox with the `dangerouslyDisableSandbox`
    /// parameter after the sandbox blocks it. When it's `false`, Claude Code ignores that
    /// parameter. While the sandbox is running, commands Claude runs are then sandboxed unless
    /// they match an
    /// [`excludedCommands`](https://code.claude.com/docs/en/settings-reference#sandbox-excludedcommands)
    /// entry.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude can retry a command outside the sandbox with the
    ///     `dangerouslyDisableSandbox` parameter after the sandbox blocks it
    ///   - `false`: Claude Code ignores that parameter, so while the sandbox is running,
    ///     commands Claude runs are sandboxed unless they match an `excludedCommands` entry
    /// - **Default**: `true`
    /// - **Scope**: any settings file, with [a limit on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#turn-off-the-retry-with-strict-sandbox-mode).
    ///
    /// Docs: [`allowUnsandboxedCommands`](https://code.claude.com/docs/en/settings-reference#sandbox-allowunsandboxedcommands).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unsandboxed_commands: Option<bool>,
    /// Silence sandbox violation reports for paths you expect a command to probe and be
    /// refused, such as a tool that checks `/etc/hosts` on startup, so those denials don't show
    /// up as violations or in what Claude sees. The sandbox still blocks the access; only the
    /// report is suppressed. Keys are substrings to match against the command, with `*`
    /// matching every command, and values are substrings of the violation to ignore for that
    /// command, such as a filesystem path.
    ///
    /// - **Values**: object mapping a command substring to an array of violation substrings,
    ///   usually paths
    /// - **Default**: unset, so every violation is reported
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`ignoreViolations`](https://code.claude.com/docs/en/settings-reference#sandbox-ignoreviolations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_violations: Option<BTreeMap<String, Vec<String>>>,
    /// Control which hosts, ports, and sockets sandboxed commands can reach. The sandbox routes
    /// outbound traffic through a proxy that enforces these lists; see [Network
    /// isolation](https://code.claude.com/docs/en/sandboxing#network-isolation) for how the
    /// proxy decides and when it prompts.
    ///
    /// - **Values**: object with the sub-keys below
    /// - **Default**: unset, so no domains are pre-allowed and your permission mode decides
    ///   [what happens to each new
    ///   host](https://code.claude.com/docs/en/sandboxing#hosts-outside-your-allowed-domains)
    /// - **Scope**: any settings file. `strictAllowlist`, `allowManagedDomainsOnly`, and
    ///   `tlsTerminate` are read from fewer sources, as their entries say.
    ///
    /// Docs: [`network`](https://code.claude.com/docs/en/settings-reference#sandbox-network).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<SandboxNetwork>,
    /// Control which paths sandboxed commands can read and write. By default they can write to
    /// the working directory, the per-user temp directory, and directories you add with
    /// `--add-dir`, `/add-dir`, or `permissions.additionalDirectories`, and can read the rest
    /// of the filesystem, including credential files. Widen or narrow that with the four path
    /// lists, or switch the filesystem layer off with `disabled`.
    ///
    /// - **Values**: object with `allowWrite`, `denyWrite`, `denyRead`, and `allowRead` arrays,
    ///   plus the `allowManagedReadPathsOnly` and `disabled` Booleans
    /// - **Default**: unset, so the default read and write boundaries apply
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`filesystem`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<SandboxFilesystem>,
    /// Declare the credential files and environment variables to [protect from sandboxed
    /// commands](https://code.claude.com/docs/en/sandboxing#protect-credentials). Each entry
    /// names a file `path` or a variable `name` and a `mode`: `deny` hides the credential
    /// inside the sandbox, and `mask` shows sandboxed commands a placeholder while the [sandbox
    /// proxy](https://code.claude.com/docs/en/sandboxing#mask-credentials) substitutes the real
    /// value on outbound requests. Claude Code protects only the entries you list; there is no
    /// built-in credential deny list.
    ///
    /// - **Values**: object with `files`, `envVars`, `allowPlaintextInject`, `awsPairs`, and
    ///   `sigv4`
    /// - **Default**: unset, so no credentials are protected
    /// - **Scope**: any settings file. Claude Code honors `mask` entries,
    ///   `allowPlaintextInject`, `awsPairs`, and `sigv4` only from user settings, managed
    ///   settings, and the `--settings` flag.
    ///
    /// Docs: [`credentials`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<SandboxCredentials>,
    /// Point the sandbox at a ripgrep binary of your own instead of the one Claude Code uses,
    /// for example when your platform needs a differently built `rg`.
    ///
    /// - **Values**: object with `command`, the path to the ripgrep binary, and optional
    ///   `args`, an array of arguments to prepend
    /// - **Default**: unset, so the sandbox uses the same ripgrep binary as Claude Code. That
    ///   is the bundled binary unless you set
    ///   [`USE_BUILTIN_RIPGREP`](https://code.claude.com/docs/en/env-vars) to `0`
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`ripgrep`](https://code.claude.com/docs/en/settings-reference#sandbox-ripgrep).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ripgrep: Option<Ripgrep>,
    /// Let sandboxed commands on macOS reach the system TLS trust service,
    /// `com.apple.trustd.agent`. Go-based tools such as `gh`, `gcloud`, and `terraform` need it
    /// to verify TLS certificates when you use
    /// [`network.httpProxyPort`](https://code.claude.com/docs/en/settings-reference#sandbox-network-httpproxyport)
    /// with a MITM proxy and a custom CA. This reduces security by opening a potential data
    /// exfiltration path through the trust service.
    ///
    /// - **Values**: Boolean
    ///   - `true`: sandboxed commands on macOS can reach `com.apple.trustd.agent`
    ///   - `false`: sandboxed commands on macOS can't reach the system TLS trust service
    /// - **Default**: `false`
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`enableWeakerNetworkIsolation`](https://code.claude.com/docs/en/settings-reference#sandbox-enableweakernetworkisolation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_network_isolation: Option<bool>,
    /// Run the Linux sandbox inside an unprivileged Docker container, where bubblewrap can't
    /// mount a fresh `/proc`. Instead the inner sandbox bind-mounts the container's existing
    /// `/proc`, which exposes process information that a fresh mount would hide. This reduces
    /// security; use it only when the outer container already provides the isolation you need.
    ///
    /// - **Values**: Boolean
    ///   - `true`: the inner sandbox bind-mounts the container's existing `/proc` instead of
    ///     mounting a fresh one
    ///   - `false`: the sandbox mounts a fresh `/proc`, which doesn't work in an unprivileged
    ///     Docker container
    /// - **Default**: `false`
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`enableWeakerNestedSandbox`](https://code.claude.com/docs/en/settings-reference#sandbox-enableweakernestedsandbox).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_nested_sandbox: Option<bool>,
    /// Point the sandbox at a bubblewrap binary installed outside `PATH`, such as a vendored
    /// copy on an air-gapped host. Claude Code uses the path both for the startup dependency
    /// check and when it wraps each sandboxed command.
    ///
    /// - **Values**: string, an absolute path; Claude Code drops a relative path and falls back
    ///   to `PATH` lookup
    /// - **Default**: unset, so Claude Code finds `bwrap` on `PATH`
    /// - **Scope**: managed. Claude Code reads it only from managed settings so that a user,
    ///   project, or local file can't point the sandbox at a different binary.
    ///
    /// Docs: [`bwrapPath`](https://code.claude.com/docs/en/settings-reference#sandbox-bwrappath).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bwrap_path: Option<String>,
    /// Point the sandbox network proxy at a `socat` binary installed outside `PATH`.
    ///
    /// - **Values**: string, an absolute path; Claude Code drops a relative path and falls back
    ///   to `PATH` lookup
    /// - **Default**: unset, so Claude Code finds `socat` on `PATH`
    /// - **Scope**: managed.
    ///
    /// Docs: [`socatPath`](https://code.claude.com/docs/en/settings-reference#sandbox-socatpath).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socat_path: Option<String>,
    /// Make Claude Code exit with an error at startup when `sandbox.enabled` is `true` but the
    /// sandbox can't start because the platform doesn't support it or the dependency check
    /// fails. Without this key, Claude Code runs commands unsandboxed. Managed deployments that
    /// require sandboxing as a security gate can use this setting.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code exits with an error at startup when `sandbox.enabled` is `true`
    ///     but the sandbox can't start. If the sandbox passes that check and then fails to
    ///     initialize, the session keeps running and Claude Code [refuses each sandboxed
    ///     command](https://code.claude.com/docs/en/sandboxing#commands-refused-with-sandbox-is-required)
    ///   - `false`: Claude Code runs commands unsandboxed when the sandbox can't start
    /// - **Default**: `false`
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`failIfUnavailable`](https://code.claude.com/docs/en/settings-reference#sandbox-failifunavailable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_if_unavailable: Option<bool>,
    /// Limit the entire sandbox configuration to the listed platforms. On platforms not in the
    /// list the sandbox config is inert: no sandbox, no auto-allow, no startup warning, and no
    /// failIfUnavailable exit. When omitted, all supported platforms are included.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`enabledPlatforms`](https://code.claude.com/docs/en/settings-reference#sandbox).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_platforms: Option<Vec<Platform>>,
    /// Let sandboxed commands on macOS send Apple Events, which `open`, `osascript`, and tools
    /// that open URLs in a browser need; without it they fail with error `-600`. This removes
    /// code-execution isolation: sandboxed commands can launch other applications unsandboxed
    /// with no user prompt, and can send AppleScript commands to running applications such as
    /// Terminal, subject to the per-app macOS automation-consent prompt (TCC).
    ///
    /// - **Values**: Boolean
    ///   - `true`: sandboxed commands on macOS can send Apple Events
    ///   - `false`: sandboxed commands on macOS can't send Apple Events, so `open` and
    ///     `osascript` fail with error `-600`
    /// - **Default**: `false`
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`allowAppleEvents`](https://code.claude.com/docs/en/settings-reference#sandbox-allowappleevents).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_apple_events: Option<bool>,
    /// Let sandboxed commands on Linux and WSL2 create user namespaces, which Chromium's and
    /// Electron's own sandbox, a nested bubblewrap, and `unshare` need. If you set this key to
    /// `true`, you turn off a security boundary: code from a sandboxed command can end up
    /// running outside the sandbox. For rootless Podman or Buildah, use
    /// [`excludedCommands`](https://code.claude.com/docs/en/settings-reference#sandbox-excludedcommands)
    /// instead, because the sandbox stops them in other ways too.
    ///
    /// - **Values**: Boolean
    ///   - `true`: sandboxed commands can create user namespaces
    ///   - `false`: this key allows nothing extra, and the sandbox's default applies
    /// - **Default**: `false`
    /// - **Scope**: user or managed. A `true` counts from `~/.claude/settings.json`, JSON
    ///   passed with `--settings`, or [managed
    ///   settings](https://code.claude.com/docs/en/managed-settings).
    ///
    /// Docs: [`allowNestedUserNamespaces`](https://code.claude.com/docs/en/settings-reference#sandbox-allownestedusernamespaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_nested_user_namespaces: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.network`.
///
/// Control which hosts, ports, and sockets sandboxed commands can reach. The sandbox routes
/// outbound traffic through a proxy that enforces these lists; see [Network
/// isolation](https://code.claude.com/docs/en/sandboxing#network-isolation) for how the proxy
/// decides and when it prompts.
///
/// Docs: [`sandbox.network`](https://code.claude.com/docs/en/settings-reference#sandbox-network).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxNetwork {
    /// Pre-allow domains for outbound traffic from sandboxed commands, so the sandbox doesn't
    /// prompt for them. Wildcards such as `*.example.com` match subdomains, and an optional
    /// `:port` suffix limits an entry to one port; an entry without a port matches every port.
    ///
    /// - **Values**: array of strings, each a domain, wildcard pattern, or IP literal, with an
    ///   optional `:port` suffix
    /// - **Default**: unset, so your permission mode decides [what happens to each new
    ///   host](https://code.claude.com/docs/en/sandboxing#hosts-outside-your-allowed-domains)
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///   Only managed settings when allowmanageddomainsonly is set.
    ///
    /// Docs: [`allowedDomains`](https://code.claude.com/docs/en/settings-reference#sandbox-network-alloweddomains).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    /// Block domains for outbound traffic from sandboxed commands, using the same wildcard,
    /// port, and IPv6 syntax as
    /// [`allowedDomains`](https://code.claude.com/docs/en/settings-reference#sandbox-network-alloweddomains).
    /// A denied domain stays blocked even when an `allowedDomains` entry matches it too.
    ///
    /// - **Values**: array of strings, each a domain, wildcard pattern, or IP literal, with an
    ///   optional `:port` suffix
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`deniedDomains`](https://code.claude.com/docs/en/settings-reference#sandbox-network-denieddomains).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_domains: Option<Vec<String>>,
    /// Lock the network allowlist to what managed settings define. Claude Code then honors only
    /// `allowedDomains` and `WebFetch(domain:...)` allow rules from managed settings, ignores
    /// domains from user, project, local, and `--settings` settings, and blocks a non-allowed
    /// domain automatically instead of prompting.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code honors only `allowedDomains` and `WebFetch(domain:...)` allow
    ///     rules from managed settings and blocks a non-allowed domain instead of prompting
    ///   - `false`: domains from other settings files can merge into the allowlist
    /// - **Default**: `false`
    /// - **Scope**: managed.
    ///
    /// Docs: [`allowManagedDomainsOnly`](https://code.claude.com/docs/en/settings-reference#sandbox-network-allowmanageddomainsonly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_domains_only: Option<bool>,
    /// List the Unix socket paths sandboxed commands can connect to on macOS. Claude Code
    /// ignores this list on Linux and WSL2, where the seccomp filter can't inspect socket
    /// paths; use
    /// [`allowAllUnixSockets`](https://code.claude.com/docs/en/settings-reference#sandbox-network-allowallunixsockets)
    /// there instead.
    ///
    /// - **Values**: array of strings, each a socket path
    /// - **Default**: unset, so the macOS sandbox blocks every Unix socket
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`allowUnixSockets`](https://code.claude.com/docs/en/settings-reference#sandbox-network-allowunixsockets).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unix_sockets: Option<Vec<String>>,
    /// Let sandboxed commands connect to every Unix socket. On Linux and WSL2, the sandbox's
    /// [seccomp filter](https://code.claude.com/docs/en/sandboxing#set-up-linux-and-wsl2)
    /// blocks `socket(AF_UNIX, ...)` calls, so this is the only way to permit Unix sockets
    /// there. When the filter is missing, which `/sandbox` reports on its Dependencies tab, the
    /// sandbox doesn't block Unix-socket calls.
    ///
    /// - **Values**: Boolean
    ///   - `true`: sandboxed commands can connect to every Unix socket. On Linux and WSL2 they
    ///     can also connect to VM sockets, the `AF_VSOCK` address family
    ///   - `false`: the sandbox blocks Unix-socket connections: on macOS except the paths in
    ///     `allowUnixSockets`, and on Linux and WSL2 through the seccomp filter when it's
    ///     present
    /// - **Default**: `false`
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`allowAllUnixSockets`](https://code.claude.com/docs/en/settings-reference#sandbox-network-allowallunixsockets).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_unix_sockets: Option<bool>,
    /// Let sandboxed commands on macOS listen on network ports, for example to start a dev
    /// server, and connect to any port on localhost. A command that listens on a non-loopback
    /// address accepts connections from other machines. The key has no effect on Linux and
    /// WSL2, where each sandboxed command has its own loopback interface.
    ///
    /// - **Values**: Boolean
    ///   - `true`: sandboxed commands on macOS can listen on any local address and connect to
    ///     any port on localhost
    ///   - `false`: sandboxed commands on macOS can't listen on a port or connect directly to
    ///     servers on localhost
    /// - **Default**: `false`
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`allowLocalBinding`](https://code.claude.com/docs/en/settings-reference#sandbox-network-allowlocalbinding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_local_binding: Option<bool>,
    /// List additional XPC and Mach service names the macOS sandbox may look up. Tools that
    /// communicate over XPC, such as the iOS Simulator or Playwright, need their services
    /// listed here. Sandboxed commands can reach outside the sandbox to each service that you
    /// list, and the sandbox [doesn't confine what those services do for those
    /// commands](https://code.claude.com/docs/en/sandboxing#security-limitations).
    ///
    /// - **Values**: array of strings, each a service name; a single trailing `*` matches a
    ///   prefix, and `"*"` alone matches every service
    /// - **Default**: unset
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`allowMachLookup`](https://code.claude.com/docs/en/settings-reference#sandbox-network-allowmachlookup).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_mach_lookup: Option<Vec<String>>,
    /// Point the sandbox at your own HTTP proxy instead of the one Claude Code runs.
    /// Organizations do this to inspect HTTPS traffic, apply their own filtering rules, or log
    /// requests. Your proxy takes over filtering, and Claude Code stops applying its domain
    /// lists and network prompts to traffic sent there.
    ///
    /// - **Values**: number, a local TCP port
    /// - **Default**: unset, so Claude Code runs its own proxy
    /// - **Scope**: any settings file, unless [other sandbox settings limit which files can set
    ///   a port](https://code.claude.com/docs/en/sandboxing#custom-proxy-configuration).
    ///
    /// Docs: [`httpProxyPort`](https://code.claude.com/docs/en/settings-reference#sandbox-network-httpproxyport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy_port: Option<u16>,
    /// Point the sandbox at your own SOCKS5 proxy instead of the one Claude Code runs. Your
    /// proxy takes over filtering, and Claude Code stops applying its domain lists and network
    /// prompts to traffic sent there. When unset, Claude Code starts its own proxy for SOCKS
    /// traffic.
    ///
    /// - **Values**: number, a local TCP port
    /// - **Default**: unset, so Claude Code runs its own proxy
    /// - **Scope**: any settings file, unless [other sandbox settings limit which files can set
    ///   a port](https://code.claude.com/docs/en/sandboxing#custom-proxy-configuration).
    ///
    /// Docs: [`socksProxyPort`](https://code.claude.com/docs/en/settings-reference#sandbox-network-socksproxyport).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_proxy_port: Option<u16>,
    /// Make the sandbox proxy terminate TLS so it can read the contents of HTTPS requests. This
    /// is experimental, and `mask` [credential
    /// substitution](https://code.claude.com/docs/en/sandboxing#mask-credentials) requires it.
    /// Set `{}` to generate an ephemeral certificate authority for the session, or set
    /// `caCertPath` and `caKeyPath` to use your own.
    ///
    /// - **Values**: object with optional `caCertPath` and `caKeyPath` strings, each a file
    ///   path
    /// - **Default**: unset, so the proxy doesn't terminate or inspect TLS
    /// - **Scope**: user or managed. A repository can't switch it on or supply a certificate
    ///   authority.
    ///
    /// Docs: [`tlsTerminate`](https://code.claude.com/docs/en/settings-reference#sandbox-network-tlsterminate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_terminate: Option<TlsTerminate>,
    /// Deny sandboxed commands access to hosts outside the allowlist instead of prompting for
    /// approval. The allowlist is
    /// [`allowedDomains`](https://code.claude.com/docs/en/settings-reference#sandbox-network-alloweddomains)
    /// plus domains from `WebFetch(domain:...)` allow rules, or only the managed settings
    /// entries when
    /// [`allowManagedDomainsOnly`](https://code.claude.com/docs/en/settings-reference#sandbox-network-allowmanageddomainsonly)
    /// is set. [Locks that apply without an admin-required
    /// sandbox](https://code.claude.com/docs/en/sandboxing#locks-that-apply-without-an-admin-required-sandbox)
    /// covers a repository's entries.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code denies sandboxed commands access to hosts outside the allowlist
    ///   - `false`: unless another trusted settings file sets `true`, Claude Code decides a
    ///     host outside the allowlist by permission mode instead of denying it outright: in
    ///     auto mode it checks the host against the command's [per-command allowed
    ///     domains](https://code.claude.com/docs/en/sandboxing#per-command-allowed-domains-in-auto-mode),
    ///     in `dontAsk` mode it denies, in `bypassPermissions` mode and in interactive terminal
    ///     plan-mode sessions where bypass is available it allows, and otherwise it asks you
    /// - **Default**: `false`
    /// - **Scope**: user or managed. A repository can't turn it on or off.
    ///
    /// Docs: [`strictAllowlist`](https://code.claude.com/docs/en/settings-reference#sandbox-network-strictallowlist).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_allowlist: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.network.tlsTerminate`; empty for a generated CA.
///
/// Make the sandbox proxy terminate TLS so it can read the contents of HTTPS requests. This is
/// experimental, and `mask` [credential
/// substitution](https://code.claude.com/docs/en/sandboxing#mask-credentials) requires it.
///
/// Docs: [`sandbox.network.tlsTerminate`](https://code.claude.com/docs/en/settings-reference#sandbox-network-tlsterminate).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TlsTerminate {
    /// Path to your own certificate authority certificate.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`caCertPath`](https://code.claude.com/docs/en/settings-reference#sandbox-network-tlsterminate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_cert_path: Option<String>,
    /// Path to your own certificate authority private key.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`caKeyPath`](https://code.claude.com/docs/en/settings-reference#sandbox-network-tlsterminate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_key_path: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.filesystem`.
///
/// Control which paths sandboxed commands can read and write. By default they can write to the
/// working directory, the per-user temp directory, and directories you add with `--add-dir`,
/// `/add-dir`, or `permissions.additionalDirectories`, and can read the rest of the filesystem,
/// including credential files.
///
/// Docs: [`sandbox.filesystem`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxFilesystem {
    /// Add paths where sandboxed commands can write, beyond the working directory, the per-user
    /// temp directory, and the directories you've added with `--add-dir`, `/add-dir`, or
    /// `permissions.additionalDirectories`. Use it when a subprocess such as `kubectl` or a
    /// build tool needs to write outside the project.
    ///
    /// - **Values**: array of path strings, using the [sandbox path
    ///   prefixes](https://code.claude.com/docs/en/settings-reference#sandbox-path-prefixes)
    /// - **Default**: unset, so sandboxed commands can write to the working directory, the
    ///   per-user temp directory, directories you've added with `--add-dir` or `/add-dir`, and
    ///   directories in
    ///   [`permissions.additionalDirectories`](https://code.claude.com/docs/en/settings-reference#permissions-additionaldirectories)
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`allowWrite`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-allowwrite).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_write: Option<Vec<String>>,
    /// Block sandboxed commands from writing to specific paths, including paths inside a
    /// directory that is otherwise writable.
    ///
    /// - **Values**: array of path strings, using the [sandbox path
    ///   prefixes](https://code.claude.com/docs/en/settings-reference#sandbox-path-prefixes)
    /// - **Default**: unset
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`denyWrite`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-denywrite).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_write: Option<Vec<String>>,
    /// Block sandboxed commands from reading specific paths, such as credential files that the
    /// default read policy would otherwise expose. To protect a credential file and keep it
    /// usable through the sandbox proxy, see
    /// [`sandbox.credentials`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials)
    /// instead.
    ///
    /// - **Values**: array of path strings, using the [sandbox path
    ///   prefixes](https://code.claude.com/docs/en/settings-reference#sandbox-path-prefixes)
    /// - **Default**: unset, so sandboxed commands keep the [default read
    ///   access](https://code.claude.com/docs/en/sandboxing#filesystem-isolation), which
    ///   includes credential files such as `~/.aws/credentials`
    /// - **Scope**: any settings file.
    ///
    /// Docs: [`denyRead`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-denyread).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_read: Option<Vec<String>>,
    /// Re-open reading for specific paths inside a region that
    /// [`denyRead`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-denyread)
    /// blocks, to build workspace-only read access. An exact or wildcard `denyRead` entry stays
    /// blocked inside a broader `allowRead`, as the [overlap
    /// table](https://code.claude.com/docs/en/sandboxing#configure-sandboxing) shows. When a
    /// wildcard `denyRead` entry such as `~/**/.env` matches a directory, Claude Code blocks
    /// reads of its contents as well.
    ///
    /// - **Values**: array of path strings, using the [sandbox path
    ///   prefixes](https://code.claude.com/docs/en/settings-reference#sandbox-path-prefixes)
    /// - **Default**: unset
    /// - **Scope**: any settings file, with [limits on project and local
    ///   settings](https://code.claude.com/docs/en/sandboxing#repository-settings-under-an-admin-required-sandbox).
    ///
    /// Docs: [`allowRead`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-allowread).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_read: Option<Vec<String>>,
    /// Honor only the
    /// [`allowRead`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-allowread)
    /// entries that come from managed settings, so developers can't re-open read access to
    /// paths your organization blocked. Claude Code still merges `denyRead` entries from every
    /// settings scope the session loads.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code honors only the `allowRead` entries from managed settings
    ///   - `false`: `allowRead` entries from other settings files can merge in
    /// - **Default**: `false`
    /// - **Scope**: managed.
    ///
    /// Docs: [`allowManagedReadPathsOnly`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-allowmanagedreadpathsonly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_read_paths_only: Option<bool>,
    /// Skip filesystem isolation while keeping network isolation. Sandboxed commands get
    /// unrestricted read and write access to the host filesystem, and their network egress
    /// stays confined to
    /// [`network.allowedDomains`](https://code.claude.com/docs/en/settings-reference#sandbox-network-alloweddomains).
    /// Use it when you sandbox to control where commands connect rather than what they write.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code skips filesystem isolation and keeps network isolation
    ///   - `false`: filesystem isolation stays on
    /// - **Default**: `false`, so filesystem isolation stays on
    /// - **Scope**: user or managed. When managed settings configure `sandbox.filesystem` at
    ///   all, or list a `sandbox.credentials.files` entry with `"mode": "deny"`, only managed
    ///   settings can set it.
    ///
    /// Docs: [`disabled`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-disabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.credentials`.
///
/// Declare the credential files and environment variables to [protect from sandboxed
/// commands](https://code.claude.com/docs/en/sandboxing#protect-credentials). Each entry names
/// a file `path` or a variable `name` and a `mode`: `deny` hides the credential inside the
/// sandbox, and `mask` shows sandboxed commands a placeholder while the [sandbox
/// proxy](https://code.claude.com/docs/en/sandboxing#mask-credentials) substitutes the real
/// value on outbound requests.
///
/// Docs: [`sandbox.credentials`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxCredentials {
    /// Protect credential files or directories from sandboxed commands. With `"mode": "deny"`,
    /// Claude Code blocks reads of the path inside the sandbox, the same read block as
    /// [`sandbox.filesystem.denyRead`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-denyread).
    /// With `"mode": "mask"`, sandboxed commands on Linux and WSL2 read a sentinel copy of the
    /// file, and the sandbox proxy substitutes the real value on outbound requests to that
    /// entry's `injectHosts`; on macOS the file is unreadable inside the sandbox instead.
    ///
    /// - **Values**: array of objects, each with `path` and a `mode` of `"deny"` or `"mask"`,
    ///   plus the optional [mask fields for
    ///   files](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files)
    /// - **Default**: unset, so no credential files are protected
    /// - **Scope**: any settings file. Claude Code drops `mask` entries from project
    ///   `.claude/settings.json` and local `.claude/settings.local.json`.
    ///
    /// Docs: [`files`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<CredentialFile>>,
    /// Protect environment variables from sandboxed commands. With `"mode": "deny"`, Claude
    /// Code removes the variable from the environment of sandboxed commands. With `"mode":
    /// "mask"`, sandboxed commands see a per-session sentinel value, and the sandbox proxy
    /// substitutes the real value on outbound requests to that entry's `injectHosts`, so tools
    /// such as `gh` and `npm` keep authenticating without ever holding the real credential.
    ///
    /// - **Values**: array of objects, each with `name` and a `mode` of `"deny"` or `"mask"`,
    ///   plus the optional [mask fields for environment
    ///   variables](https://code.claude.com/docs/en/settings-reference#mask-fields-for-environment-variables)
    /// - **Default**: unset, so no environment variables are protected
    /// - **Scope**: any settings file. Claude Code drops `mask` entries from project
    ///   `.claude/settings.json` and local `.claude/settings.local.json`.
    ///
    /// Docs: [`envVars`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-envvars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<Vec<CredentialEnvVar>>,
    /// Allow `mask` substitution on plain HTTP requests as well as TLS-terminated HTTPS. On
    /// plain HTTP the upstream identity is unverified and the credential travels in cleartext,
    /// so leave this off outside trusted test networks.
    ///
    /// - **Values**: Boolean
    ///   - `true`: Claude Code allows `mask` substitution on plain HTTP requests as well as
    ///     TLS-terminated HTTPS
    ///   - `false`: Claude Code allows `mask` substitution only on TLS-terminated HTTPS
    /// - **Default**: `false`
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`allowPlaintextInject`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-allowplaintextinject).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_plaintext_inject: Option<bool>,
    /// Group masked environment variables that form one AWS credential for [SigV4
    /// re-signing](https://code.claude.com/docs/en/sandboxing#re-sign-aws-requests) when your
    /// credential lives in variables with non-standard names. Claude Code links the
    /// conventional `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and `AWS_SESSION_TOKEN` trio
    /// automatically when you mask their whole values, so you need this key only for other
    /// names. Requires Claude Code v2.1.224 or later.
    ///
    /// - **Values**: array of objects, each with `accessKeyIdVar`, `secretAccessKeyVar`, and
    ///   optionally `sessionTokenVar`, naming `sandbox.credentials.envVars` entries
    /// - **Default**: unset, so only the conventional trio is paired
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`awsPairs`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-awspairs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_pairs: Option<Vec<AwsPair>>,
    /// Choose what the sandbox proxy does with AWS request forms it [can't
    /// re-sign](https://code.claude.com/docs/en/sandboxing#re-sign-aws-requests): `streaming`
    /// for aws-chunked streaming uploads, `presigned` for presigned URLs, and `sigv4a` for
    /// SigV4A asymmetric signatures. This applies only to requests signed with a masked pair's
    /// placeholder access key ID. Requires Claude Code v2.1.224 or later.
    ///
    /// - **Values**: object with `streaming`, `presigned`, and `sigv4a`, each one of:
    ///   - `"deny"`: the proxy fails the request
    ///   - `"passthrough"`: the proxy forwards the request signed with the masked placeholder,
    ///     so the tool receives AWS's own rejection
    /// - **Default**: unset, so every form is `"deny"`
    /// - **Scope**: user or managed.
    ///
    /// Docs: [`sigv4`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-sigv4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sigv4: Option<Sigv4>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sandbox.credentials.files`.
///
/// Protect credential files or directories from sandboxed commands. With `"mode": "deny"`,
/// Claude Code blocks reads of the path inside the sandbox, the same read block as
/// [`sandbox.filesystem.denyRead`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-denyread).
///
/// Docs: [`sandbox.credentials.files`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-files).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CredentialFile {
    /// Path to the credential file to deny.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`path`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Access mode; only "deny" is supported.
    ///
    /// - **Values**: `"deny"`
    /// - **Default**: unset
    ///
    /// Docs: [`mode`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<CredentialMode>,
    /// Mask only the text captured by group 1 of each match, so the rest of the file stays
    /// parseable. With `decode` also set, Claude Code checks each capture as a possible JWT
    /// instead of replacing it outright. Requires v2.1.221 or later.
    ///
    /// - **Values**: string, a regular expression with at least one capturing group
    /// - **Default**: unset
    ///
    /// Docs: [`extract`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<String>,
    /// What happens when `extract` or `decode` finds nothing to mask. `warn` leaves the file
    /// readable as-is inside the sandbox, `deny` makes it unreadable, and `error` stops sandbox
    /// setup until you fix the configuration. Claude Code treats `deny` as `error` when the
    /// read block wouldn't be enforced, because you [disable filesystem
    /// isolation](https://code.claude.com/docs/en/sandboxing#disable-filesystem-isolation) or a
    /// [`sandbox.filesystem.allowRead`](https://code.claude.com/docs/en/settings-reference#sandbox-filesystem-allowread)
    /// entry re-opens the path.
    ///
    /// - **Values**: `"warn"`, `"deny"`, or `"error"`; default `"warn"`
    /// - **Default**: unset
    ///
    /// Docs: [`onExtractNoMatch`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_extract_no_match: Option<ExtractNoMatch>,
    /// Find JSON Web Tokens (JWTs) in the file, with a built-in pattern or with `extract` when
    /// set, verify each candidate, and replace it with a structurally valid fake token, so code
    /// inside the sandbox that decodes the token keeps working. When no candidate verifies,
    /// `onExtractNoMatch` governs the outcome. Requires v2.1.224 or later.
    ///
    /// - **Values**: the string `"jwt"`
    /// - **Default**: unset
    ///
    /// Docs: [`decode`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode: Option<TokenDecode>,
    /// Mask only the named top-level payload claims inside each verified JWT and rebuild the
    /// token around the modified payload, so the other claims stay readable. When no named
    /// claim matches, `onExtractNoMatch` governs the outcome. Requires v2.1.224 or later.
    ///
    /// - **Values**: array of strings, at least one claim name; requires `decode`
    /// - **Default**: unset
    ///
    /// Docs: [`maskClaims`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_claims: Option<Vec<String>>,
    /// Also replace verbatim copies of each masked value elsewhere in the file, such as a
    /// secret pasted into a comment. Claude Code matches raw substrings, so reserve it for
    /// long, high-entropy secrets. Consulted only when `extract` or `decode` is set.
    ///
    /// - **Values**: Boolean, default `false`
    /// - **Default**: unset
    ///
    /// Docs: [`maskDuplicates`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_duplicates: Option<bool>,
    /// Narrow the hosts where the sandbox proxy substitutes the real value. When unset, the
    /// proxy substitutes it on requests to every host in `sandbox.network.allowedDomains`.
    /// Requires v2.1.221 or later.
    ///
    /// - **Values**: array of strings, each a host that
    ///   [`sandbox.network.allowedDomains`](https://code.claude.com/docs/en/settings-reference#sandbox-network-alloweddomains)
    ///   also admits
    /// - **Default**: unset
    ///
    /// Docs: [`injectHosts`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-files).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject_hosts: Option<Vec<String>>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sandbox.credentials.envVars`.
///
/// Protect environment variables from sandboxed commands. With `"mode": "deny"`, Claude Code
/// removes the variable from the environment of sandboxed commands.
///
/// Docs: [`sandbox.credentials.envVars`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-envvars).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CredentialEnvVar {
    /// Environment variable name to deny.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`name`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-envvars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Access mode; only "deny" is supported.
    ///
    /// - **Values**: `"deny"`
    /// - **Default**: unset
    ///
    /// Docs: [`mode`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-envvars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<CredentialMode>,
    /// Mask only the text captured by group 1 of each match, such as the password inside a
    /// `DATABASE_URL` connection string, so the rest of the value stays parseable. Requires
    /// v2.1.224 or later.
    ///
    /// - **Values**: string, a regular expression with at least one capturing group
    /// - **Default**: unset
    ///
    /// Docs: [`extract`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-environment-variables).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<String>,
    /// What happens when `extract` matches nothing. `warn` passes the variable through
    /// unmasked, `deny` unsets it inside the sandbox, and `error` stops sandbox setup until you
    /// fix the configuration. Requires v2.1.224 or later.
    ///
    /// - **Values**: `"warn"`, `"deny"`, or `"error"`; default `"warn"`. On an entry with
    ///   `decode`, only `"warn"` is accepted
    /// - **Default**: unset
    ///
    /// Docs: [`onExtractNoMatch`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-environment-variables).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_extract_no_match: Option<ExtractNoMatch>,
    /// Verify the whole value is a JWT and replace it with a structurally valid fake token, so
    /// code inside the sandbox that decodes the token keeps working; the proxy substitutes the
    /// whole real token on egress. A value that doesn't verify passes through unmasked with a
    /// warning. Requires v2.1.224 or later.
    ///
    /// - **Values**: the string `"jwt"`
    /// - **Default**: unset
    ///
    /// Docs: [`decode`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-environment-variables).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode: Option<TokenDecode>,
    /// Mask only the named top-level payload claims inside the decoded JWT and rebuild the
    /// token around the modified payload, so the other claims stay readable. When no named
    /// claim matches, the variable passes through unmasked with a warning. Requires v2.1.224 or
    /// later.
    ///
    /// - **Values**: array of strings, at least one claim name; requires `decode`
    /// - **Default**: unset
    ///
    /// Docs: [`maskClaims`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-environment-variables).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_claims: Option<Vec<String>>,
    /// Narrow the hosts where the sandbox proxy substitutes the real value. When unset, the
    /// proxy substitutes it on requests to every host in `sandbox.network.allowedDomains`.
    /// Write an IPv6 destination as the bare compressed address, such as `"::1"`, not the
    /// bracketed form; see [IPv6 destinations in
    /// `injectHosts`](https://code.claude.com/docs/en/sandboxing#ipv6-destinations-in-injecthosts).
    ///
    /// - **Values**: array of strings, each a host that
    ///   [`sandbox.network.allowedDomains`](https://code.claude.com/docs/en/settings-reference#sandbox-network-alloweddomains)
    ///   also admits
    /// - **Default**: unset
    ///
    /// Docs: [`injectHosts`](https://code.claude.com/docs/en/settings-reference#mask-fields-for-environment-variables).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject_hosts: Option<Vec<String>>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sandbox.credentials.awsPairs`.
///
/// Group masked environment variables that form one AWS credential for [SigV4
/// re-signing](https://code.claude.com/docs/en/sandboxing#re-sign-aws-requests) when your
/// credential lives in variables with non-standard names. Claude Code links the conventional
/// `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and `AWS_SESSION_TOKEN` trio automatically
/// when you mask their whole values, so you need this key only for other names.
///
/// Docs: [`sandbox.credentials.awsPairs`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-awspairs).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AwsPair {
    /// The masked variable that holds the access key ID.
    ///
    /// - **Values**: string, the `name` of a `sandbox.credentials.envVars` entry (required)
    /// - **Default**: unset
    ///
    /// Docs: [`accessKeyIdVar`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id_var: Option<String>,
    /// The masked variable that holds the secret access key.
    ///
    /// - **Values**: string, the `name` of a `sandbox.credentials.envVars` entry (required)
    /// - **Default**: unset
    ///
    /// Docs: [`secretAccessKeyVar`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key_var: Option<String>,
    /// The masked variable that holds the session token.
    ///
    /// - **Values**: string, the `name` of a `sandbox.credentials.envVars` entry
    /// - **Default**: unset
    ///
    /// Docs: [`sessionTokenVar`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token_var: Option<String>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.credentials.sigv4`.
///
/// Choose what the sandbox proxy does with AWS request forms it [can't
/// re-sign](https://code.claude.com/docs/en/sandboxing#re-sign-aws-requests): `streaming` for
/// aws-chunked streaming uploads, `presigned` for presigned URLs, and `sigv4a` for SigV4A
/// asymmetric signatures. This applies only to requests signed with a masked pair's placeholder
/// access key ID. Requires Claude Code v2.1.224 or later.
///
/// Docs: [`sandbox.credentials.sigv4`](https://code.claude.com/docs/en/settings-reference#sandbox-credentials-sigv4).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Sigv4 {
    /// What the proxy does with aws-chunked streaming uploads it can't re-sign.
    ///
    /// - **Values**: `"deny"` (fail the request) or `"passthrough"` (forward it signed with the
    ///   placeholder)
    /// - **Default**: `"deny"`
    ///
    /// Docs: [`streaming`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<Sigv4Action>,
    /// What the proxy does with presigned URLs.
    ///
    /// - **Values**: `"deny"` or `"passthrough"`
    /// - **Default**: `"deny"`
    ///
    /// Docs: [`presigned`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presigned: Option<Sigv4Action>,
    /// What the proxy does with SigV4A asymmetric signatures.
    ///
    /// - **Values**: `"deny"` or `"passthrough"`
    /// - **Default**: `"deny"`
    ///
    /// Docs: [`sigv4a`](None).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sigv4a: Option<Sigv4Action>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.ripgrep`.
///
/// Point the sandbox at a ripgrep binary of your own instead of the one Claude Code uses, for
/// example when your platform needs a differently built `rg`.
///
/// Docs: [`sandbox.ripgrep`](https://code.claude.com/docs/en/settings-reference#sandbox-ripgrep).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Ripgrep {
    /// Path to the ripgrep binary to use.
    ///
    /// - **Values**: string
    /// - **Default**: unset
    ///
    /// Docs: [`command`](https://code.claude.com/docs/en/settings-reference#sandbox-ripgrep).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Additional arguments to pass to the ripgrep binary.
    ///
    /// - **Values**: array of strings
    /// - **Default**: unset
    ///
    /// Docs: [`args`](https://code.claude.com/docs/en/settings-reference#sandbox-ripgrep).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Keys Claude Code accepts here that this crate doesn't model (newer or
    /// undocumented ones), kept as written so the document round-trips.
    #[serde(flatten)]
    pub extra: Extra,
}

walk!(Sandbox {
    network: "network",
    filesystem: "filesystem",
    credentials: "credentials",
    ripgrep: "ripgrep",
});
walk!(SandboxNetwork {
    tls_terminate: "tlsTerminate"
});
walk!(TlsTerminate {});
walk!(SandboxFilesystem {});
walk!(SandboxCredentials {
    files: "files",
    env_vars: "envVars",
    aws_pairs: "awsPairs",
    sigv4: "sigv4",
});
walk!(AwsPair {});
walk!(Sigv4 {});
walk!(CredentialFile {});
walk!(CredentialEnvVar {});
walk!(Ripgrep {});
