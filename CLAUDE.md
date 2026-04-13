# CLAUDE.md — Iris Mail Contributor Conventions

This file is the guide for anyone contributing to Iris Mail, whether human or AI. It is written primarily so that Claude Code (and other AI coding assistants) can make changes that feel consistent with the rest of the codebase on the first try. Humans should read it too.

Iris Mail is a product we care deeply about. Code should reflect that care. Beautiful code is not an affectation — it is how we keep the project maintainable, contributable, and correct over the long run.

---

## Core Principles

1. **Clarity over cleverness.** If a reader has to stop and puzzle out what a line does, rewrite it.
2. **DRY, but not dogmatically.** Duplicate twice, extract on the third. Premature abstraction is worse than duplication.
3. **Small functions, clear names.** A function should do one thing. Its name should say what that thing is.
4. **Types are documentation.** Use Rust's type system to make illegal states unrepresentable. An `Option<NonEmptyString>` is worth more than a comment.
5. **Errors are values, and they are specific.** No `unwrap()` in production paths. No `String` errors. Every fallible function returns a typed error.
6. **Async all the way down.** The mail protocol is async. The database is async. The UI is event-driven. Don't block.
7. **Test the boundaries.** Unit-test pure logic, integration-test the crate seams, end-to-end-test the sync engine against a mock IMAP server.

---

## Rust Conventions

### Formatting and linting

These run in CI. They must pass before a PR merges.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Local setup should run `cargo fmt` on save. A `rustfmt.toml` at the workspace root pins the style; do not override it per-file.

### Naming

- `snake_case` for functions, variables, modules, files.
- `CamelCase` for types, traits, enum variants.
- `SCREAMING_SNAKE_CASE` for constants and statics.
- Acronyms are words: `ImapClient`, not `IMAPClient`. `HttpServer`, not `HTTPServer`. `OauthToken`, not `OAuthToken`.
- Boolean fields and functions read as questions: `is_synced`, `has_attachment`, `can_refresh`.

### Error handling

- Library crates (`iris-core`, `iris-db`, `iris-mail`, etc.) define their errors with `thiserror`. Each crate has one public `Error` enum and a `Result<T> = std::result::Result<T, Error>` alias.
- The application crate (`iris-app`) uses `anyhow::Result` at command boundaries for ergonomic error chaining.
- Never `.unwrap()` or `.expect()` in production code except where a panic genuinely indicates a bug (e.g. a `RwLock` that cannot be poisoned by design). Comment such cases.
- Propagate with `?`. Convert with `From` implementations or `.map_err()`.
- User-facing error messages are written in `iris-app`, not deep in the stack. Lower layers return structured errors; the app translates them.

### Async

- `tokio` is the runtime. Do not mix runtimes.
- Spawn long-running work with `tokio::spawn`. Structured concurrency via `tokio::task::JoinSet` for fan-out.
- Never call blocking code in an async context. Use `tokio::task::spawn_blocking` for genuinely blocking work (e.g. `libpff` PST parsing).
- Cancellation matters. Respect cancellation tokens in background jobs so shutdown is clean.

### Database

- All SQL lives in `iris-db`. Other crates call repository methods, never construct queries.
- Use `sqlx::query!` and `sqlx::query_as!` for compile-time checked queries wherever possible.
- Migrations live in `crates/iris-db/migrations/` and are applied automatically on startup.
- Transactions are explicit. If an operation touches more than one table, it takes a `&mut Transaction`.
- The `messages_fts` virtual table is kept in sync via SQL triggers, not application code.

### Modules and visibility

- Default visibility is private. Make things `pub` only when needed by another crate.
- Use `pub(crate)` liberally for crate-internal exposure.
- Re-export a crate's public API from its `lib.rs`. Users should not have to know the internal module structure.

### Comments and documentation

- Every public item (`pub fn`, `pub struct`, etc.) has a doc comment.
- Doc comments explain *what* and *why*, not *how*. The code is the *how*.
- Use `//` for in-line comments sparingly, and only to explain non-obvious decisions or to flag caveats.
- `TODO` comments include a GitHub issue reference: `// TODO(#42): handle this case`.
- No commented-out code. Delete it. Git remembers.

### Testing

- Unit tests live in the same file as the code they test, in a `#[cfg(test)] mod tests` block.
- Integration tests live in `tests/` at the crate root.
- Test names read as sentences: `#[test] fn message_bodies_are_sanitised_before_rendering()`.
- Use `assert_eq!` with meaningful values, not `assert!(x == y)`.
- Prefer property tests (`proptest` crate) for parsers and serialisation.
- The IMAP layer is tested against a mock server so CI doesn't need the internet.

