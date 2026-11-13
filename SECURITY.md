# Security policy

Security fixes target the latest published versions of `ccwire`,
`ccwire-oauth` and `ccwire-settings`. This project is early in development; review changes before
upgrading and avoid assuming compatibility with every Claude Code release.

Report vulnerabilities privately through
[GitHub's private vulnerability reporting](https://github.com/talaagent/ccwire/security/advisories/new).
If that form is unavailable, email the maintainer at **github@igm.pub**.
Please do not disclose an unpatched vulnerability in a public issue.

Include the affected crate and version, impact, and a minimal reproduction.
Never include live OAuth tokens, API keys, account data, or unsanitized transcripts.
Token refresh issues should describe cancellation and storage behavior without
including the credentials themselves.
