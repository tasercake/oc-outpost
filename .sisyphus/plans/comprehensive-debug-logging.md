# Comprehensive Debug Logging Instrumentation

## TL;DR

> **Quick Summary**: Add verbose debug-level logging to ALL 40 source files in oc-outpost, transforming a near-silent codebase (69 log calls in 5 files) into a fully observable system where every decision point, state transition, and data flow is traceable. Also investigate and improve logging around 2 known error paths.
> 
> **Deliverables**:
> - Debug/info logging added to all 35 currently unlogged source files
> - Enhanced logging on 5 files that already have partial logging
> - Improved error investigation logging for SessionNotFound and TelegramError paths
> - Consistent structured field naming across all log calls
> 
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 2 waves (10 tasks in Wave 1, 1 in Wave 2)
> **Critical Path**: All tasks in Wave 1 are independent → Wave 2 (build verification)

---

## Context

### Original Request
Add comprehensive debug-level logging throughout the entire oc-outpost codebase. Currently only 69 log calls exist across 5 files; 35 files have zero instrumentation. Success paths are completely invisible — you cannot trace a message from Telegram through OpenCode and back. Also investigate the SessionNotFound error (caused by `/new` creating TopicMapping with `session_id: None`) and improve logging around that path.

### Interview Summary
**Key Discussions**:
- User wants `debug!` level logs everywhere so workings are visible when `RUST_LOG=oc_outpost=debug` is set
- DatabaseLayer already captures ALL levels to SQLite, so debug logs are always persisted regardless of env filter
- Each file is an independent edit — highly parallelizable
- No architectural changes needed, purely additive instrumentation

**Research Findings**:
- **40 .rs source files total**: 5 have some logging (main.rs: 29, integration.rs: 16, manager.rs: 12, stream_handler.rs: 11, disconnect.rs: 1), 35 have zero
- **SessionNotFound root cause**: `/new` handler (new.rs:127-137) creates TopicMapping with `session_id: None`. When user sends a message, integration.rs:116 checks `mapping.session_id` and returns SessionNotFound. The session creation happens later via `/connect` or first message routing — but the debug trail is invisible.
- **TelegramError "no thread_id"**: Already reasonably logged in main.rs:254-266 with `topic_id: None` context. Just needs a debug log at the START of `handle_message` showing incoming message metadata.
- **Tracing setup**: `tracing` v0.1, `tracing-subscriber` v0.3 with `env-filter`. Three layers: EnvFilter → fmt → DatabaseLayer. Default: `oc_outpost=info`.

### Gap Analysis
**Addressed**:
- Logging level choice: Use `debug!` for internal mechanics, `info!` for significant state changes (instance created, message routed)
- Structured fields: Use consistent names — `topic_id`, `instance_id`, `session_id`, `project_path`, `port`, `sender_id`
- Sensitive data: Never log `telegram_bot_token` or `api_key` values. Config already has masked Display impl.
- Performance: Debug logs behind env filter by default. DatabaseLayer accepts all levels by design.

---

## Work Objectives

### Core Objective
Instrument every source file with structured debug/info logging so that operators can trace any request through the entire system by setting `RUST_LOG=oc_outpost=debug`.

### Concrete Deliverables
- Every function entry/exit at key decision points has a `debug!` call
- Every DB query, HTTP request, state transition, and resource allocation is logged
- Two error paths (SessionNotFound, TelegramError) have enhanced diagnostic logging
- All log calls use consistent structured field names

### Definition of Done
- [ ] `cargo build` succeeds with zero errors
- [ ] `cargo clippy -- -D warnings` passes clean
- [ ] `cargo test` passes (existing tests still work)
- [ ] Running with `RUST_LOG=oc_outpost=debug` produces visible debug output for all major code paths

### Must Have
- `use tracing::{debug, info, warn, error};` import in every file that gains logging
- Structured fields on all log calls (not just string interpolation)
- Log calls at function entry for all public functions
- Log calls at every decision branch (if/match arms)

### Must NOT Have (Guardrails)
- **DO NOT** log sensitive values: `telegram_bot_token`, `api_key` values, message content beyond truncated previews
- **DO NOT** change any existing log levels (don't downgrade existing `info!` to `debug!` or upgrade `debug!` to `info!`)
- **DO NOT** add `trace!` level — stick to `debug!` for new fine-grained logs, `info!` for significant events
- **DO NOT** modify any business logic, control flow, or function signatures
- **DO NOT** add logging to `types/` module files (error.rs, opencode.rs, forum.rs, instance.rs) — these are pure data types
- **DO NOT** add logging to `mod.rs` files — these are just re-exports
- **DO NOT** modify test code — only add logging to production code paths
- **DO NOT** log full message bodies — truncate to first ~100 chars using char-boundary-safe method: `&text[..text.char_indices().take(100).last().map(|(i, c)| i + c.len_utf8()).unwrap_or(text.len())]` or simpler: `text.chars().take(100).collect::<String>()` for preview fields
- **DO NOT** add tracing spans or #[instrument] attributes — just macro calls (keeps changes minimal and reviewable)

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES (cargo test, 50+ existing tests)
- **User wants tests**: NO — This is logging instrumentation. Tests verify behavior, not log output.
- **Framework**: cargo test (existing)

### Automated Verification

Each task will be verified by:

```bash
# Must pass — no compile errors from new logging
cargo build 2>&1

# Must pass — no clippy warnings from unused imports or dead code
cargo clippy -- -D warnings 2>&1

# Must pass — existing tests still work
cargo test 2>&1
```

Visual verification (run once at end):
```bash
# Start with debug logging to verify output
RUST_LOG=oc_outpost=debug cargo run 2>&1 | head -100
```

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Start Immediately — ALL independent):
├── Task 1: config.rs + main.rs (startup/config)
├── Task 2: integration.rs (message routing + error investigation)
├── Task 3: orchestrator/manager.rs (instance lifecycle)
├── Task 4: orchestrator/instance.rs + store.rs + port_pool.rs
├── Task 5: opencode/client.rs + stream_handler.rs
├── Task 6: opencode/discovery.rs
├── Task 7: bot/handlers/ (all 11 handler files)
├── Task 8: forum/store.rs + db/log_store.rs + db/tracing_layer.rs
├── Task 9: api/mod.rs
└── Task 10: telegram/markdown.rs

