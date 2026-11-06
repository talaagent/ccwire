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
serializable, zeroed on drop. Errors never carry a token.

The endpoints are not publicly documented and may change; unknown fields
land in each type's `extra` map instead of failing.

## License

Apache-2.0. See [LICENSE](LICENSE).
