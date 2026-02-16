# AGENTS.md - oc-outpost

Telegram bot (Rust) that orchestrates multiple OpenCode instances through forum topics.
Tech stack: teloxide, tokio, sqlx (SQLite), axum, reqwest, serde, anyhow/thiserror, bollard (Docker).

**WARNING**: This codebase was put together quickly. Many existing patterns are antipatterns.
This document describes what to DO, not necessarily what the code currently does.

## Build / Lint / Test Commands

```bash
just check              # Run all checks: fmt-check + clippy + test
just fmt                # Format code (cargo fmt)
just fmt-check          # Check formatting without modifying
just clippy             # cargo clippy --all-targets --all-features -- -D warnings
just test               # cargo test --lib
just build              # cargo build --release
just run                # cargo run (dev mode)
just lint               # fmt + clippy in one command

# Single test
cargo test --lib test_name_here
# Tests in a specific module
cargo test --lib config::tests
cargo test --lib orchestrator::store::tests
# Run with output
cargo test --lib test_name_here -- --nocapture
```

Clippy treats warnings as errors. Always run `just check` before submitting.

## Project Structure

```
src/
  main.rs              # Entry point, teloxide dispatcher setup
  config.rs            # Config from env vars (dotenvy)
  integration.rs       # Wires bot <-> OpenCode (message routing, streaming)
  bot/                 # Telegram bot: commands, handlers/, state
  orchestrator/        # Instance lifecycle: manager, store, port_pool, container
  opencode/            # OpenCode API client, discovery, stream_handler
  types/               # Shared types: error, instance, forum, Opencode
  db/                  # Database init, log_store, tracing_layer
  api/                 # External HTTP API (axum)
  forum/               # Topic store
  git/                 # Git/worktree operations
  telegram/            # Telegram utilities (markdown conversion)
migrations/            # SQLite migration SQL files
```

## Error Handling

Two error tiers, separated by **contract boundary** (not "process boundary"):

- **`anyhow::Result<T>`** — Internal application code where callers don't need to match on error variants. Allowed broadly (not just startup), but never exposed across handler/API/integration boundaries. Use `.context(...)` / `.with_context(...)` aggressively.
- **`OutpostError` (thiserror)** — Typed error contract for handler/integration/API boundaries where callers need structured handling (HTTP status, Telegram reply, retry policy, metrics). Prefer `#[from]` for propagation and `#[source]` for chains. Use `types::error::Result<T>` in those layers.

**Rules:**
- Do not stringify errors for propagation (`map_err(|e| e.to_string())`, `format!(...)`); it discards type + `source()` chain. Wrap with `#[from]`/`#[source]` (typed) or `.context(...)` (anyhow). Stringify only at final presentation boundaries (logs, user messages), and even then prefer `?e` formatting plus a human message.
- Prefer direct enum construction and `#[from]` conversions. Add constructors only when they enforce invariants, hide boxing/type-erasure, or prevent repeated inconsistent boilerplate.
- Add error variants when you have a real call site and the variant carries meaning (distinct handling, status mapping, metrics tag). No speculative "future" variants. If an error enum is part of a public contract, consider `#[non_exhaustive]` instead.
- If you keep `is_user_error()`, make it an exhaustive `match` and treat it as a routing hint, not the sole log-level source. Choose severity at the call site using both operation context and error classification. If it can't be kept accurate, remove it.

## Code Style

### Formatting & Linting
- **rustfmt.toml** pins Rust's default style (100-char max, 4-space indent) and forces `newline_style = "Unix"` for stable cross-OS diffs; imports are reordered (`reorder_imports = true`). Use `cargo fmt`; don't hand-format.
- **clippy.toml** relaxes two knobs above defaults (`too-many-arguments-threshold`: 8 vs default 7; `type-complexity-threshold`: 500 vs default 250). Treat both as "danger zone" — if you're near these limits, refactor (bundle args into structs, add type aliases/newtypes) instead of normalizing complexity.
- Clippy: all warnings are errors (`-D warnings`).