Wave 2 (After ALL Wave 1 complete):
└── Task 11: Final build verification + integration check
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 11 | 2-10 |
| 2 | None | 11 | 1, 3-10 |
| 3 | None | 11 | 1-2, 4-10 |
| 4 | None | 11 | 1-3, 5-10 |
| 5 | None | 11 | 1-4, 6-10 |
| 6 | None | 11 | 1-5, 7-10 |
| 7 | None | 11 | 1-6, 8-10 |
| 8 | None | 11 | 1-7, 9-10 |
| 9 | None | 11 | 1-8, 10 |
| 10 | None | 11 | 1-9 |
| 11 | 1-10 | None | None (final) |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Agents |
|------|-------|-------------------|
| 1 | 1-10 | All parallel: `category="quick"`, each task independent file edits |
| 2 | 11 | Single: `category="quick"`, build + clippy + test verification |

---

## Logging Conventions (APPLY TO ALL TASKS)

### Import Pattern
Every file that gains logging must have this import at the top (add only what's needed):
```rust
use tracing::{debug, info};  // Add warn, error only if used
```

### Structured Field Names (Mandatory Consistency)
| Field | Type | Usage |
|-------|------|-------|
| `topic_id` | `i32` | Telegram forum topic thread ID |
| `chat_id` | `i64` | Telegram chat ID |
| `session_id` | `&str` or `?Option` | OpenCode session ID |
| `instance_id` | `&str` or `?Option` | OpenCode instance ID |
| `project_path` | `%display` or `&str` | Project directory path |
| `port` | `u16` | Network port |
| `sender_id` | `?Option<u64>` | Telegram user ID |
| `sender_username` | `?Option<&str>` | Telegram username |
| `url` | `&str` | HTTP request URL |
| `status` | `u16` or `&str` | HTTP response status |
| `method` | `&str` | HTTP method |
| `count` | `usize` | Item counts |
| `elapsed_ms` | `u128` | Duration in milliseconds |

### Log Level Guide
| Level | When to Use |
|-------|-------------|
| `debug!` | Internal mechanics: DB queries, HTTP calls, lookups, decision branches, batch sizes |
| `info!` | Significant state changes: instance created/stopped, stream connected, config loaded (ONLY for new significant events not already logged) |
| `warn!` | Already used for recoverable errors — don't add new warn! |
| `error!` | Already used for hard errors — don't add new error! |

### Message Text Truncation Pattern
When logging user message content, always truncate (UTF-8 safe):
```rust
// SAFE: .chars().take(N) respects char boundaries (no panic on multi-byte)
let preview: String = text.chars().take(100).collect();
debug!(text_preview = %preview, text_len = text.len(), "Processing message");
```
**WARNING**: Do NOT use `&text[..100]` — this panics on multi-byte UTF-8 characters. Always use `.chars().take(N).collect::<String>()`.

---

## TODOs

- [ ] 1. Startup & Config Logging (config.rs + main.rs)

  **What to do**:
  
  **config.rs** — Log each config field as it's loaded, distinguishing env var vs default:
  - At end of `from_env()` before `Ok(Config{...})`: Add `debug!` logging which env vars were explicitly set vs which used defaults. Use a simple approach: for each optional var, log whether it came from env or default.
  - Approach: After building the Config struct, add a single `debug!` or series of `debug!` calls showing the resolved config (use the existing Display impl for the full dump, but also log individual notable settings).
  - Example: `debug!(opencode_path = ?config.opencode_path, max_instances = config.opencode_max_instances, port_range = %format!("{}-{}", config.opencode_port_start, config.opencode_port_start + config.opencode_port_pool_size), "Config loaded");`
  - **DO NOT** log `telegram_bot_token` or `api_key` values — use the existing masked Display.
  
  **main.rs** — Add timing and step-level debug logging:
  - After config load (line 40): `debug!("Config loaded from environment");` 
  - After LogStore init (line 45): `debug!(log_db = %config.log_db_path.display(), "Log store initialized");`
  - After tracing init (line 70): `debug!("Tracing subscriber initialized with env filter");`
  - After orchestrator_store init (line 76): `debug!(db_path = %config.orchestrator_db_path.display(), "Orchestrator store initialized");`
  - After topic_store init (line 77): `debug!(db_path = %config.topic_db_path.display(), "Topic store initialized");`
  - After port_pool creation (line 85): `debug!(start = config.opencode_port_start, size = config.opencode_port_pool_size, "Port pool created");`
  - After InstanceManager init (line 87): `debug!("Instance manager created");`
  - After bot_state creation (line 97): `debug!("Bot state initialized");`
  - After API listener bind (line 106): `debug!(addr = %api_addr, "API listener bound");`
  - Before dispatcher dispatch (line 293): `debug!("Starting Telegram dispatcher");`
  - In shutdown sequence — each step already has `info!`, add `debug!` for elapsed timing.

  **Must NOT do**:
  - Don't change any existing `info!` or `error!` calls in main.rs
  - Don't log telegram_bot_token or api_key

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple additive edits to 2 small files, no complex logic
  - **Skills**: []
    - No special skills needed — just text editing with tracing macros
  - **Skills Evaluated but Omitted**:
    - None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 2-10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/config.rs:39-157` — `from_env()` function where config loading happens. Each field parsed from env var with fallback default. Add debug logs after the final Config struct is built.
  - `src/config.rs:160-184` — `Display` impl with masked secrets. Reference this pattern for safe logging.
  - `src/main.rs:38-319` — Full main function. Existing `info!` calls at lines 72-73, 75, 89, 92, 107, 290, 294, 297, 301, 306, 309, 312, 317. Add `debug!` between these for fine-grained step tracking.
  - `src/main.rs:57-58` — Env filter setup. Log which filter was applied.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] No secrets logged (token, api_key)
  - [ ] With `RUST_LOG=oc_outpost=debug`, startup shows step-by-step initialization detail

  **Commit**: YES
  - Message: `feat(logging): add debug logging to startup and config loading`
  - Files: `src/config.rs`, `src/main.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 2. Integration Layer Logging + SessionNotFound Investigation (integration.rs)

  **What to do**:
  
  This is the most critical file — it's the message routing backbone. Add debug logging to EVERY decision point:
  
  **handle_message() (line 76-159)**:
  - Line 78 (after extracting text): `debug!(topic_id = ?msg.thread_id.map(|t| t.0.0), sender_id = ?msg.from.as_ref().map(|u| u.id.0), text_len = text.len(), text_preview = %text.chars().take(100).collect::<String>(), "Received text message");`
  - Line 96 (after extracting topic_id): `debug!(topic_id = topic_id, "Extracted topic ID from message");`
  - Line 99-105 (after DB lookup): `debug!(topic_id = topic_id, mapping_found = mapping.is_some(), "Topic mapping lookup");`
  - Line 108 (when mapping found): `debug!(topic_id = topic_id, session_id = ?m.session_id, instance_id = ?m.instance_id, streaming = m.streaming_enabled, "Found topic mapping");`
  - **SessionNotFound investigation** — Line 116-132: BEFORE the match, add: `debug!(topic_id = topic_id, session_id = ?mapping.session_id, instance_id = ?mapping.instance_id, project_path = %mapping.project_path, "Checking session_id for message routing");`
  - Line 135 (port resolved): `debug!(topic_id = topic_id, port = port, instance_id = ?mapping.instance_id, "Resolved instance port");`
  - Line 139 (mark_from_telegram): `debug!(session_id = session_id, text_len = text.len(), "Marked message as from Telegram for dedup");`
  - Line 142-145 (send_message_async): `debug!(session_id = session_id, topic_id = topic_id, "Sending message to OpenCode");`
  - Line 153 (streaming check): `debug!(topic_id = topic_id, streaming_enabled = mapping.streaming_enabled, "Checked streaming state");`
  
  **get_instance_port() (line 162-173)**:
  - Line 164: `debug!(instance_id = ?mapping.instance_id, "Looking up instance port");`
  - Line 166-168: `debug!(instance_id = instance_id, port = info.port, "Found port in orchestrator store");`
  - Line 172: `debug!(port = self.state.config.opencode_port_start, "Falling back to default port");`
  
  **ensure_stream_subscription() (line 176-213)**:
  - Line 186 (already subscribed): `debug!(topic_id = topic_id, "Stream already subscribed, skipping");`
  - Line 197-201 (subscribing): `debug!(topic_id = topic_id, session_id = %session_id, "Subscribing to SSE stream");`
  - Line 204 (forwarder spawned): `debug!(topic_id = topic_id, "Stream forwarder spawned");`
  
  **spawn_stream_forwarder() (line 216-275)**:
  - Line 228 (task start): `debug!(topic_id = topic_id, session_id = %session_id, "Stream forwarder task started");`
  - Line 232 (event received): `debug!(topic_id = topic_id, event_type = describe_stream_event(&event), "Received stream event");`
  
  **handle_stream_event() (line 278-384)**:
  - Line 287-301 (TextChunk): `debug!(topic_id = topic_id, chunk_len = text.len(), pending_len = state.pending_text.len(), should_send = should_send, "Text chunk batched");`
  - Line 304 (ToolInvocation): `debug!(topic_id = topic_id, tool_name = %name, "Tool invocation received");`
  - Line 316 (ToolResult): `debug!(topic_id = topic_id, result_len = result.len(), truncated = result.len() > 500, "Tool result received");`
  - Line 337 (SessionError): `debug!(topic_id = topic_id, error = %error, "Session error received");`
  - Line 343 (PermissionRequest): `debug!(topic_id = topic_id, permission_id = %id, permission_type = %permission_type, "Permission request received");`
  
  **flush_pending_text() (line 386-411)**:
  - Line 393-404 (flushing): `debug!(topic_id = topic_id, text_len = text_to_send.len(), "Flushing pending text to Telegram");`
  
  **send_telegram_message() (line 414-432)**:
  - Line 421-422: `debug!(topic_id = topic_id, parts = parts.len(), total_len = text.len(), "Sending message to Telegram");`
  
  **update_topic_name() (line 434-467)**:
  - Line 442: `debug!(topic_id = topic_id, project_path = %mapping.project_path, "Attempting topic name update");`
  
  **stop_all_streams() (line 485-495)**:
  - Line 487: `debug!(count = handles.len(), "Stopping all active streams");`
  
  Also add a helper function `describe_stream_event()` that returns a `&'static str` for each StreamEvent variant (similar to `describe_message_kind`).

  **Must NOT do**:
  - Don't change existing warn!/info!/debug! messages (lines 81-87, 110, 119-126, 147-150, 243, 252, 273, 329, 365, 371, 375, 379, 409, 464)
  - Don't log full message text — truncate to 100 chars

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Additive edits to one file, no logic changes, but many insertion points
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1, 3-10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/integration.rs:76-159` — `handle_message()`: Main routing flow. Add debug at each numbered step.
  - `src/integration.rs:116-132` — SessionNotFound check. This is where the error occurs. Add debug BEFORE the match to show full mapping state.
  - `src/integration.rs:162-173` — `get_instance_port()`: Port resolution with fallback.
  - `src/integration.rs:176-213` — `ensure_stream_subscription()`: Stream subscription with dedup check.
  - `src/integration.rs:216-275` — `spawn_stream_forwarder()`: Async task forwarding SSE events.
  - `src/integration.rs:278-384` — `handle_stream_event()`: Event type dispatch.
  - `src/integration.rs:386-411` — `flush_pending_text()`: Rate-limited batched sends.
  - `src/integration.rs:414-432` — `send_telegram_message()`: Message splitting and sending.
  - `src/integration.rs:434-467` — `update_topic_name()`: Topic name update after first response.
  - `src/integration.rs:505-545` — `describe_message_kind()`: Reference pattern for the new `describe_stream_event()` helper.
  - `src/bot/handlers/new.rs:127-137` — TopicMapping creation with `session_id: None`. Context for why SessionNotFound happens.
  - `src/types/forum.rs` — TopicMapping struct definition.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] Existing 16 log calls in integration.rs are unchanged
  - [ ] With `RUST_LOG=oc_outpost=debug`, message routing shows complete trace from receive → lookup → route → response

  **Commit**: YES
  - Message: `feat(logging): add comprehensive debug logging to integration layer`
  - Files: `src/integration.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 3. Instance Manager Logging (orchestrator/manager.rs)

  **What to do**:
  
  Add debug logging to all instance lifecycle decision points:
  
  **get_or_create() (line 118-168)**:
  - Line 118 entry: `debug!(project_path = %path_str, "get_or_create called");`
  - Line 124 (found in memory): `debug!(project_path = %path_str, "Instance found in memory");`
  - Line 127 (Running/Starting): `debug!(project_path = %path_str, state = ?inst.state().await, "Returning existing running instance");`
  - Line 132 (Stopped/Error): `debug!(project_path = %path_str, state = ?inst.state().await, "Instance stopped/error, will restart");`
  - Line 145 (found in DB): `debug!(project_path = %path_str, instance_id = %info.id, db_state = ?info.state, "Found instance in database");`
  - Line 152 (not in DB): `debug!(project_path = %path_str, "No instance found in memory or database");`
  - Line 158 (max check): `debug!(current = instances.len(), max = self.config.opencode_max_instances, "Instance count check");`
  - Line 167: `debug!(project_path = %path_str, "Spawning new instance");`
  
  **stop_instance() (line 192-223)**:
  - Line 193 entry: `debug!(instance_id = id, "Stopping instance");`
  - Line 199-203: `debug!(instance_id = id, port = port, "Instance stopped, releasing port");`
  - Line 212: `debug!(instance_id = id, "Instance removed from memory");`
  
  **stop_all() (line 226-253)**:
  - Line 226: `debug!(count = instance_ids.len(), "Stopping all instances");`
  - Line 245: `debug!("All instances stopped successfully");` (in the Ok path)
  
  **recover_from_db() (line 286-308)**:
  - Line 286: `debug!("Starting instance recovery from database");`
  - Line 288: `debug!(total = all_instances.len(), "Found instances in database");`
  - Line 292: `debug!(instance_id = %info.id, state = ?info.state, "Attempting to restore instance");`
  - Line 307: `debug!("Instance recovery complete");`
  
  **start_health_check_loop() (line 310-611)**:
  - Line 324: `debug!(interval = ?config.opencode_health_check_interval, "Health check loop started");`
  - Line 328: `debug!(instance_count = instance_ids.len(), "Health check tick");`
  - Line 559-564 (healthy, Ok(false)): `debug!(instance_id = %id, "Instance healthy, reset restart tracker");`
  - Line 577-578 (idle check): `debug!(instance_id = %id, idle_duration = ?activity.last_activity.elapsed(), timeout = ?config.opencode_idle_timeout, "Idle timeout check");`
  
  **record_activity() (line 614-618)**:
  - `debug!(instance_id = id, "Activity recorded");`
  
  **spawn_new_instance() (line 620-707)**:
  - Line 630 (port allocated): `debug!(port = port, project_path = %path_str, "Port allocated for new instance");`
  - Line 633 (ID generated): `debug!(instance_id = %id, port = port, project_path = %path_str, "Spawning instance");`
  - Line 648-655 (spawn result): `debug!(instance_id = %id, "Instance process spawned, waiting for ready");`
  - Line 667-668 (ready check): `debug!(instance_id = %id, ready = ready, "Instance readiness check result");`
  - Line 694-696 (saved to DB): `debug!(instance_id = %id, "Instance saved to database");`
  - Line 700 (added to map): `debug!(instance_id = %id, "Instance added to memory map");`
  
  **restart_instance_by_path() (line 710-755)**:
  - Line 710: `debug!(project_path = %path_str, "Restart requested for instance");`
  - Line 722 (found): `debug!(instance_id = %id, old_port = old_port, "Found instance to restart");`
  - Line 739-741 (backoff): `debug!(instance_id = %id, attempt = tracker.attempt, delay = ?delay, "Restart backoff calculated");`
  
  **restore_instance() (line 757-794)**:
  - Line 758: `debug!(instance_id = %info.id, port = info.port, "Attempting instance restore");`
  - Line 762 (port check): `debug!(port = info.port, available = !self.port_pool.is_available(info.port).await, "Port availability check for restore");`
  - Line 779 (health pass): `debug!(instance_id = %info.id, "Restored instance is healthy");`
  - Line 792 (health fail): `debug!(instance_id = %info.id, "Restored instance not healthy, spawning new");`

  **Must NOT do**:
  - Don't change existing 12 log calls (warn/info/error at lines 295-299, 363, 376-380, 388-391, 414-418, 513-517, 520-523, 536-540, 550-553, 566, 579)
  - Don't modify business logic or restart backoff calculations

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Additive debug! insertions only, one large file
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1-2, 4-10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/orchestrator/manager.rs:118-168` — `get_or_create()`: Main entry point with memory→DB→new fallback chain.
  - `src/orchestrator/manager.rs:192-223` — `stop_instance()`: Stop + port release + DB update + cleanup.
  - `src/orchestrator/manager.rs:286-308` — `recover_from_db()`: Startup recovery flow.
  - `src/orchestrator/manager.rs:310-611` — `start_health_check_loop()`: Background task with crash detection, restart, idle timeout.
  - `src/orchestrator/manager.rs:620-707` — `spawn_new_instance()`: Port alloc → spawn → ready → DB save.
  - `src/orchestrator/manager.rs:710-755` — `restart_instance_by_path()`: Backoff + respawn.
  - `src/orchestrator/manager.rs:757-794` — `restore_instance()`: Port check → external → health → fallback.
  - `src/types/instance.rs` — `InstanceState`, `InstanceInfo`, `InstanceConfig` types for field references.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] With debug logging, can trace full instance lifecycle: create → health check → idle timeout → stop

  **Commit**: YES
  - Message: `feat(logging): add debug logging to instance manager lifecycle`
  - Files: `src/orchestrator/manager.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 4. Orchestrator Support Files (instance.rs + store.rs + port_pool.rs)

  **What to do**:
  
  **orchestrator/instance.rs** — Process lifecycle logging:
  - `spawn()`: Log PID, command, working directory, port
  - `health_check()`: Log URL, response status, latency (use `Instant` for timing)
  - `wait_for_ready()`: Log each poll attempt, final result, total wait time
  - `check_for_crash()`: Log exit status check, whether crash detected
  - `stop()`: Log SIGTERM sent, graceful timeout, SIGKILL fallback if needed
  - `external()`: Log creation of external instance reference
  - All state getters: Don't add logging (too noisy for simple getters)
  
  **orchestrator/store.rs** — Database operation logging:
  - `save_instance()`: `debug!(instance_id = %info.id, state = ?info.state, port = info.port, "Saving instance to database");`
  - `get_instance()`: `debug!(instance_id = id, found = result.is_some(), "DB lookup instance by ID");`
  - `get_instance_by_path()`: `debug!(project_path = path, found = result.is_some(), "DB lookup instance by path");`
  - `get_all_instances()`: `debug!(count = results.len(), "DB retrieved all instances");`
  - `get_active_count()`: `debug!(count = count, "DB active instance count");`
  - `update_state()`: `debug!(instance_id = id, new_state = ?state, "DB updating instance state");`
  - `delete_instance()`: `debug!(instance_id = id, "DB deleting instance");`
  
  **orchestrator/port_pool.rs** — Port management logging:
  - `allocate()`: `debug!(port = allocated_port, remaining = available_count, "Port allocated");`
  - `release()`: `debug!(port = port, "Port released back to pool");`
  - `is_available()`: `debug!(port = port, available = result, "Port availability check via lsof");`
  - `cleanup_orphan()`: `debug!(port = port, "Cleaning up orphan process on port");`
  - `allocated_count()`: No logging (too frequent, called in status)

  **Must NOT do**:
  - Don't add logging to simple getters (id(), port(), project_path(), state())
  - Don't log in hot paths that run per-health-check if too noisy

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Three small files with straightforward debug! additions
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1-3, 5-10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/orchestrator/instance.rs` — Full file. Contains `spawn()`, `external()`, `health_check()`, `wait_for_ready()`, `check_for_crash()`, `stop()`. Currently zero logging.
  - `src/orchestrator/store.rs` — Full file. SQLite CRUD via sqlx. Currently zero logging.
  - `src/orchestrator/port_pool.rs` — Full file. Sequential port allocation with lsof availability check. Currently zero logging.
  - `src/types/instance.rs` — `InstanceState` enum and `InstanceInfo`/`InstanceConfig` structs for field references.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] With debug logging, instance spawn shows: port allocated → process started → PID → health poll → ready
  - [ ] DB operations show query + result summary

  **Commit**: YES
  - Message: `feat(logging): add debug logging to orchestrator instance, store, and port pool`
  - Files: `src/orchestrator/instance.rs`, `src/orchestrator/store.rs`, `src/orchestrator/port_pool.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 5. OpenCode Client & Stream Handler (client.rs + stream_handler.rs)

  **What to do**:
  
  **opencode/client.rs** — HTTP request/response logging (currently ZERO logging):
  - `health()`: `debug!(url = %url, "Health check request");` then `debug!(url = %url, status = %response.status(), "Health check response");`
  - `list_sessions()`: `debug!(url = %url, "Listing sessions");` then `debug!(count = sessions.len(), "Sessions listed");`
  - `get_session()`: `debug!(session_id = id, "Getting session");` then `debug!(session_id = id, found = result.is_some(), "Session lookup result");`
  - `create_session()`: `debug!(project_path = %path, "Creating session");` then `debug!(session_id = %session.id, "Session created");`
  - `send_message()`: `debug!(session_id = id, text_len = text.len(), "Sending message (sync)");` then `debug!(session_id = id, "Message sent (sync)");`
  - `send_message_async()`: `debug!(session_id = id, text_len = text.len(), "Sending message (async)");` then `debug!(session_id = id, status = %response.status(), "Message sent (async)");`
  - `sse_url()`: `debug!(session_id = session_id, url = %url, "Generated SSE URL");`
  - `reply_permission()`: `debug!(session_id = id, permission_id = permission_id, allowed = allowed, "Replying to permission request");` then `debug!("Permission reply sent");`
  
  **opencode/stream_handler.rs** — SSE lifecycle logging (currently 11 calls, enhance):
  - `subscribe()` (line 141-166): `debug!(session_id = %session_id, "Creating SSE subscription");` then after spawn: `debug!(session_id = %session_id, "SSE stream loop spawned");`
  - `mark_from_telegram()` (line 169-194): `debug!(session_id = session_id, text_len = text.len(), "Marking text as from Telegram");`
  - `unsubscribe()` (line 197-208): `debug!(session_id = session_id, found = cancel_tx.is_some(), "Unsubscribe requested");`
  - `run_stream_loop()` (line 224-301): `debug!(session_id = %session_id, "Stream loop started");` At line 249 (connection success): `debug!(session_id = %session_id, "SSE connection established");`
  - `connect_and_process()` (line 303-375): `debug!(session_id = %session_id, url = %url, "Connecting to SSE endpoint");` At line 328 (message received): `debug!(session_id = %session_id, "SSE message received");` At line 360 (batch timeout): `debug!(session_id = %session_id, pending_len = batch_text.len(), "Batch timeout, flushing");`
  - `handle_sse_message()` (line 378-499): Add debug for each event type parsed:
    - Line 388: `debug!("Parsed message.part.updated event");`
    - Line 401: `debug!(text_len = new_text.len(), skipped = should_skip, "Text chunk processed");`
    - Line 404: `debug!(tool_name = %name, "Tool invocation parsed");`
    - Line 417: `debug!(result_len = result.len(), "Tool result parsed");`
    - Line 433: `debug!(message_id = %msg.id, role = %msg.role, "Message complete parsed");`
    - Line 448: `debug!("Session idle parsed");`
    - Line 460: `debug!(error = %error, "Session error parsed");`
    - Line 470: `debug!(permission_id = %id, "Permission request parsed");`
    - Line 482: `debug!(permission_id = %id, allowed = allowed, "Permission reply parsed");`

  **Must NOT do**:
  - Don't change existing 11 log calls in stream_handler.rs
  - Don't log full HTTP response bodies — just status codes and lengths
  - Don't log message text content beyond length

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Two related files with HTTP/SSE client patterns
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1-4, 6-10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/opencode/client.rs` — Full file (~561 lines). All HTTP methods: health, sessions, send_message, permissions. Currently ZERO logging.
  - `src/opencode/stream_handler.rs` — Full file (~1061 lines). SSE subscription, reconnection with backoff, text batching, dedup. Has 11 existing log calls.
  - `src/types/opencode.rs` — OpenCode API types (SessionInfo, MessageInfo, etc.) for field references.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] With debug logging, every HTTP request shows URL + status code
  - [ ] SSE lifecycle shows connect → events → batch → flush → reconnect if needed

  **Commit**: YES
  - Message: `feat(logging): add debug logging to OpenCode client and stream handler`
  - Files: `src/opencode/client.rs`, `src/opencode/stream_handler.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 6. OpenCode Discovery Logging (discovery.rs)

  **What to do**:
  
  **opencode/discovery.rs** — Process discovery logging:
  - Function entry: `debug!("Starting OpenCode process discovery");`
  - After `ps aux` execution: `debug!(process_count = matching_lines, "Found potential OpenCode processes");`
  - For each found process: `debug!(pid = pid, port = ?extracted_port, working_dir = ?working_dir, "Discovered process");`
  - After lsof port extraction: `debug!(pid = pid, port = port, "Port extracted via lsof");`
  - After session info fetch: `debug!(port = port, session_count = sessions.len(), "Fetched sessions from instance");`
  - After filtering grep processes: `debug!(filtered_count = count, "Filtered out grep/self processes");`
  - Function exit: `debug!(total_discovered = result.len(), "Discovery complete");`

  **Must NOT do**:
  - Don't log raw ps/lsof output (too verbose)
  - Don't modify the process discovery logic

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single small file with straightforward additions
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1-5, 7-10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/opencode/discovery.rs` — Full file. Process discovery via ps/lsof, port extraction, session info fetching. Currently zero logging.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] With debug logging, discovery shows process count → per-process details → final tally

  **Commit**: YES
  - Message: `feat(logging): add debug logging to OpenCode process discovery`
  - Files: `src/opencode/discovery.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 7. Bot Command Handlers (all 11 handler files)

  **What to do**:
  
  Add consistent debug logging to ALL handler files. Every handler follows the same pattern:
  1. Log command entry with user context
  2. Log each significant step (DB lookups, instance operations, API calls)
  3. Log response being sent
  
  **For EACH handler file**, add at minimum:
  - Function entry: `debug!(chat_id = msg.chat.id.0, topic_id = ?msg.thread_id.map(|t| t.0.0), sender_id = ?msg.from.as_ref().map(|u| u.id.0), "Handling /command_name");`
  - Key decision points: whatever if/match branches exist
  - DB operations: result summaries
  - Response sent: `debug!("Sent /command_name response");`
  
  **Specific per-handler details**:
  
  **new.rs** (highest priority — SessionNotFound investigation):
  - Line 49 entry: `debug!(sender_id = ?msg.from.as_ref().map(|u| u.id.0), "Handling /new command");`
  - Line 57 (validation): `debug!(name = %name, "Validating project name");`
  - Line 60 (General topic check): `debug!(is_general = is_general_topic(&msg), handle_general = state.config.handle_general_topic, "General topic check");`
  - Line 70 (project path): `debug!(project_path = %project_path.display(), "Resolved project path");`
  - Line 73 (exists check): `debug!(exists = project_path.exists(), "Project directory check");`
  - Line 84-92 (dir creation): `debug!(project_path = %project_path.display(), auto_create = state.config.auto_create_project_dirs, "Creating project directory");`
  - Line 95 (forum topic): `debug!(chat_id = msg.chat.id.0, name = %name, "Creating forum topic");`
  - Line 96 (topic created): `debug!(topic_id = forum_topic.thread_id.0.0, "Forum topic created");`
  - Line 109-118 (instance spawn): `debug!(project_path = %project_path.display(), "Spawning instance via manager");` then `debug!(instance_id = %instance_id, port = port, "Instance spawned");`
  - **Line 127-137 (CRITICAL — TopicMapping creation)**: `debug!(topic_id = mapping.topic_id, session_id = ?mapping.session_id, instance_id = ?mapping.instance_id, "Creating TopicMapping (session_id=None, will be set on first message or /connect)");`
  - Line 138-142 (save): `debug!(topic_id = mapping.topic_id, "TopicMapping saved to database");`
  - Line 157-162 (confirmation): `debug!(topic_id = forum_topic.thread_id.0.0, "Confirmation message sent to topic");`
  
  **connect.rs**:
  - Entry + session search + found/not-found + topic creation + mapping save + confirmation
  
  **disconnect.rs** (already has 1 warn):
  - Entry + mapping lookup + instance stop + topic deletion + confirmation
  
  **link.rs**:
  - Entry + path validation + mapping update + confirmation
  
  **stream.rs**:
  - Entry + current state + toggle result + confirmation
  
  **session.rs**:
  - Entry + mapping lookup + instance lookup + info display
  
  **sessions.rs**:
  - Entry + managed list + discovered list + total count + response
  
  **status.rs**:
  - Entry + manager status fetch + formatting + response
  
  **clear.rs**:
  - Entry + stale mapping count + instances stopped count + mappings deleted + response
  
  **help.rs**:
  - Entry + context detection (general vs topic) + response type
  
  **permissions.rs**:
  - Callback entry + data parsing + permission type + session lookup + reply sent

  **Must NOT do**:
  - Don't change existing warn! in disconnect.rs
  - Don't log full message bodies from bot responses
  - Don't modify any handler logic

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: 11 small files with repetitive pattern, but many files
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1-6, 8-10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/bot/handlers/new.rs` — /new handler. **CRITICAL for SessionNotFound**: lines 127-137 create TopicMapping with `session_id: None`.
  - `src/bot/handlers/connect.rs` — /connect handler. Creates TopicMapping with `session_id: Some(...)` (the happy path).
  - `src/bot/handlers/disconnect.rs` — /disconnect handler. Has 1 existing `warn!` call.
  - `src/bot/handlers/link.rs` — /link handler. Path validation + mapping update.
  - `src/bot/handlers/stream.rs` — /stream handler. Toggle streaming flag.
  - `src/bot/handlers/session.rs` — /session handler. Info display.
  - `src/bot/handlers/sessions.rs` — /sessions handler. Lists managed + discovered.
  - `src/bot/handlers/status.rs` — /status handler. Manager status display.
  - `src/bot/handlers/clear.rs` — /clear handler. Stale mapping cleanup.
  - `src/bot/handlers/help.rs` — /help handler. Context-aware help.
  - `src/bot/handlers/permissions.rs` — Permission callback handler. Inline keyboard parsing.
  - `src/bot/handlers.rs` — Handler re-exports (no logging needed here).
  - `src/types/forum.rs` — TopicMapping struct for field references.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] `cargo test` passes (handler tests still work)
  - [ ] Every bot command shows entry → processing → response in debug logs
  - [ ] `/new` handler specifically logs that TopicMapping is created with `session_id: None`

  **Commit**: YES
  - Message: `feat(logging): add debug logging to all bot command handlers`
  - Files: `src/bot/handlers/new.rs`, `src/bot/handlers/connect.rs`, `src/bot/handlers/disconnect.rs`, `src/bot/handlers/link.rs`, `src/bot/handlers/stream.rs`, `src/bot/handlers/session.rs`, `src/bot/handlers/sessions.rs`, `src/bot/handlers/status.rs`, `src/bot/handlers/clear.rs`, `src/bot/handlers/help.rs`, `src/bot/handlers/permissions.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 8. Data Store Logging (forum/store.rs + db/log_store.rs + db/tracing_layer.rs)

  **What to do**:
  
  **forum/store.rs** — Topic mapping database operations:
  - `save_mapping()`: `debug!(topic_id = mapping.topic_id, chat_id = mapping.chat_id, session_id = ?mapping.session_id, instance_id = ?mapping.instance_id, "Saving topic mapping");`
  - `get_mapping()`: `debug!(topic_id = topic_id, found = result.is_some(), "Topic mapping lookup");`
  - `get_mapping_by_session()`: `debug!(session_id = session_id, found = result.is_some(), "Topic mapping lookup by session");`
  - `update_session()`: `debug!(topic_id = topic_id, session_id = session_id, "Updating session_id in mapping");`
  - `toggle_streaming()`: `debug!(topic_id = topic_id, "Toggling streaming state");` then `debug!(topic_id = topic_id, new_state = new_state, "Streaming state toggled");`
  - `mark_topic_name_updated()`: `debug!(topic_id = topic_id, "Marking topic name as updated");`
  - `delete_mapping()`: `debug!(topic_id = topic_id, "Deleting topic mapping");`
  - `get_stale_mappings()`: `debug!(older_than = older_than, count = results.len(), "Found stale mappings");`
  - `get_all_mappings()`: `debug!(count = results.len(), "Retrieved all mappings");`
  
  **db/log_store.rs** — Log persistence (be careful — this IS the logging system):
  - `new()`: `debug!(db_path = %path.display(), "Log store initialized");` — NOTE: This runs BEFORE tracing is initialized in main.rs, so this specific log won't appear. Add it anyway for completeness, but add a comment explaining why.
  - `create_run()`: `debug!(run_id = run_id, version = version, "Created run record");`
  - `finish_run()`: `debug!(run_id = run_id, "Finished run record");`
  - `insert_log()`: **DO NOT add logging here** — this would cause infinite recursion (log insertion triggers a log which triggers insertion...)
  
  **db/tracing_layer.rs** — Custom tracing layer:
  - `on_event()`: **DO NOT add logging** — same infinite recursion risk
  - `new()`: Can add a comment explaining the layer, but no `debug!` call (same bootstrap timing issue)
  - Instead, add a comment at top of file: `// NOTE: This module must NOT use tracing macros — doing so causes infinite recursion since this IS the tracing layer.`

  **Must NOT do**:
  - **CRITICAL**: DO NOT add `debug!` inside `insert_log()` or `on_event()` — causes infinite recursion
  - Don't add logging to tracing_layer.rs event processing

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Three small files, but must be careful with recursion risk in log_store/tracing_layer
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1-7, 9-10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/forum/store.rs` — Full file. TopicMapping CRUD operations via sqlx. Currently zero logging.
  - `src/db/log_store.rs` — Full file. `create_run()`, `finish_run()`, `insert_log()`. **insert_log MUST NOT be logged.**
  - `src/db/tracing_layer.rs` — Full file. Custom `Layer<S>` impl. **Must NOT use tracing macros internally.**

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] No infinite recursion when running with debug logging
  - [ ] Topic mapping operations show in debug logs with full field context

  **Commit**: YES
  - Message: `feat(logging): add debug logging to forum store and log store`
  - Files: `src/forum/store.rs`, `src/db/log_store.rs`, `src/db/tracing_layer.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 9. API Server Logging (api/mod.rs)

  **What to do**:
  
  **api/mod.rs** — Axum HTTP API request/response logging:
  - Each endpoint handler entry: `debug!(method = "POST", path = "/api/register", "API request received");`
  - Auth check result: `debug!(authenticated = is_authed, "API auth check");`
  - Register endpoint: `debug!(project_path = %path, port = port, "Registering external instance");`
  - Unregister endpoint: `debug!(project_path = %path, "Unregistering external instance");`
  - Status endpoint: `debug!(project_path = %path, found = result.is_some(), "Instance status check");`
  - List instances endpoint: `debug!(count = instances.len(), "Listing external instances");`
  - Response: `debug!(status = %status_code, "API response sent");`
  - Auth middleware: `debug!(has_api_key = api_key.is_some(), has_bearer = bearer.is_some(), "API auth middleware");`

  **Must NOT do**:
  - Don't log API key values
  - Don't modify router setup

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single file with clear HTTP handler pattern
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1-8, 10)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/api/mod.rs` — Full file. Axum router with 4 endpoints + auth middleware. Currently zero logging.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] API requests show method + path + auth result + response status in debug logs

  **Commit**: YES
  - Message: `feat(logging): add debug logging to API server`
  - Files: `src/api/mod.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 10. Telegram Markdown Logging (telegram/markdown.rs)

  **What to do**:
  
  **telegram/markdown.rs** — Conversion and splitting logging:
  - `markdown_to_telegram_html()`: `debug!(input_len = input.len(), "Converting markdown to Telegram HTML");` then `debug!(output_len = result.len(), "Markdown conversion complete");`
  - `split_message()`: `debug!(input_len = text.len(), max_len = max_length, parts = parts.len(), "Splitting message for Telegram");`
  - `truncate_message()`: `debug!(input_len = text.len(), max_len = max_length, truncated = text.len() > max_length, "Truncating message");`
  - Internal conversion functions (if any): Only log if they contain significant decision points (e.g., code block detection, link parsing). Use discretion — don't over-log recursive helpers.

  **Must NOT do**:
  - Don't log full input/output text (could be large)
  - Don't add logging inside tight recursive loops

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single utility file with simple additions
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Tasks 1-9)
  - **Blocks**: Task 11
  - **Blocked By**: None

  **References**:
  - `src/telegram/markdown.rs` — Full file. Markdown→HTML conversion, `split_message()`, `truncate_message()`. Currently zero logging.

  **Acceptance Criteria**:
  - [ ] `cargo build` succeeds
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] Markdown conversion shows input/output lengths in debug logs

  **Commit**: YES
  - Message: `feat(logging): add debug logging to Telegram markdown conversion`
  - Files: `src/telegram/markdown.rs`
  - Pre-commit: `cargo build && cargo clippy -- -D warnings`

