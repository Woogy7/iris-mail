# Iris Mail — Specification

> *"Iris, swift-footed messenger of the gods, bore tidings across the rainbow bridge."*

Iris Mail is a fast, beautiful, open-source desktop email client for Windows and Linux. It is built for people who are frustrated with Outlook's sluggish search, Thunderbird's dated feel, and the lack of a modern, lightweight alternative that respects both power users and their system resources.

Iris Mail is not trying to be everything. It is trying to be the best *reading, searching, and managing* email client you can install.

**Repository:** [`Woogy7/iris-mail`](https://github.com/Woogy7/iris-mail)
**Licence:** MIT OR Apache-2.0

---

## 1. Vision & Principles

### 1.1 Vision

A cross-platform desktop email client that feels instant, looks considered, and gets out of the way. Email is the last piece of software most professionals use every day that hasn't had a proper modern rewrite. Iris Mail is that rewrite.

### 1.2 Principles

1. **Speed is a feature.** Every interaction — opening, searching, switching folders — should feel instant. If it can be measured in milliseconds, it should be.
2. **Local-first.** The local database is the source of truth for reading and searching. The network is an implementation detail.
3. **Respect the user's hardware.** Small binary, modest memory footprint, minimal background CPU. No Electron.
4. **Secure by default.** Remote images blocked, HTML sandboxed, tokens in the OS keychain, no telemetry.
5. **Familiar, not novel.** Three-pane layout. Users should feel at home within thirty seconds. Innovation goes into what happens *underneath* the familiar surface.
6. **Beautiful in the details.** Typography, spacing, colour, and motion are deliberate. Catppuccin palette, Inter typeface, restrained accents.
7. **Open and contributable.** Clean architecture, thorough documentation, conventional code. Anyone with Claude Code and a few hours should be able to contribute meaningfully.

### 1.3 Non-goals (for v1)

- Mobile clients (iOS / Android)
- Calendar, contacts, or task management
- End-to-end encryption (PGP / S/MIME)
- Web version
- Enterprise management features (MDM, central policy)
- Exchange ActiveSync
- Supporting email providers beyond IMAP/SMTP and OAuth-capable M365/Gmail

---

## 2. Technology Stack

| Layer | Choice | Rationale |
|---|---|---|
| Core language | Rust (stable) | Performance, safety, single binary output |
| Desktop shell | Tauri v2 | Native webview, tiny binaries, cross-platform |
| Frontend framework | Svelte 5 | Lightweight, reactive, minimal boilerplate, Hotwire-adjacent feel |
| Styling | Tailwind CSS + Catppuccin | Utility-first with a curated palette |
| Typeface | Inter (UI), JetBrains Mono (monospace) | Clear, modern, widely loved |
| Database | SQLite with FTS5 | Embedded, zero-install, fast full-text search |
| DB driver | `sqlx` | Async, compile-time query checking, migrations |
| Async runtime | `tokio` | Standard, every library supports it |
| IMAP client | `async-imap` | Async-native, actively maintained |
| SMTP client | `lettre` | Standard Rust SMTP library |
| OAuth2 | `oauth2` + `tauri-plugin-oauth` | Handles PKCE flow and redirect capture |
| Secret storage | OS keychain via `keyring` crate | Never touch plaintext tokens |
| HTML → text | `html2text` | For search indexing only |
| HTML sanitisation | `ammonia` | Strip JS, sanitise for safe rendering |
| Error handling | `thiserror` (libs) + `anyhow` (app) | Community standard |
| Logging | `tracing` + `tracing-subscriber` | Structured logging, spans |
| Config | `serde` + TOML | Readable, user-editable |
| Packaging | Tauri bundler | Deb, AppImage, MSI, NSIS |

---

## 3. Architecture

### 3.1 Workspace layout

Iris Mail is a Cargo workspace split into focused crates. Each crate has a single responsibility and a minimal public API.

```
iris-mail/
├── Cargo.toml              # Workspace root
├── CLAUDE.md               # Conventions for contributors
├── README.md
├── LICENSE                 # GPL-3.0
├── rustfmt.toml
├── clippy.toml
├── .github/workflows/      # CI: fmt, clippy, test, build
├── crates/
│   ├── iris-core/          # Domain types, errors, traits
│   ├── iris-db/            # SQLite schema, migrations, queries
│   ├── iris-mail/          # IMAP/SMTP/OAuth — the "mail protocol" layer
│   ├── iris-search/        # Query parser + FTS5 wrapper
│   ├── iris-sync/          # Sync engine, storage tiering, background jobs
│   ├── iris-import/        # PST and mbox import
│   └── iris-app/           # Tauri shell and commands
├── ui/                     # Svelte frontend (built by Tauri)
│   ├── src/
│   ├── package.json
│   └── tailwind.config.js
└── docs/
    ├── architecture.md
    ├── data-model.md
    ├── search-syntax.md
    └── contributing.md
```

### 3.2 Crate boundaries

- **`iris-core`** defines the shared vocabulary: `Account`, `Folder`, `Message`, `Attachment`, `SyncState`, plus the `Error` enum. No I/O, no dependencies on other Iris crates. Pure types and traits.

- **`iris-db`** owns the SQLite schema and all queries. Exposes typed repository structs (`AccountRepo`, `MessageRepo`, etc.) that take a connection pool. Runs migrations on startup. No other crate writes SQL directly.

- **`iris-mail`** wraps `async-imap`, `lettre`, and the OAuth flow. Speaks to remote servers. Knows nothing about SQLite. Returns domain types from `iris-core`.

- **`iris-search`** parses the query language (see §6), translates it into SQL + FTS5 match expressions, and returns ranked results. Depends on `iris-db` for execution.

- **`iris-sync`** is the orchestrator. It coordinates `iris-mail` (fetching) and `iris-db` (storing), manages the sync state machine, runs background jobs for progressive sync and storage tiering, and emits events to the UI.

- **`iris-import`** handles PST and mbox import. Isolated so its heavy dependencies don't bloat the main app.

- **`iris-app`** is the Tauri binary. It wires everything together, exposes `#[tauri::command]` functions to the Svelte frontend, and handles app lifecycle (startup, shutdown, tray icon). This is the only crate that depends on Tauri.

### 3.3 Dependency direction

```
iris-app ──► iris-sync ──► iris-mail ──► iris-core
     │           │              │
     │           └──► iris-db ──┘
     │
     └──► iris-search ──► iris-db ──► iris-core
```

No circular dependencies. `iris-core` has zero Iris dependencies. `iris-app` depends on everything and is the composition root.

### 3.4 Frontend ↔ backend

The Svelte frontend never talks to the network or the database. It calls Tauri commands defined in `iris-app`, which delegate to the appropriate crate. Events flow the other way via Tauri's event system — the sync engine emits `sync-progress`, `new-message`, `account-state-changed`, and the UI subscribes.

All business logic lives in Rust. The frontend is a thin view layer.

---

## 4. Data Model

Full schema is in `docs/data-model.md`. The key tables:

### 4.1 `accounts`

One row per configured email account. Stores display name, email address, provider type (`m365`, `gmail`, `imap-generic`), OAuth token reference (a keychain identifier, *never* the token itself), sync preferences, storage tier thresholds, accent colour.

### 4.2 `folders`

Hierarchical folder tree. `parent_id` for nesting. Flags for special folders (Inbox, Sent, Drafts, Trash, Archive). Per-folder sync state (last UID seen, UIDVALIDITY, last sync timestamp).

### 4.3 `messages`

One row per message. Stores headers, flags (read, flagged, answered), folder reference, thread ID, date, size, and two critical booleans:

- `stored_local` — we have the full body and attachments on disk
- `stored_remote` — the message is still present on the server

A message can be in four states based on these flags, which drives the storage tier feature (§7) and the local/remote indicator in the UI.

### 4.4 `message_bodies`

Separated from `messages` so list views can load thousands of rows without pulling megabytes of HTML. Stores the full HTML (or plaintext) body, the sanitised HTML for rendering, and a reference to the plaintext version used for search indexing.

### 4.5 `attachments`

Deduplicated by SHA-256 hash. The `attachments` table stores the content once, keyed by hash. A join table `message_attachments` links messages to attachments with the filename and MIME type as declared in that specific message. Sending the same 5MB PDF to twelve people stores it once.

### 4.6 `messages_fts` (FTS5 virtual table)

Indexes subject, plaintext body, sender name, sender address, recipient addresses, and attachment filenames. Populated via triggers so it stays in sync with `messages` automatically.

### 4.7 `sync_jobs`

Queue of background work: initial sync, backfill, tier enforcement, attachment download. Survives app restarts.

---

## 5. Sync Engine

### 5.1 Progressive initial sync

When a user adds an account, they are up and running in under a minute even on a large mailbox. The sync engine works in phases:

1. **Folder discovery** (seconds). Fetch the full folder tree. User can navigate immediately, though folders are empty.
2. **Recent headers** (seconds to a minute). Fetch message headers only — no bodies, no attachments — for the last 60 days across all folders. The message list populates. The user sees what's there.
3. **Recent bodies** (a few minutes). Download bodies for the last 60 days, Inbox first, then other folders by recency. Attachments download alongside bodies for this tier.
4. **Background backfill** (hours to days, throttled). A low-priority worker downloads everything older, rate-limited to a configurable default (50 messages per minute) to avoid hammering the server or the user's connection.

The user can read and search everything that's been downloaded so far at any point. The backfill is invisible.

### 5.2 Ongoing sync

IMAP IDLE keeps a persistent connection to the Inbox for near-instant new mail delivery. A periodic poll (every 2 minutes by default) catches changes in other folders. Gmail's `X-GM-EXT-1` extension is used when the account is Gmail, giving proper thread IDs and label support.

### 5.3 Offline mode

The local database is always the source of truth for reads. Composing, replying, forwarding, flagging, and moving work offline — actions are queued in the `outbox` table and `pending_actions` table, and replayed to the server when connectivity returns. Conflicts (message deleted server-side while queued for a move) are logged and surfaced to the user.

---

## 6. Search

Search is Iris Mail's headline feature. It must be instant, expressive, and obvious.

### 6.1 Query language

A GitHub-style query language parsed by a hand-rolled parser combinator (`chumsky` crate). Full grammar in `docs/search-syntax.md`. Examples:

```
from:terry subject:invoice has:attachment after:2025-01-01 widgets
in:inbox is:unread
to:me from:@fastell.co.za
filename:*.pdf larger:5mb
"exact phrase match"
```

Supported operators:

| Operator | Meaning |
|---|---|
| `from:` | Sender address or name |
| `to:` / `cc:` / `bcc:` | Recipient matching |
| `subject:` | Subject line |
| `in:` | Folder name |
| `has:attachment` | Attachment presence |
| `is:unread` / `is:read` / `is:flagged` / `is:answered` | Flag state |
| `before:` / `after:` | Date range |
| `larger:` / `smaller:` | Size range |
| `filename:` | Attachment filename |
| `account:` | Filter to one account |
| `"..."` | Exact phrase |
| `-term` | Negation |

Unqualified terms search the full-text index across subject, body, and headers.

### 6.2 Execution

Structured operators become SQL `WHERE` clauses on indexed columns. Free-text terms become an FTS5 `MATCH` expression. The two are combined in a single query, so a complex search hits the index once and returns in milliseconds even on mailboxes with hundreds of thousands of messages.

Results are ranked by FTS5's built-in BM25 scoring, with a recency boost so a fresh matching message outranks a five-year-old one.

### 6.3 Virtual folders

Saved searches appear in the sidebar as virtual folders. Built-in set:

- **By Year** — auto-generated per-year buckets
- **By Sender** — frequent senders, grouped
- **Has Attachments**
- **Large Messages** (> 10MB)
- **Unread Everywhere**

Users can save any search as a custom virtual folder.

---

## 7. Storage Tiering

Iris Mail's signature feature. Automates the painful dance of keeping an M365 mailbox under quota without losing access to old mail.

### 7.1 Concept

Per account, the user configures two thresholds:

- **Synced tier** (default 30 GB): messages are on both the server and local disk. Full two-way sync.
- **Local archive tier** (unlimited): messages are on local disk only. The server has moved on.

When the server-side mailbox approaches the synced tier limit, the oldest messages *that are confirmed stored locally and verified intact* are moved to a designated `Iris Archive` folder on the server and then removed from the active mailbox (or deleted entirely if the user opts in). The local copy is untouched. Search continues to find them. The reading pane opens them instantly. The user's experience is unchanged — their mail is just *there* — but their server quota stays comfortable.

### 7.2 Safety rails

Because this feature touches the "delete from server" button, it is designed with paranoia:

1. **Integrity verification** — before any server-side action, the local message body and attachments are re-read and checksummed. Any mismatch aborts the operation and logs an error.
2. **Minimum age** — messages newer than a configurable floor (default 90 days) are never archived regardless of quota pressure.
3. **Dry run by default** — the first time tiering runs on an account, it produces a report ("I would archive 1,247 messages totalling 28 GB — proceed?") and waits for user confirmation. Subsequent runs can be auto-approved or kept manual per the user's preference.
4. **Reversible archive** — the default action is "move to `Iris Archive` folder on the server", not delete. Users can recover from the server for as long as the archive folder exists.
5. **Audit log** — every tiering action is written to an `audit_log` table with timestamp, message UID, action, and outcome. Surfaced in the UI as a "Storage activity" view.
6. **Never touches Sent / Drafts / Trash** — tiering only operates on Inbox and user-created folders by default.

### 7.3 Local/remote indicator

The message list shows a small two-dot glyph next to each message:

- ● ● both filled — synced both places
- ● ○ left only — local archive only
- ○ ● right only — remote only (headers present, body not yet downloaded)
- ○ ○ neither — should never happen; indicates a bug

The user always knows where their mail lives.

---

## 8. Accounts & Authentication

### 8.1 Flow

Adding an account:

1. User clicks "Add Account" → chooses Microsoft 365, Gmail, or "Other (IMAP)".
2. For M365 and Gmail, Iris Mail opens the system browser to the provider's OAuth consent page.
3. A local HTTP listener (via `tauri-plugin-oauth`) catches the redirect on `http://localhost:<random-port>/callback`.
4. The authorization code is exchanged for access + refresh tokens.
5. Refresh token is stored in the OS keychain keyed by a UUID. The `accounts` table stores only the UUID.
6. Initial sync begins.

For generic IMAP, a traditional server/port/username/password form is shown. Passwords also go to the keychain.

### 8.2 Token lifecycle

- Access tokens are acquired on demand and held in memory only.
- Refresh tokens live in the keychain.
- Token refresh is automatic and transparent to the rest of the app — `iris-mail` exposes a `get_valid_access_token(account_id)` function that handles expiry.
- Google's 7-day refresh token expiry for unverified OAuth apps is a known issue. During development, accounts are added to the Google Cloud project test user list. For release, Iris Mail will go through Google's verification process.

### 8.3 Keychain backends

- Windows → Credential Manager
- Linux → Secret Service (via libsecret / GNOME Keyring / KWallet)

---

## 9. User Interface

### 9.1 Layout

Classic three-pane, Outlook-familiar, pixel-considered:

```
┌─────────────┬──────────────────┬──────────────────────┐
│             │                  │                      │
│  Accounts   │  Message list    │  Reading pane        │
│  & folders  │                  │                      │
│             │                  │                      │
│             │                  │                      │
└─────────────┴──────────────────┴──────────────────────┘
```

- **Left pane**: Account list (each account with its accent dot), folder tree, virtual folders, storage indicator.
- **Middle pane**: Virtualised message list. Sender, subject, preview, date, local/remote dots, attachment paperclip.
- **Right pane**: Reading view. Sanitised HTML, attachment strip, action toolbar (reply, forward, archive, delete, flag, move).

Toolbar across the top: compose, search (prominent, cmd/ctrl-K), refresh, account switcher.

### 9.2 Visual design

- **Typeface**: Inter for UI, JetBrains Mono for monospace.
- **Palette**: Catppuccin — Latte (light), Frappé, Macchiato, Mocha (dark). User-selectable, with an automatic option that follows the system theme.
- **Accent**: Catppuccin Mauve by default. Per-account accents drawn from Catppuccin's named colours (Red, Peach, Yellow, Green, Sapphire, Mauve, Lavender).
- **Motion**: Subtle. Fades under 150ms for state changes. No bounce, no elastic. Respect `prefers-reduced-motion`.
- **Density**: Three density modes — Comfortable, Normal, Compact. Normal is the default.

### 9.3 HTML email rendering

Messages render in a sandboxed webview with a strict CSP:

- JavaScript disabled
- Remote images blocked by default with a per-sender whitelist
- External links intercepted and opened in the system browser with a confirmation for suspicious URLs
- Styles sanitised via `ammonia` to strip `position: fixed`, `javascript:` URLs, and other abuse vectors

A banner at the top of each message shows security state: "Remote images blocked — Show | Always show from this sender".

### 9.4 Virtualisation

Folder tree and message list are virtualised. No mailbox size causes UI lag. Scrolling is smooth at 60fps on modest hardware.

---

## 10. PST / OST Import

### 10.1 v1: PST only

PST is a documented-enough format that import is feasible. Uses `libpff` via Rust bindings. Flow:

1. User chooses "Import from PST" and selects a file.
2. Iris Mail parses the PST and shows a preview: folder structure, message count, total size.
3. User picks a destination (existing account folder or a new local-only folder).
4. Import runs in the background with progress reporting, writing directly into the SQLite store.
5. Imported messages are marked `stored_local = true, stored_remote = false` and are indexed for search immediately.

### 10.2 OST: deferred

OST files are encrypted against the originating Outlook profile and are usually impossible to import without that profile. For users with OST files the pragmatic answer is to connect the live M365 account, which gives them everything in the OST through sync. OST import is deferred to v2 pending user demand.

---

## 11. Security

- No telemetry. None. Not even opt-in for v1.
- All credentials in the OS keychain.
- All network traffic over TLS; certificate verification cannot be disabled.
- HTML sandboxed, JS disabled, remote content opt-in.
- External links confirmed before opening.
- Database file readable only by the current user (Unix mode 600, Windows ACL equivalent).
- Optional full database encryption at rest via SQLCipher — deferred to v1.1, designed for from the start.
- Dependency audits via `cargo audit` in CI.

---

## 12. Performance Targets

These are commitments, not aspirations. CI includes benchmarks that fail the build if any of these regress.

| Operation | Target |
|---|---|
| Cold start to usable UI | < 1.5s |
| Folder switch | < 50ms |
| Open a message in the reading pane | < 100ms |
| Search across 100k messages | < 200ms |
| Search across 1M messages | < 1s |
| Memory footprint (idle, 3 accounts, 100k messages) | < 300MB |
| Binary size (release, stripped) | < 30MB |

---

## 13. Development Phases

### Phase 1 — Foundations (spec → working skeleton)

Workspace setup, CI, `iris-core` types, `iris-db` schema and migrations, Tauri shell with a placeholder three-pane UI, Catppuccin theming. Deliverable: app opens, shows empty panes, closes cleanly.

### Phase 2 — Mail protocol

OAuth flow for M365 and Gmail, IMAP connection and folder discovery, basic message fetching. Deliverable: add account, see folder tree, read most recent 50 messages.

### Phase 3 — Sync engine

Progressive sync, ongoing IDLE, offline outbox, state machine. Deliverable: full 60-day window syncs reliably, new mail arrives in real time, offline compose queues and sends on reconnect.

### Phase 4 — Search

Query parser, FTS5 integration, ranked results, virtual folders. Deliverable: the search-Outlook-envy moment.

### Phase 5 — Storage tiering

Tier configuration, background enforcement, safety rails, audit log, local/remote indicators. Deliverable: the signature feature ships.

### Phase 6 — Polish

PST import, attachment deduplication, density modes, accessibility pass, performance tuning. Deliverable: v1.0.

---

## 14. Open Questions

Flagged for later decision, not blocking a start:

1. Should the query language support `OR` and grouping with parentheses in v1, or keep it flat AND-only?
2. Threading view: Gmail-style conversation grouping or Outlook-style flat list as default? (Probably offer both, default to flat.)
3. Should virtual folders be stored per-account or globally across all accounts?
4. What's the upgrade/migration story when the schema changes? (sqlx handles it, but user-facing communication matters.)

---

## 15. Licence

Dual-licensed under **MIT OR Apache-2.0**, at the user's option. This is the Rust ecosystem standard and gives users maximum flexibility while providing patent protection via Apache 2.0. Contributors agree their contributions are licensed under the same terms.

---

*End of specification. See `CLAUDE.md` for contributor conventions and `docs/` for deeper dives.*