### Imports & Re-exports
Let rustfmt handle import ordering — don't hand-maintain groups. Avoid `pub use some_module::*` at public module roots. Allowed exceptions: small curated `prelude` modules, tightly-scoped leaf aggregation (e.g., error/rejection types), and internal/test-only modules. In `src/bot/mod.rs`, replace `pub use handlers::*;` with an explicit export list.

### Module Organization
- One handler per file in `src/bot/handlers/` — good, keep it.
- Use `//!` for crate/module-level docs when the module has non-obvious purpose or invariants. Use `///` on public types/functions. Don't add boilerplate docs just to satisfy a ritual.

### Serde
- Internal/DB types: keep Rust naming; don't add serde casing attributes unless the type is part of a wire format.
- Enums exposed over the wire: pick an explicit wire casing (often `snake_case`) via `#[serde(rename_all = "snake_case")]`.
- HTTP API DTOs: choose ONE casing for the public API based on consumers; apply consistently with `#[serde(rename_all = "...")]`. Don't embed raw internal structs in API responses.
- Optional response fields: use `#[serde(skip_serializing_if = "Option::is_none")]` when "absent" == "not applicable"; don't use it if clients must distinguish `null` vs missing. Optional request fields: prefer `Option<T>` with `#[serde(default)]`.
- Derive only what's needed. Don't cargo-cult `PartialEq` onto types that never get compared.

## Async Patterns

### Choosing Concurrency Primitives
Default to **no shared mutable state** when practical.

1. **Immutable / already-concurrent handles** (config, `sqlx::Pool`, clients that are `Clone + Send + Sync`): prefer `T: Clone` or `Arc<T>`. Do **not** wrap in a Mutex.
2. **In-memory data with short critical sections** (maps, caches, counters): prefer `std::sync::Mutex` / `std::sync::RwLock` (or `parking_lot`) **as long as you do not hold the guard across `.await`**. Consider atomics for simple counters/flags.
3. **State requiring async work while "logically locked"** (serialized protocol, single shared connection): prefer an **owner task + message passing** (`tokio::sync::mpsc` + `oneshot`). If you truly must lock across `.await`, use `tokio::sync::Mutex`, and keep the locked region small.
4. **Read-mostly evolving state**: `tokio::sync::RwLock` only when you need async locking; otherwise prefer sync `RwLock` or `tokio::sync::watch` for config-like state.

**Existing antipattern**: `BotState` wraps pool-backed stores in `Arc<Mutex<>>`, serializing all DB access for zero benefit. Stores are already concurrency-safe via their connection pools. Don't copy this.

### Lock Discipline (CRITICAL)
**Rule 1 (hard):** Never hold `std::sync::MutexGuard` / `RwLockGuard` across `.await`. Prefer the "wrapper" pattern: lock only inside non-async methods so the guard cannot outlive the method.

**Rule 2 (soft):** You *may* hold `tokio::sync::MutexGuard` / `RwLockGuard` across `.await` (they are designed for it), but treat it as a last resort:
- Keep the locked region small.
- Never `.await` something that might try to take the same lock (deadlock).
- Avoid re-entrant patterns with `RwLock` (read -> write -> read can deadlock per tokio docs).

**Default:** If you're tempted to hold an async lock across I/O, consider an owner-task + message passing design instead.

### Channels & Coordination
- `tokio::sync::mpsc` is the default for "many producers -> one consumer loop" (work queues, actor mailboxes). For request/response, pair with `oneshot`.
- If every consumer must see every message, use `broadcast`. If only the latest value matters (config/shutdown state), use `watch`.

### Task Lifecycle
- For long-lived services/bots: every background task should have a shutdown signal path, and we should wait for completion.
  - Preferred: `CancellationToken` to signal + `TaskTracker` (or `JoinSet`) to wait.
  - Intentionally detached ("fire-and-forget") tasks are allowed only when loss on exit is acceptable and documented.
