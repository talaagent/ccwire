//! `sandbox`: the sandboxed Bash environment.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::Extra;
use crate::enums::{CredentialMode, ExtractNoMatch, Platform, Sigv4Action, TokenDecode};
use crate::walk::walk;

/// `sandbox`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Sandbox {
    /// Run Bash in the sandbox.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Allow sandboxed Bash commands without asking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_allow_bash_if_sandboxed: Option<bool>,
    /// Commands that never run in the sandbox.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_commands: Option<Vec<String>>,
    /// Let a command ask to run outside the sandbox
    /// (`dangerouslyDisableSandbox`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unsandboxed_commands: Option<bool>,
    /// Command patterns → paths whose violations to ignore (`*`: all
    /// commands).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_violations: Option<BTreeMap<String, Vec<String>>>,
    /// Network isolation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<SandboxNetwork>,
    /// Filesystem isolation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<SandboxFilesystem>,
    /// Credential files and variables hidden from sandboxed commands.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<SandboxCredentials>,
    /// A custom ripgrep.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ripgrep: Option<Ripgrep>,
    /// macOS: allow the system TLS trust service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_network_isolation: Option<bool>,
    /// A weaker sandbox for unprivileged containers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_nested_sandbox: Option<bool>,
    /// A custom bubblewrap binary (managed settings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bwrap_path: Option<String>,
    /// A custom socat binary (managed settings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socat_path: Option<String>,
    /// Fail at startup when the sandbox's dependencies are missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_if_unavailable: Option<bool>,
    /// The platforms the sandbox configuration applies on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_platforms: Option<Vec<Platform>>,
    /// macOS: allow Apple Events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_apple_events: Option<bool>,
    /// Linux and WSL2: let sandboxed commands create user namespaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_nested_user_namespaces: Option<bool>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.network`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxNetwork {
    /// Domains sandboxed commands may reach (`*.example.com`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    /// Domains they may not, even under an allowed wildcard.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denied_domains: Option<Vec<String>>,
    /// Only managed settings' domains apply (managed settings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_domains_only: Option<bool>,
    /// Unix sockets they may use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unix_sockets: Option<Vec<String>>,
    /// Allow every Unix socket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_unix_sockets: Option<bool>,
    /// Allow binding local addresses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_local_binding: Option<bool>,
    /// macOS: XPC/Mach services they may look up.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_mach_lookup: Option<Vec<String>>,
    /// The HTTP proxy's port (one is started when unset).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy_port: Option<u16>,
    /// The SOCKS proxy's port (one is started when unset).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_proxy_port: Option<u16>,
    /// Terminate TLS in the proxy (experimental).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_terminate: Option<TlsTerminate>,
    /// Deny unlisted hosts without asking (undocumented).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_allowlist: Option<bool>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.network.tlsTerminate`; empty for a generated CA.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TlsTerminate {
    /// Your CA certificate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_cert_path: Option<String>,
    /// Your CA's private key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_key_path: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.filesystem`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxFilesystem {
    /// Paths sandboxed commands may write.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_write: Option<Vec<String>>,
    /// Paths they may not write (beats `allowWrite`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_write: Option<Vec<String>>,
    /// Paths they may not read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_read: Option<Vec<String>>,
    /// Paths inside `denyRead` they may read again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_read: Option<Vec<String>>,
    /// Only managed settings' `allowRead` applies (managed settings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_managed_read_paths_only: Option<bool>,
    /// No filesystem isolation (network isolation stays).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.credentials`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SandboxCredentials {
    /// Credential files to hide.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<CredentialFile>>,
    /// Secret variables to unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<Vec<CredentialEnvVar>>,
    /// Substitute masks on plain HTTP too.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_plaintext_inject: Option<bool>,
    /// Masked variables that form one AWS credential.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_pairs: Option<Vec<AwsPair>>,
    /// What the proxy does with AWS requests it can't re-sign.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sigv4: Option<Sigv4>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sandbox.credentials.files`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CredentialFile {
    /// The file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `deny`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<CredentialMode>,
    /// A regex whose group 1 is the part to mask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<String>,
    /// What happens when nothing matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_extract_no_match: Option<ExtractNoMatch>,
    /// Mask JSON Web Tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode: Option<TokenDecode>,
    /// The JWT claims to mask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_claims: Option<Vec<String>>,
    /// Also mask copies of the value elsewhere in the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_duplicates: Option<bool>,
    /// The hosts the real value is sent to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject_hosts: Option<Vec<String>>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sandbox.credentials.envVars`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CredentialEnvVar {
    /// The variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `deny`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<CredentialMode>,
    /// A regex whose group 1 is the part to mask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract: Option<String>,
    /// What happens when nothing matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_extract_no_match: Option<ExtractNoMatch>,
    /// Mask JSON Web Tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decode: Option<TokenDecode>,
    /// The JWT claims to mask.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_claims: Option<Vec<String>>,
    /// The hosts the real value is sent to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inject_hosts: Option<Vec<String>>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// An entry of `sandbox.credentials.awsPairs`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AwsPair {
    /// The access key ID variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id_var: Option<String>,
    /// The secret access key variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key_var: Option<String>,
    /// The session token variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token_var: Option<String>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.credentials.sigv4`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Sigv4 {
    /// aws-chunked streaming uploads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<Sigv4Action>,
    /// Presigned URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presigned: Option<Sigv4Action>,
    /// SigV4A signatures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sigv4a: Option<Sigv4Action>,
    /// Keys this crate doesn't model.
    #[serde(flatten)]
    pub extra: Extra,
}

/// `sandbox.ripgrep`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Ripgrep {
    /// The ripgrep binary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Extra arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Keys this crate doesn't model.
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
