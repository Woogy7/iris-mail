# Iris Mail

> *"Iris, swift-footed messenger of the gods, bore tidings across the rainbow bridge."*

A fast, beautiful, open-source desktop email client for Windows and Linux. Built for people frustrated with Outlook's sluggish search, Thunderbird's dated feel, and the lack of a modern, lightweight alternative.

Iris Mail is local-first, instant, and considered. Three-pane layout you already know. Search that returns in milliseconds. Storage tiering that keeps your M365 mailbox under quota without losing access to old mail. No Electron. No telemetry. Built in Rust with Tauri.

## Status

Early development. See [`iris-mail-spec.md`](iris-mail-spec.md) for the full specification and roadmap.

## Highlights

- **Cross-platform** — Windows and Linux, single small binary
- **Lightning-fast search** — SQLite FTS5 with a GitHub-style query language (`from:terry has:attachment after:2025-01-01`)
- **Microsoft 365 and Gmail** — OAuth2 login, no app passwords
- **Storage tiering** — automatically archive old mail locally while keeping your server quota healthy
- **Progressive sync** — usable in under a minute, even on huge mailboxes
- **Offline-first** — read, search, and compose without a connection
- **Outlook-familiar UI** — three-pane layout, no learning curve
- **Catppuccin theming** — Latte, Frappé, Macchiato, Mocha
- **Secure by default** — remote images blocked, HTML sandboxed, tokens in the OS keychain

## Tech Stack

Rust · Tauri v2 · Svelte 5 · Tailwind CSS · SQLite (FTS5) · sqlx · tokio · async-imap · lettre

## Building

*(Coming once the Phase 1 skeleton lands.)*

## Contributing

Contributions are welcome. Please read [`CLAUDE.md`](CLAUDE.md) before submitting a pull request — it covers code style, architectural conventions, and how to work effectively with the codebase (including guidance for AI coding assistants).

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT licence ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 licence, shall be dual licensed as above, without any additional terms or conditions.