- Prefer cooperative shutdown so tasks can clean up. `abort()` is a valid fallback (e.g., after a timeout or to break cycles), but still **wait for cancellation** (e.g., `JoinSet::shutdown()`, or `handle.await` expecting `JoinError::is_cancelled()`).
- Never rely on "drop the JoinHandle" as shutdown; that just detaches the task.

## Logging

Uses `tracing` with structured fields.

**Rules:**
- Use `#[instrument]` on handler/service boundaries for span correlation, but default to `skip_all` and explicitly whitelist safe fields (IDs, counts, state). Don't add "entered function" `debug!` logs — either use a span or log a meaningful state change.
- Log mutations (create/update/delete) at `info` with entity IDs. Log reads at `trace` or not at all.
- Never log Telegram message text, OpenCode prompt content, request/response bodies, or credentials/tokens. Prefer allowlisting: log IDs/metadata only. Consider type-wrapping secrets (e.g., `secrecy`) so accidental `Debug` doesn't leak.
- `warn!` for expected operational issues; `error!` for genuine failures needing investigation.
- The DB tracing layer (`src/db/tracing_layer.rs`) spawns an async insert per event (unbounded, no backpressure) and silently drops insertion errors. Treat as a dev/diagnostic feature unless you add filtering/sampling and a bounded ingestion path.

## Axum API Patterns

Handler signatures follow standard Axum (`State`, `Json`, `impl IntoResponse`). Keep `create_router()`.

**Existing issues — don't perpetuate:**
- For JSON endpoints: return a consistent JSON error body for all non-success responses; implement a single `ApiError` that maps domain failures to `(StatusCode, Json<...>)` via `IntoResponse`. Prefer RFC 9457 Problem Details for external APIs; `{code, message}` is fine for internal APIs if consistent. Include a `request_id` when possible.
- Don't echo internal errors to clients (including `Display`/`Debug` output, backtraces, sqlx errors). Log the full error server-side; return a stable, safe message. Exception: for 4xx validation errors, return structured client-actionable details that don't include secrets or internals.
- Prefer `tower_http::trace::TraceLayer::new_for_http()` for request/response spans; don't log bodies; treat headers as sensitive-by-default — apply `SetSensitiveRequestHeadersLayer`/`SetSensitiveResponseHeadersLayer` (Authorization/Cookie/Set-Cookie at minimum).
- If the API is not called by browsers: omit CORS entirely. If browsers call it: allowlist explicit origins; never use `CorsLayer::permissive()` in production.

## Testing

### What works
- **Inline unit tests** (`#[cfg(test)] mod tests`) for pure/isolated modules.
- **`tempfile::TempDir`** for file-backed SQLite test DBs — realistic and isolated.
- **`tower::ServiceExt::oneshot`** for API endpoint testing.

### Known issues — don't perpetuate
- **No black-box integration tests** (`tests/`). We only have in-crate tests; we don't exercise the compiled binary / public surface the way production does.
- **API tests only check status codes** — also assert JSON response bodies and DB side effects. Test at least one failure mode per endpoint.
- **Be careful with SQLite in-memory + pools.** Raw SQLite `:memory:` creates one DB per connection. Prefer per-test file DBs (`TempDir` or `#[sqlx::test]`). If you insist on in-memory, use a shared URI (e.g., `file::memory:?cache=shared`); `max_connections(1)` is a last-resort workaround, not a default.
- **Env-based config tests are currently serialized** (`serial_test` + manual cleanup). Prefer (a) parsing from an injected source for true parallelism, or (b) scoped env guards like `temp_env::with_vars(...)`.
- **Test helpers are duplicated.** For unit tests, factor common helpers into `src/test_utils.rs` behind `#[cfg(test)]`. For integration tests, use `tests/common/mod.rs` or a small dev-only support crate.
- **Avoid asserting full error strings** unless the string is a user-facing contract. Prefer asserting the error type/variant and, if needed, a stable identifier/subset.