---

- [ ] 11. Final Build Verification & Integration Check

  **What to do**:
  
  After ALL tasks 1-10 complete, run full project verification:
  
  1. `cargo build` — Ensure all new logging compiles correctly
  2. `cargo clippy -- -D warnings` — No warnings from unused imports, etc.
  3. `cargo test` — All existing tests still pass
  4. Review all files touched to ensure no existing log calls were modified
  5. Verify tracing import consistency across all files

  **Must NOT do**:
  - Don't modify any source files — just verify
  - Don't change test expectations

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Verification only, no code changes
  - **Skills**: []
  - **Skills Evaluated but Omitted**: None relevant

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sequential, after all Wave 1 tasks)
  - **Blocks**: None (final)
  - **Blocked By**: Tasks 1-10

  **References**:
  - All source files modified in Tasks 1-10
  - `Cargo.toml` — tracing dependency versions
  - `justfile` or project root — for build/test commands (check, fmt, clippy, test)

  **Acceptance Criteria**:

  ```bash
  # All must succeed:
  cargo build 2>&1
  # Assert: exit code 0, no errors

  cargo clippy -- -D warnings 2>&1
  # Assert: exit code 0, no warnings

  cargo test 2>&1
  # Assert: exit code 0, all tests pass
  ```

  **Commit**: NO (verification only, no changes)