---

## Frontend (Svelte) Conventions

### Structure

- Components live in `ui/src/lib/components/`, grouped by feature.
- Routes (if any) live in `ui/src/routes/`.
- Stores (Svelte 5 runes-based) live in `ui/src/lib/stores/`.
- Tauri command bindings live in `ui/src/lib/api/` — one module per backend crate, each function a thin wrapper around `invoke()`.

### Style

- Tailwind CSS utility classes. No custom CSS except for Catppuccin theme variables and unavoidable one-offs.
- Component files use `<script lang="ts">`. TypeScript throughout.
- Props are typed. No `any`.
- Event names are past-tense verbs: `on:messageSelected`, not `on:selectMessage`.
- One component per file. Filenames in PascalCase matching the component name.

### State

- Local state with `$state` runes.
- Shared state in stores. Stores are small and focused; do not create a single "app store."
- Derived values with `$derived`, not manual subscriptions.
- Effects (`$effect`) are a last resort. Prefer derived state.

### Tauri interop

- Never call `invoke()` directly in a component. Always go through `ui/src/lib/api/`.
- API modules catch errors, convert them to typed results, and let components render error states.
- Events from the backend are subscribed to in stores, never in components.

---

## Commit and PR Conventions

### Commit messages

Conventional Commits. Examples:

```
feat(search): add `larger:` size filter
fix(sync): recover from UIDVALIDITY change on reconnect
refactor(db): extract attachment repo into its own module
docs(spec): clarify storage tier safety rails
test(mail): add IMAP IDLE reconnection test
chore(deps): bump sqlx to 0.8.3
```

The scope in parentheses is the crate or area affected.

### Pull requests

- One concern per PR. A feature and a refactor are two PRs.
- PR description explains *why*, not just *what*. Link the issue it closes.
- CI must pass: `fmt`, `clippy`, `test`, and the performance benchmark gate.
- At least one reviewer for non-trivial changes.
- Squash on merge. The commit message becomes the PR title.

---

## Working with Claude Code

When Claude Code is asked to make a change, the expectation is:

1. **Read before writing.** Check `iris-mail-spec.md`, this file, and any relevant `docs/*.md` before touching code.
2. **Respect the architecture.** Changes should land in the right crate. If unsure, ask. Cross-crate refactors need discussion first.
3. **Write the test first when fixing a bug.** Reproduce the bug with a failing test, then make it pass.
4. **Run the checks locally before proposing a commit.** `cargo fmt`, `cargo clippy`, `cargo test`.
5. **Keep PRs small.** A 200-line PR that is easy to review is worth more than a 2000-line one that is not.
6. **Ask about unclear requirements.** The spec is the source of truth, but it has gaps. Flag them and propose a resolution rather than guessing.
7. **Be conservative about dependencies.** Every new crate in `Cargo.toml` is a long-term commitment. Justify additions in the PR description.

---

## File Layout Cheat Sheet

```
crates/iris-core/src/
  lib.rs              # Re-exports
  error.rs            # Error enum
  account.rs          # Account type
  folder.rs           # Folder type
  message.rs          # Message type
  attachment.rs       # Attachment type

crates/iris-db/src/
  lib.rs              # Pool, migrations
  schema/             # Migration SQL files
  repo/
    account.rs
    folder.rs
    message.rs
    attachment.rs
    audit_log.rs

crates/iris-mail/src/
  lib.rs
  oauth/
    m365.rs
    gmail.rs
  imap/
    client.rs
    idle.rs
    sync.rs
  smtp.rs

crates/iris-search/src/
  lib.rs
  parser.rs           # Query language
  executor.rs         # SQL + FTS5

crates/iris-sync/src/
  lib.rs
  engine.rs           # Orchestrator
  jobs/
    initial.rs
    backfill.rs
    idle.rs
    tiering.rs
  outbox.rs

crates/iris-import/src/
  lib.rs
  pst.rs

crates/iris-app/src/
  main.rs
  commands/           # Tauri commands, grouped by feature
  events.rs           # Event emission helpers
  setup.rs            # Startup wiring
```

---

## Questions?

Open a GitHub Discussion. For architectural questions, tag `@Woogy7`. For anything that might change this file, open a PR against it — conventions should evolve with the project, but deliberately.

Welcome aboard. Let's build something worth using.