---

## Commit Strategy

| After Task | Message | Key Files | Verification |
|------------|---------|-----------|--------------|
| 1 | `feat(logging): add debug logging to startup and config loading` | config.rs, main.rs | cargo build + clippy |
| 2 | `feat(logging): add comprehensive debug logging to integration layer` | integration.rs | cargo build + clippy |
| 3 | `feat(logging): add debug logging to instance manager lifecycle` | manager.rs | cargo build + clippy |
| 4 | `feat(logging): add debug logging to orchestrator instance, store, and port pool` | instance.rs, store.rs, port_pool.rs | cargo build + clippy |
| 5 | `feat(logging): add debug logging to OpenCode client and stream handler` | client.rs, stream_handler.rs | cargo build + clippy |
| 6 | `feat(logging): add debug logging to OpenCode process discovery` | discovery.rs | cargo build + clippy |
| 7 | `feat(logging): add debug logging to all bot command handlers` | 11 handler files | cargo build + clippy + test |
| 8 | `feat(logging): add debug logging to forum store and log store` | forum/store.rs, log_store.rs, tracing_layer.rs | cargo build + clippy |
| 9 | `feat(logging): add debug logging to API server` | api/mod.rs | cargo build + clippy |
| 10 | `feat(logging): add debug logging to Telegram markdown conversion` | markdown.rs | cargo build + clippy |
| 11 | (no commit — verification only) | — | cargo build + clippy + test |

---

## Success Criteria

### Verification Commands
```bash
cargo build           # Expected: Compiling oc_outpost... Finished
cargo clippy -- -D warnings  # Expected: no warnings
cargo test            # Expected: all tests pass
```

### Final Checklist
- [ ] All 35 previously-unlogged files now have debug logging
- [ ] All 5 files with partial logging have enhanced coverage
- [ ] SessionNotFound error path has diagnostic debug logs showing mapping state
- [ ] No existing log calls were modified
- [ ] No sensitive data is logged (token, api_key)
- [ ] No infinite recursion in log_store/tracing_layer
- [ ] All structured fields use consistent names per conventions table
- [ ] `cargo build && cargo clippy -- -D warnings && cargo test` all pass
