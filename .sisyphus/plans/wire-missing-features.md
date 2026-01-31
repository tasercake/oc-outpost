# Wire All Promised-but-Missing Features in oc-outpost

## TL;DR

> **Quick Summary**: Connect the fully-coded-but-unused runtime machinery (InstanceManager, PortPool, OpenCodeInstance) into the live bot, fix config test failures, register the permission callback handler, remove dead code, and make `cargo test` + `cargo clippy` clean.
> 
> **Deliverables**:
> - `/new` actually spawns OpenCode processes with allocated ports
> - `/disconnect` actually stops processes (SIGTERM→SIGKILL)
> - `/status` shows real port usage and uptime
> - Permission Allow/Deny buttons work (callback registered)
> - Health check loop runs in background
> - Instance recovery on startup
> - All 355+ tests pass (0 failures)
> - `cargo clippy` — 0 warnings
> - No dead code, no unnecessary `#[allow(dead_code)]`
> 
> **Estimated Effort**: Medium (12 tasks, ~4-6 hours)
> **Parallel Execution**: YES - 4 waves
> **Critical Path**: Task 1 (BotState) → Task 2 (main.rs) → Tasks 3-7 (handlers/integration) → Tasks 8-12 (cleanup)

---

## Context

### Original Request
Wire all promised-but-missing features revealed by a completion audit. The core runtime machinery (InstanceManager, PortPool, OpenCodeInstance) is fully coded and tested in isolation but never connected to `main.rs` or any live code path. The bot operates as a metadata bookkeeper rather than an actual orchestrator.

### Interview Summary
**Key Findings from Code Review**:
- `InstanceManager` (908 lines), `PortPool` (328 lines), `OpenCodeInstance` (678 lines) — all fully implemented, never used
- `BotState` only holds `OrchestratorStore`, `TopicStore`, `Config` — missing `InstanceManager`
- `main.rs` creates stores directly, no PortPool, no InstanceManager, no health check loop
- `/new` creates metadata only (hardcoded port, no process spawned)
- `/disconnect` updates DB state only (never stops process)
- `/status` has hardcoded port_used=port_total and uptime=epoch seconds
- Permission callback handler is implemented but never registered in dispatcher
- Config tests: 9 fail due to HANDLE_GENERAL_TOPIC env var leaking between parallel tests
- 12 clippy warnings for dead code, 68 `#[allow(dead_code)]` annotations across 21 files
- README says `src/storage/` but actual path is `src/db/`

**Ownership Resolution**:
- `OrchestratorStore` derives `Clone` (uses `SqlitePool` which is `Arc` internally) — cheap to clone
- Strategy: Clone `OrchestratorStore` before passing to `InstanceManager`; pass clone to `api::AppState`
- `BotState` gets `InstanceManager` (which holds `Arc<Mutex<OrchestratorStore>>` internally)
- For handlers needing raw store access: access through `InstanceManager`'s public `store` field, or store a separate cloned reference in BotState

### Metis Review
**Identified Gaps** (addressed):
- Ownership model validated: OrchestratorStore clone is cheap (SqlitePool is Arc-based)
- Shutdown sequence defined: stop_all_streams → manager.stop_all() → api_handle.abort()
- Health check restart: use existing `restart_instance_by_path()` at line 371
- Config test isolation: add `serial_test` crate to dev-dependencies
- Session ID flow clarified: OpenCode generates session_id; bot queries it after spawn via health check
- Bot start time tracking: store `Instant::now()` at startup for real uptime calculation

---

## Work Objectives

### Core Objective
Wire the existing runtime machinery into the live bot so that commands actually spawn/stop processes, health monitoring runs, and all promised features work end-to-end.

### Concrete Deliverables
- Modified `src/bot/state.rs` with `InstanceManager` field + `bot_start_time`
- Modified `src/main.rs` with full lifecycle (PortPool, InstanceManager, health check, recovery, callback handler, shutdown)
- Modified `/new` handler that spawns real OpenCode processes
- Modified `/disconnect` handler that stops real processes
- Modified `/status` handler with real metrics
- Registered permission callback handler in Telegram dispatcher
- Fixed session_id in permission requests
- Fixed health check loop restart logic
- Fixed 9 config tests (serial_test)
- Removed dead types in `types/opencode.rs`
- Removed unnecessary `#[allow(dead_code)]`
- Fixed README project structure

### Definition of Done
- [ ] `cargo test` — ALL tests pass (0 failures, currently 9 failing)
- [ ] `cargo clippy` — 0 warnings (currently 12)
- [ ] `cargo build --release` — succeeds
- [ ] `/new` spawns OpenCode process (allocates port, gets PID)
- [ ] `/disconnect` stops OpenCode process (SIGTERM/SIGKILL)
- [ ] Permission Allow/Deny buttons work (callback registered in dispatcher)
- [ ] `/status` shows real port usage and uptime
- [ ] Health check loop runs in background
- [ ] Instance recovery runs on startup
- [ ] No dead code, no unnecessary `#[allow(dead_code)]`

### Must Have
- InstanceManager wired into BotState and main.rs
- PortPool created and used for port allocation
- Health check loop started on boot
- Instance recovery on startup
- Graceful shutdown with `manager.stop_all()`
- Callback query handler registered in dispatcher
- Real metrics in `/status`

### Must NOT Have (Guardrails)
- **No new features** — only wire existing code
- **No API changes** — api::AppState can keep using cloned OrchestratorStore directly
- **No new dependencies** except `serial_test` for dev-dependencies
- **No signature changes** to InstanceManager, PortPool, or OpenCodeInstance unless absolutely necessary for wiring
- **No refactoring** of working, tested code — only change what's needed for wiring
- **No breaking existing passing tests** — all 346 currently passing tests must continue to pass
- **No adding new error types** — use existing OutpostError variants
- **Do NOT modify the `api` module's AppState** — it works with its own OrchestratorStore clone, leave it

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: YES (`cargo test`, 346 pass, 9 fail)
- **User wants tests**: Fix existing failures + ensure no regressions
- **Framework**: `cargo test` (built-in) + `serial_test` crate for config tests

### Verification Approach
Each task includes automated verification:
1. `cargo test` — all tests pass
2. `cargo clippy` — 0 warnings
3. `cargo build` — compiles without errors

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation — Start Immediately):
└── Task 1: Restructure BotState to hold InstanceManager

Wave 2 (Main Wiring — After Wave 1):
└── Task 2: Rewire main.rs lifecycle (PortPool, InstanceManager, health check, recovery, shutdown, callback handler)

Wave 3 (Handler Wiring — After Wave 2):
├── Task 3: Wire /new handler to spawn processes
├── Task 4: Wire /disconnect handler to stop processes
├── Task 5: Wire /status handler with real metrics
├── Task 6: Fix permission callback session_id
└── Task 7: Fix health check loop restart logic

Wave 4 (Cleanup — After Wave 3):
├── Task 8: Fix config test failures (serial_test)
├── Task 9: Remove dead types in types/opencode.rs
├── Task 10: Audit and remove #[allow(dead_code)] annotations
├── Task 11: Fix clippy warnings
└── Task 12: Fix README project structure
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 2 | None (must be first) |
| 2 | 1 | 3, 4, 5, 6, 7 | None (must be second) |
| 3 | 2 | 10 | 4, 5, 6, 7 |
| 4 | 2 | 10 | 3, 5, 6, 7 |
| 5 | 2 | 10 | 3, 4, 6, 7 |
| 6 | 2 | 10 | 3, 4, 5, 7 |
| 7 | 2 | 10 | 3, 4, 5, 6 |
| 8 | None | 10, 11 | 1, 2, 3-7, 9, 12 |
| 9 | None | 10, 11 | 1, 2, 3-7, 8, 12 |
| 10 | 3, 4, 5, 6, 7, 8, 9 | 11 | 12 |
| 11 | 10 | None | 12 |
| 12 | None | None | 8, 9, 10, 11 |

### Agent Dispatch Summary

| Wave | Tasks | Recommended Agents |
|------|-------|-------------------|
| 1 | 1 | `delegate_task(category="quick")` |
| 2 | 2 | `delegate_task(category="unspecified-high")` |
| 3 | 3, 4, 5, 6, 7 | `delegate_task(category="quick")` in parallel |
| 4 | 8, 9, 10, 11, 12 | `delegate_task(category="quick")` in parallel |

---

## TODOs

### Wave 1: Foundation

- [x] 1. Restructure BotState to hold InstanceManager

  **What to do**:
  - Modify `BotState` struct to add `instance_manager: Arc<InstanceManager>` field
  - Add `bot_start_time: std::time::Instant` field (for real uptime in /status)
  - Keep `orchestrator_store` and `topic_store` fields (handlers still need direct store access for queries that don't go through InstanceManager)
  - Update `BotState::new()` to accept `InstanceManager` and `Instant`
  - The `orchestrator_store` in BotState will be a separate clone from the one inside InstanceManager — this is fine because SqlitePool is Arc-based (same underlying connection pool)
  - Update existing BotState tests to construct with the new signature

  **Must NOT do**:
  - Don't remove `orchestrator_store` from BotState — many handlers use it directly for queries. InstanceManager is for lifecycle operations only.
  - Don't change any handler code yet (that's later tasks)
  - Don't change InstanceManager's constructor signature

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small struct modification with well-defined changes
  - **Skills**: []
  - **Skills Evaluated but Omitted**:
    - `git-master`: Not needed — no git operations

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1 (sole task)
  - **Blocks**: Task 2
  - **Blocked By**: None

  **References**:

  **Pattern References**:
  - `src/bot/state.rs:1-27` — Current BotState struct definition. Currently holds `orchestrator_store: Arc<Mutex<OrchestratorStore>>`, `topic_store: Arc<Mutex<TopicStore>>`, `config: Arc<Config>`. Add `instance_manager: Arc<InstanceManager>` and `bot_start_time: std::time::Instant`.

  **API/Type References**:
  - `src/orchestrator/manager.rs:71-79` — InstanceManager struct definition. Fields: `config`, `store: Arc<Mutex<OrchestratorStore>>`, `port_pool`, `instances`, `restart_trackers`, `activity_trackers`, `shutdown_signal`. Note: store is already `Arc<Mutex<>>` inside InstanceManager.
  - `src/orchestrator/manager.rs:89-103` — `InstanceManager::new()` signature: takes `config: Arc<Config>`, `store: OrchestratorStore`, `port_pool: PortPool`. Returns `Result<Self>`. Takes ownership of store and wraps in `Arc<Mutex<>>`.

  **Test References**:
  - `src/bot/state.rs:29-104` — Existing tests for BotState. `create_test_config()` helper creates Config + TempDir. Tests construct BotState with `BotState::new(orchestrator_store, topic_store, config)`. These must be updated to also pass InstanceManager + Instant.
  - `src/orchestrator/manager.rs:622-660` — `create_test_manager()` helper shows how to construct InstanceManager for tests: `InstanceManager::new(Arc::new(config), store, port_pool).await.unwrap()`.

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test --lib bot::state` — all state tests pass with updated constructor
  - [ ] BotState has `instance_manager: Arc<InstanceManager>` field
  - [ ] BotState has `bot_start_time: std::time::Instant` field
  - [ ] `orchestrator_store` and `topic_store` fields are preserved

  **Commit**: YES
  - Message: `refactor(bot): add InstanceManager and bot_start_time to BotState`
  - Files: `src/bot/state.rs`
  - Pre-commit: `cargo test --lib bot::state`

---

### Wave 2: Main Wiring

- [x] 2. Rewire main.rs lifecycle

  **What to do**:
  - Create `PortPool` from config: `PortPool::new(config.opencode_port_start, config.opencode_port_pool_size)`
  - Clone `OrchestratorStore` before passing to `InstanceManager`: `let store_for_manager = orchestrator_store.clone();`
  - Create `InstanceManager`: `InstanceManager::new(Arc::new(config.clone()), store_for_manager, port_pool).await?`
  - Call `manager.recover_from_db().await?` on startup
  - Call `manager.start_health_check_loop()` on startup (save JoinHandle)
  - Store `Instant::now()` as `bot_start_time`
  - Pass `InstanceManager` + `bot_start_time` to `BotState::new()`
  - Pass original (not cloned) `orchestrator_store` to `api::AppState` (this already works)
  - Add `Update::filter_callback_query()` branch to dispatcher that routes to `handle_permission_callback`
  - Import `handle_permission_callback` from bot handlers
  - Add `manager.stop_all().await` to shutdown block (before API abort)
  - Keep existing `integration.stop_all_streams()` in shutdown

  **Dispatcher change** — Add callback query branch:
  ```rust
  // After the existing .branch(Update::filter_message()...) sections, add:
  .branch(
      Update::filter_callback_query().endpoint({
          let state = Arc::clone(&bot_state);
          move |bot: Bot, q: CallbackQuery| {
              let state = Arc::clone(&state);
              async move {
                  if let Err(e) = handle_permission_callback(bot, q, state).await {
                      error!("Error handling callback: {:?}", e);
                  }
                  respond(())
              }
          }
      })
  )
  ```

  **Shutdown sequence** (in `tokio::select!` after block):
  ```rust
  info!("Stopping active streams...");
  integration.stop_all_streams().await;
  
  info!("Stopping all OpenCode instances...");
  if let Err(e) = instance_manager.stop_all().await {
      error!("Error stopping instances: {:?}", e);
  }
  
  info!("Stopping API server...");
  api_handle.abort();
  
  info!("Shutdown complete.");
  ```

  **Must NOT do**:
  - Don't change api::AppState — it already works with cloned OrchestratorStore
  - Don't modify Integration constructor — it takes Arc<BotState> which will now contain InstanceManager
  - Don't change the OpenCodeClient creation (it's used by Integration for stream handling, independent of InstanceManager)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Main.rs wiring is the most complex change — multiple components, careful ordering required
  - **Skills**: []
  - **Skills Evaluated but Omitted**:
    - `git-master`: Not needed for implementation

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (sole task)
  - **Blocks**: Tasks 3, 4, 5, 6, 7
  - **Blocked By**: Task 1

  **References**:

  **Pattern References**:
  - `src/main.rs:31-240` — Current main.rs. Lines 46-47: store creation. Lines 49-52: api state. Lines 54-58: BotState creation. Lines 78-214: dispatcher handler tree (no callback_query branch). Lines 223-238: shutdown block. This is the PRIMARY file to modify.
  - `src/main.rs:203-214` — Current plain message handler branch. The callback_query branch goes AFTER this (as a new `.branch()` at the same level as `Update::filter_message()`).

  **API/Type References**:
  - `src/orchestrator/port_pool.rs:31-37` — `PortPool::new(start: u16, size: u16) -> Self`
  - `src/orchestrator/manager.rs:89-103` — `InstanceManager::new(config: Arc<Config>, store: OrchestratorStore, port_pool: PortPool) -> Result<Self>`
  - `src/orchestrator/manager.rs:276-301` — `recover_from_db(&self) -> Result<()>` — restores Running/Starting instances
  - `src/orchestrator/manager.rs:308-436` — `start_health_check_loop(&self) -> JoinHandle<()>` — spawns background health check task
  - `src/orchestrator/manager.rs:219-246` — `stop_all(&self) -> Result<()>` — graceful shutdown of all instances
  - `src/bot/handlers/permissions.rs:67-102` — `handle_permission_callback(bot: Bot, q: CallbackQuery, state: Arc<BotState>) -> Result<()>` — the handler to register
  - `src/api/mod.rs:17-22` — `api::AppState` takes `store: OrchestratorStore` (not Arc<Mutex<>>). It's separate from BotState.

  **Import References**:
  - `src/main.rs:13-16` — Current bot handler imports. Must add `handle_permission_callback` import.
  - `src/bot/mod.rs` — Check what's exported from the bot module to ensure `handle_permission_callback` is public.

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test` — all previously passing tests still pass
  - [ ] main.rs creates PortPool, InstanceManager
  - [ ] main.rs calls `recover_from_db()` on startup
  - [ ] main.rs calls `start_health_check_loop()` on startup
  - [ ] Dispatcher has `Update::filter_callback_query()` branch
  - [ ] Shutdown block calls `instance_manager.stop_all()`
  - [ ] `handle_permission_callback` is imported and wired

  **Commit**: YES
  - Message: `feat: wire InstanceManager, PortPool, health checks, and callback handler into main.rs`
  - Files: `src/main.rs`
  - Pre-commit: `cargo build && cargo test`

---

### Wave 3: Handler Wiring

- [x] 3. Wire /new handler to spawn processes via InstanceManager

  **What to do**:
  - After creating the forum topic and project directory, call `state.instance_manager.get_or_create(&project_path).await?` to spawn the actual OpenCode process
  - Remove the manual `InstanceInfo` construction (lines 120-129) — `InstanceManager::get_or_create()` handles this internally via `spawn_new_instance()` which creates InstanceInfo, saves to DB, and tracks in memory
  - After spawn, get the instance's actual port and PID from the returned `Arc<Mutex<OpenCodeInstance>>`
  - Update the `TopicMapping` to include the real instance_id from the spawned instance
  - Update the confirmation message to show actual port
  - Handle error case: InstanceManager returns error if max instances reached or port pool exhausted

  **Key change** — Replace lines 110-134 with:
  ```rust
  // Spawn OpenCode instance via InstanceManager
  let instance_lock = state.instance_manager
      .get_or_create(&project_path)
      .await
      .map_err(|e| OutpostError::io_error(format!("Failed to spawn instance: {}", e)))?;
  
  let inst = instance_lock.lock().await;
  let instance_id = inst.id().to_string();
  let port = inst.port();
  drop(inst);
  ```

  **Must NOT do**:
  - Don't modify InstanceManager's `get_or_create()` method
  - Don't change validation logic (project name validation, General topic check, directory creation)
  - Don't break existing /new handler tests (validation tests are pure unit tests, unaffected)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Focused change in one handler file
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 4, 5, 6, 7)
  - **Blocks**: Task 10
  - **Blocked By**: Task 2

  **References**:

  **Pattern References**:
  - `src/bot/handlers/new.rs:51-174` — Current /new handler. Lines 110-134 construct InstanceInfo manually with hardcoded `state.config.opencode_port_start` as port and `InstanceState::Starting` — this is the code to replace with InstanceManager call.

  **API/Type References**:
  - `src/orchestrator/manager.rs:113-163` — `get_or_create(&self, project_path: &Path) -> Result<Arc<Mutex<OpenCodeInstance>>>`. Checks memory, checks DB, checks limits, then calls `spawn_new_instance()`. Returns locked instance.
  - `src/orchestrator/manager.rs:446-532` — `spawn_new_instance()`: allocates port, generates ID, spawns process, waits for ready, saves to DB. This is what actually runs when get_or_create creates new.
  - `src/orchestrator/instance.rs:50-100` — `OpenCodeInstance::spawn()`. Runs `opencode serve --port PORT --project PATH`. After spawn, instance has `id()`, `port()`, `pid()` methods.

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test --lib bot::handlers::new` — existing validation tests pass
  - [ ] New handler calls `state.instance_manager.get_or_create()` instead of manual InstanceInfo creation
  - [ ] TopicMapping uses real instance_id from spawned instance
  - [ ] Confirmation message shows actual port (not hardcoded config port)

  **Commit**: YES
  - Message: `feat(new): wire /new handler to spawn OpenCode processes via InstanceManager`
  - Files: `src/bot/handlers/new.rs`
  - Pre-commit: `cargo test --lib bot::handlers::new`

---

- [x] 4. Wire /disconnect handler to stop processes via InstanceManager

  **What to do**:
  - After confirming the instance is Managed, call `state.instance_manager.stop_instance(instance_id).await` instead of just updating DB state
  - `stop_instance()` handles: process stop (SIGTERM→SIGKILL), port release, DB state update, memory cleanup
  - Remove the manual `store.update_state(instance_id, InstanceState::Stopped)` call — `stop_instance()` does this internally
  - Keep the `InstanceType::Managed` check — only stop managed instances, discovered/external just disconnect

  **Key change** — Replace lines 43-58 with:
  ```rust
  if let Some(instance_id) = &mapping.instance_id {
      let store = state.orchestrator_store.lock().await;
      if let Some(instance_info) = store
          .get_instance(instance_id)
          .await
          .map_err(|e| OutpostError::database_error(e.to_string()))?
      {
          drop(store); // Release lock before calling InstanceManager
          if instance_info.instance_type == InstanceType::Managed {
              if let Err(e) = state.instance_manager.stop_instance(instance_id).await {
                  warn!("Failed to stop instance {}: {:?}", instance_id, e);
                  // Continue with disconnect even if stop fails
              }
          }
      } else {
          drop(store);
      }
  }
  ```

  **Must NOT do**:
  - Don't change the topic deletion or mapping deletion logic
  - Don't stop Discovered or External instances (only disconnect from them)
  - Don't modify the `get_topic_id()` helper
  - Don't break existing disconnect tests

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small focused change in one handler
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 3, 5, 6, 7)
  - **Blocks**: Task 10
  - **Blocked By**: Task 2

  **References**:

  **Pattern References**:
  - `src/bot/handlers/disconnect.rs:26-82` — Current /disconnect handler. Lines 43-58: gets instance info, checks if Managed, then only calls `store.update_state()` — never stops the actual process. This is the section to fix.

  **API/Type References**:
  - `src/orchestrator/manager.rs:186-216` — `stop_instance(&self, id: &str) -> Result<()>`. Calls `inst.stop()`, releases port via `port_pool.release()`, updates DB state to Stopped, removes from memory maps. Full lifecycle cleanup.
  - `src/orchestrator/instance.rs` — `stop()` sends SIGTERM, waits GRACEFUL_SHUTDOWN_TIMEOUT (5s), then SIGKILL if still alive.

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test --lib bot::handlers::disconnect` — existing tests pass
  - [ ] Managed instances are stopped via `InstanceManager::stop_instance()`
  - [ ] Discovered/External instances are NOT stopped (just disconnected)
  - [ ] Port is released back to pool after stop

  **Commit**: YES
  - Message: `feat(disconnect): wire /disconnect to stop processes via InstanceManager`
  - Files: `src/bot/handlers/disconnect.rs`
  - Pre-commit: `cargo test --lib bot::handlers::disconnect`

---

- [x] 5. Wire /status handler with real metrics from InstanceManager

  **What to do**:
  - Replace hardcoded port_used/port_total (line 89-90) with real data from `state.instance_manager.get_status().await`
  - Replace hardcoded uptime (line 97) with real uptime: `state.bot_start_time.elapsed().as_secs()`
  - `ManagerStatus` provides: `total_instances`, `running_instances`, `stopped_instances`, `error_instances`, `available_ports`
  - Calculate `port_used` as `config.opencode_port_pool_size - status.available_ports`
  - Keep the existing instance counting by type from DB (lines 74-86) — this gives managed/discovered/external breakdown
  - Replace hardcoded "Health: Healthy" with dynamic health based on error_instances count

  **Key changes**:
  ```rust
  // Replace lines 88-97 with:
  let manager_status = state.instance_manager.get_status().await;
  let port_total = state.config.opencode_port_pool_size as usize;
  let port_used = port_total - manager_status.available_ports;
  let uptime_seconds = state.bot_start_time.elapsed().as_secs();
  ```

  **Must NOT do**:
  - Don't change the `format_status_output()` function signature (keep it pure for testing)
  - Don't add new metrics beyond what ManagerStatus provides
  - Don't change the output format (keep same text structure)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small data source replacement
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 3, 4, 6, 7)
  - **Blocks**: Task 10
  - **Blocked By**: Task 2

  **References**:

  **Pattern References**:
  - `src/bot/handlers/status.rs:57-114` — Current /status handler. Lines 88-90: hardcoded port metrics. Line 97: `uptime_seconds = now` (epoch seconds, not bot uptime). Line 51: hardcoded "Health: Healthy".

  **API/Type References**:
  - `src/orchestrator/manager.rs:248-274` — `get_status(&self) -> ManagerStatus`. Returns: total_instances, running_instances, stopped_instances, error_instances, available_ports. Uses `self.port_pool.allocated_count()` for port calculation.
  - `src/orchestrator/manager.rs:31-39` — `ManagerStatus` struct definition.
  - `src/orchestrator/port_pool.rs:159` — `allocated_count(&self) -> usize` — used by get_status().

  **Test References**:
  - `src/bot/handlers/status.rs:116-193` — Existing tests for `format_uptime()` and `format_status_output()`. These are pure unit tests that test formatting functions — they will NOT break because we're only changing the data source, not the formatter.

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test --lib bot::handlers::status` — existing formatting tests pass
  - [ ] Port usage shows real allocated vs total (not hardcoded)
  - [ ] Uptime shows time since bot start (not Unix epoch)
  - [ ] Health status is dynamic (based on error_instances count)

  **Commit**: YES
  - Message: `feat(status): wire /status to real metrics from InstanceManager`
  - Files: `src/bot/handlers/status.rs`
  - Pre-commit: `cargo test --lib bot::handlers::status`

---

- [x] 6. Fix permission request session_id in integration.rs

  **What to do**:
  - In `src/integration.rs`, line 325, replace `let session_id = "";` with the actual session_id from the stream subscription context
  - The session_id is available: `ensure_stream_subscription()` receives a `mapping: &TopicMapping` which has `session_id: Option<String>`
  - The stream event handler in `handle_stream_event()` doesn't currently receive session_id, but the `spawn_stream_forwarder()` method has access to `mapping.session_id`
  - Fix: Pass session_id from the mapping into the stream event handler closure, then use it in the PermissionRequest branch
  - Also fix `handle_permission_callback` in permissions.rs (line 79-82): it currently uses `state.config.opencode_port_start` to create OpenCodeClient — should look up the actual instance port from the mapping/store

  **Key change in integration.rs** — The `spawn_stream_forwarder` method (line 195-246) creates the event processing closure. The `mapping` is available there. Pass session_id into the event handler:
  ```rust
  // In spawn_stream_forwarder, extract session_id before the loop
  let session_id_for_perms = mapping.session_id.clone().unwrap_or_default();
  
  // In the match arm for PermissionRequest (currently line 325):
  let session_id = &session_id_for_perms;
  ```

  **Key change in permissions.rs** — line 79-82: Instead of `state.config.opencode_port_start`, look up the actual instance port:
  ```rust
  // Look up actual port from orchestrator store
  let port = if let Some(store) = ... {
      // Get instance by looking up from callback data's session_id
  };
  ```
  Note: This is harder because the callback only has session_id, not instance_id. The simplest fix is to encode the port in the callback data, or look up through the topic store. Evaluate the simplest approach that works.

  **Must NOT do**:
  - Don't change the StreamEvent enum structure
  - Don't change the handle_stream_event function signature (it's a static method)
  - Don't add new fields to TopicMapping

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Small data threading change
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 3, 4, 5, 7)
  - **Blocks**: Task 10
  - **Blocked By**: Task 2

  **References**:

  **Pattern References**:
  - `src/integration.rs:194-246` — `spawn_stream_forwarder()` method. Has access to `mapping: TopicMapping` which contains `session_id: Option<String>`. Creates closure that processes events.
  - `src/integration.rs:311-338` — PermissionRequest handling in `handle_stream_event()`. Line 325: `let session_id = "";` — the empty string that needs fixing.
  - `src/integration.rs:170-173` — session_id is extracted from mapping in `ensure_stream_subscription()`.

  **API/Type References**:
  - `src/types/forum.rs` (TopicMapping struct) — has `session_id: Option<String>`, `instance_id: Option<String>`
  - `src/bot/handlers/permissions.rs:67-102` — `handle_permission_callback`. Lines 79-82: creates OpenCodeClient with hardcoded port. Needs to use actual instance port.
  - `src/bot/handlers/permissions.rs:17-25` — Callback data format: `perm:{session_id}:{permission_id}:{allow|deny}`. Session ID is available in callback data.

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test` — all tests pass
  - [ ] Permission requests include real session_id (not empty string)
  - [ ] Permission callback uses correct instance port (not hardcoded config port)

  **Commit**: YES
  - Message: `fix: pass real session_id to permission requests and use correct instance port`
  - Files: `src/integration.rs`, `src/bot/handlers/permissions.rs`
  - Pre-commit: `cargo test`

---

- [x] 7. Fix health check loop restart logic

  **What to do**:
  - In `src/orchestrator/manager.rs`, lines 362-382, the crash detection code marks instance as `Error` but never actually restarts
  - The comment says `"simplified - would need more context"` — we now have the context
  - After detecting a crash (line 354: `check_for_crash() -> Ok(true)`), if under MAX_RESTART_ATTEMPTS, actually call restart instead of just marking Error
  - The instance's project_path is available from the store: look up the instance info to get `project_path`, then call `restart_instance_by_path()`
  - Challenge: `restart_instance_by_path()` is a method on `InstanceManager`, but inside `start_health_check_loop()` we only have cloned Arcs — not `&self`. Need to restructure the restart call.
  - Simplest approach: After the backoff delay, look up the instance's project_path from the store, then remove the old instance from the `instances` map, release its port, and spawn a new one via the same code path. OR, make InstanceManager Clone-safe and clone it into the loop.
  - Since InstanceManager already clones its internals (all `Arc<Mutex<>>` fields) into the loop, the restart can be done by duplicating the `spawn_new_instance` logic inline, or by passing a restart callback.
  - **Recommended approach**: Get the `project_path` from the store, then: (1) release the old port, (2) remove from instances map, (3) call the port_pool.allocate + spawn logic inline. This avoids needing `&self` reference.

  **Must NOT do**:
  - Don't change the health check interval or MAX_RESTART_ATTEMPTS
  - Don't add new configuration options
  - Don't break existing manager tests

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Async lock management in background task is tricky
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 3, 4, 5, 6)
  - **Blocks**: Task 10
  - **Blocked By**: Task 2

  **References**:

  **Pattern References**:
  - `src/orchestrator/manager.rs:317-436` — Full `start_health_check_loop()` implementation. Lines 353-394: crash detection and handling. Line 371: `// Try to restart (simplified - would need more context)` — the TODO to fix.
  - `src/orchestrator/manager.rs:534-580` — `restart_instance_by_path()` — existing restart logic that we want to replicate. Gets old port, checks restart tracker, waits backoff, removes old instance, releases port, calls `spawn_new_instance`.

  **API/Type References**:
  - `src/orchestrator/manager.rs:446-532` — `spawn_new_instance()` — allocates port, generates ID, spawns process, waits for ready, saves to DB.
  - `src/orchestrator/instance.rs:50-100` — `OpenCodeInstance::spawn(config, port)` — the actual process spawning.
  - `src/orchestrator/store.rs:70+` — `get_instance(&self, id)` — can retrieve project_path for restart.

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test --lib orchestrator::manager` — existing manager tests pass
  - [ ] When instance crashes and attempts < MAX_RESTART_ATTEMPTS, a restart is attempted (not just Error state)
  - [ ] After MAX_RESTART_ATTEMPTS, instance is marked Error permanently (existing behavior preserved)
  - [ ] Backoff delay is applied before restart (1s, 2s, 4s, 8s, 16s)

  **Commit**: YES
  - Message: `fix(manager): implement auto-restart in health check loop on crash detection`
  - Files: `src/orchestrator/manager.rs`
  - Pre-commit: `cargo test --lib orchestrator::manager`

---

### Wave 4: Cleanup

- [x] 8. Fix config test failures (serial_test)

  **What to do**:
  - Add `serial_test = "3"` to `[dev-dependencies]` in Cargo.toml
  - Add `#[serial]` attribute to ALL config tests that use `std::env::set_var`
  - Import `serial_test::serial` in the test module
  - Root cause: Tests run in parallel and share the process environment. One test sets HANDLE_GENERAL_TOPIC to invalid value, another reads it. `#[serial]` forces sequential execution for these tests.
  - Alternative considered: Per-test env cleanup. Rejected because it's fragile and `serial` is the standard Rust approach.

  **Must NOT do**:
  - Don't change the config parsing logic
  - Don't refactor tests to avoid env vars (would be a larger change)
  - Don't add `serial_test` to non-dev dependencies

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical annotation addition
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (independent of all other tasks)
  - **Parallel Group**: Wave 4 (with Tasks 9, 12)
  - **Blocks**: Tasks 10, 11
  - **Blocked By**: None (can start immediately, even in Wave 1)

  **References**:

  **Pattern References**:
  - `src/config.rs:181-405` — All 16 config tests. 9 fail due to HANDLE_GENERAL_TOPIC env var leaking. Each test uses `std::env::set_var()` without isolation. Tests that fail: `test_missing_telegram_bot_token`, `test_missing_telegram_chat_id`, `test_missing_project_base_path`, `test_defaults_applied_correctly`, `test_path_expansion`, `test_telegram_allowed_users_parsing`, `test_telegram_allowed_users_empty`, `test_masked_display`, `test_invalid_opencode_max_instances`.

  **External References**:
  - `serial_test` crate: https://crates.io/crates/serial_test — provides `#[serial]` attribute for test serialization

  **Acceptance Criteria**:
  - [ ] `cargo test --lib config` — ALL 16 config tests pass (0 failures)
  - [ ] `serial_test` added to dev-dependencies only
  - [ ] Tests can run with `cargo test` (parallel by default, serial tests isolated)

  **Commit**: YES
  - Message: `fix(config): serialize config tests to prevent env var leakage`
  - Files: `Cargo.toml`, `src/config.rs`
  - Pre-commit: `cargo test --lib config`

---

- [x] 9. Remove dead types in types/opencode.rs

  **What to do**:
  - Remove unused types from `src/types/opencode.rs`: `SseEvent`, `ContentBlock`, `ContentDelta`, `MessageMetadata`, `MessageDeltaData`, `ErrorData`
  - These 6 types are defined but never used — the stream_handler has its own internal SSE parsing types
  - Keep: `SessionInfo`, `MessagePart`, `ImageSource`, `Message`, `CreateMessageRequest` (these ARE used)
  - Remove tests that test the removed types: `test_sse_event_message_start`, `test_sse_event_content_block_delta`, `test_sse_event_message_stop`, `test_sse_event_error`
  - Keep tests for retained types

  **Must NOT do**:
  - Don't remove types that ARE used (verify with `cargo build` after removal)
  - Don't modify the stream_handler's internal types
  - Don't add new types

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical deletion
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (independent)
  - **Parallel Group**: Wave 4 (with Tasks 8, 12)
  - **Blocks**: Tasks 10, 11
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `src/types/opencode.rs:42-98` — Dead types to remove: SseEvent (line 42), MessageMetadata (line 67), ContentBlock (line 73), ContentDelta (line 81), MessageDeltaData (line 89), ErrorData (line 94).
  - `src/types/opencode.rs:229-306` — Tests to remove: test_sse_event_message_start (line 229), test_sse_event_content_block_delta (line 251), test_sse_event_message_stop (line 275), test_sse_event_error (line 288).

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test --lib types::opencode` — remaining tests pass
  - [ ] `cargo clippy` reports fewer dead_code warnings (6 fewer)
  - [ ] No SseEvent, ContentBlock, ContentDelta, MessageMetadata, MessageDeltaData, ErrorData in codebase

  **Commit**: YES
  - Message: `cleanup: remove dead SSE types from types/opencode.rs`
  - Files: `src/types/opencode.rs`
  - Pre-commit: `cargo test --lib types::opencode`

---

- [x] 10. Audit and remove unnecessary #[allow(dead_code)] annotations

  **What to do**:
  - After all wiring tasks (1-7) and cleanup tasks (8-9) are complete, audit all 68 `#[allow(dead_code)]` annotations across 21 files
  - For each annotation, check if the code is now actually used (wired into live path)
  - Remove `#[allow(dead_code)]` where code IS now used
  - Keep `#[allow(dead_code)]` only where justified: 
    - Public API methods that may be used externally but aren't called internally
    - Test helper code
    - Types that must exist for serialization but aren't constructed directly
  - Verify with `cargo clippy` after each removal

  **Expected removals** (code that becomes live after wiring):
  - `src/bot/state.rs:7,15` — BotState is used in main.rs
  - `src/orchestrator/manager.rs:32,70,81` — InstanceManager, ManagerStatus are used via BotState
  - `src/orchestrator/port_pool.rs:12,19` — PortPool is used via InstanceManager
  - `src/orchestrator/instance.rs:24,36` — OpenCodeInstance is used via InstanceManager
  - `src/bot/handlers/disconnect.rs:9,25` — handler is registered in dispatcher
  - `src/bot/handlers/permissions.rs:42,66` — handlers are registered in dispatcher
  - `src/bot/handlers/status.rs:17,30,57` — handler is registered in dispatcher
  - `src/integration.rs:48,58,78` — Integration is used in main.rs
  - `src/config.rs:7,39` — Config is used everywhere
  - Various handler files — handlers registered in dispatcher

  **Expected keeps** (code that legitimately needs the annotation):
  - `src/api/mod.rs` — Axum handler functions are called by the framework, not directly. Some struct fields are only serialized, not accessed. May need `#[allow(dead_code)]` for Deserialize-only fields.
  - Some `src/forum/store.rs` methods may not be called yet

  **Must NOT do**:
  - Don't remove annotations that would cause clippy warnings — verify each removal
  - Don't add new `#[allow(dead_code)]` annotations (goal is to reduce)
  - Don't refactor code — only remove annotations

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Systematic audit across many files, needs careful verification
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO (depends on all wiring being done first)
  - **Parallel Group**: Wave 4 (after Tasks 3-9)
  - **Blocks**: Task 11
  - **Blocked By**: Tasks 3, 4, 5, 6, 7, 8, 9

  **References**:

  **Pattern References**:
  - All 21 files with `#[allow(dead_code)]`: `src/opencode/stream_handler.rs`, `src/api/mod.rs`, `src/opencode/discovery.rs`, `src/opencode/client.rs`, `src/config.rs`, `src/orchestrator/manager.rs`, `src/db/mod.rs`, `src/orchestrator/port_pool.rs`, `src/orchestrator/instance.rs`, `src/integration.rs`, `src/bot/commands.rs`, `src/forum/store.rs`, `src/bot/state.rs`, `src/bot/handlers/disconnect.rs`, `src/bot/handlers/permissions.rs`, `src/bot/handlers/status.rs`, `src/bot/handlers/link.rs`, `src/bot/handlers/clear.rs`, `src/bot/handlers/connect.rs`, `src/bot/handlers/stream.rs`, `src/bot/handlers/session.rs`

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo clippy` — check warning count (should decrease)
  - [ ] `cargo test` — all tests pass
  - [ ] Each remaining `#[allow(dead_code)]` has a justified reason
  - [ ] At least 40+ annotations removed (out of 68)

  **Commit**: YES
  - Message: `cleanup: remove unnecessary #[allow(dead_code)] annotations after wiring`
  - Files: All 21 files listed above
  - Pre-commit: `cargo build && cargo clippy && cargo test`

---

- [ ] 11. Fix remaining clippy warnings

  **What to do**:
  - After Task 10 (dead_code annotation cleanup), run `cargo clippy` and fix ALL remaining warnings
  - Current warnings (12 total, may change after wiring):
    - `MessageResponse` struct never constructed (`src/opencode/client.rs:18`)
    - Methods `health`, `list_sessions`, `get_session`, `create_session`, `send_message` never used (`src/opencode/client.rs`)
    - Methods `get_instance_by_port`, `get_active_count` never used (`src/orchestrator/store.rs`)
    - Function `truncate_message` never used (`src/telegram/markdown.rs`)
    - Multiple variants never constructed in `OutpostError` (`src/types/error.rs`)
    - Dead SSE types (handled by Task 9)
  - For genuinely unused code: either remove it or add `#[allow(dead_code)]` with a justification comment
  - For code that WILL be used once more features are implemented: add `#[allow(dead_code)]` with `// Used by future: XYZ` comment

  **Must NOT do**:
  - Don't suppress warnings blindly — evaluate each one
  - Don't remove code that's tested and may be needed in future
  - Don't refactor — just address warnings

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Mechanical fixes following clippy output
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 12)
  - **Parallel Group**: Wave 4 (after Task 10)
  - **Blocks**: None (final cleanup)
  - **Blocked By**: Task 10

  **References**:

  **Pattern References**:
  - `cargo clippy` output — 12 warnings from files listed above
  - `src/opencode/client.rs:18` — MessageResponse struct
  - `src/opencode/client.rs:44-185` — 5 unused methods on OpenCodeClient
  - `src/orchestrator/store.rs:70,120` — 2 unused methods on OrchestratorStore
  - `src/telegram/markdown.rs:189` — unused truncate_message function
  - `src/types/error.rs:4-12` — unused OutpostError variants

  **Acceptance Criteria**:
  - [ ] `cargo clippy` — 0 warnings
  - [ ] `cargo build` compiles without errors
  - [ ] `cargo test` — all tests pass
  - [ ] Any remaining `#[allow(dead_code)]` has a justification comment

  **Commit**: YES
  - Message: `cleanup: fix all remaining clippy warnings`
  - Files: Various (as identified by clippy output)
  - Pre-commit: `cargo clippy && cargo test`

---

- [x] 12. Fix README project structure

  **What to do**:
  - Update `README.md` line 77 (project structure section)
  - Replace `src/storage/` with the actual directory structure
  - Current actual structure:
    ```
    src/
    ├── main.rs          # Application entry point
    ├── config.rs        # Configuration from env vars
    ├── bot/             # Telegram bot logic (handlers, state, commands)
    ├── orchestrator/    # Instance orchestration (manager, port_pool, instance, store)
    ├── opencode/        # OpenCode client, discovery, stream handler
    ├── integration.rs   # Wires bot ↔ OpenCode
    ├── forum/           # Topic store
    ├── db/              # Database initialization
    ├── api/             # External API server (axum)
    ├── telegram/        # Telegram-specific utilities (markdown)
    └── types/           # Shared type definitions
    ```

  **Must NOT do**:
  - Don't rewrite the entire README
  - Don't add feature documentation for the new wiring (that's a separate task)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Single line fix in markdown
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (independent of all other tasks)
  - **Parallel Group**: Wave 4 (with Tasks 8, 9, 10, 11)
  - **Blocks**: None
  - **Blocked By**: None (can start immediately)

  **References**:

  **Pattern References**:
  - `README.md:65-78` — Current project structure section. Line 77 says `├── storage/         # Database layer` — should reflect actual structure.

  **Acceptance Criteria**:
  - [ ] README project structure matches actual `src/` directory structure
  - [ ] No `src/storage/` reference remains
  - [ ] README compiles to valid markdown

  **Commit**: YES
  - Message: `docs: fix project structure in README to match actual layout`
  - Files: `README.md`

---

## Commit Strategy

| After Task | Message | Key Files | Verification |
|------------|---------|-----------|--------------|
| 1 | `refactor(bot): add InstanceManager and bot_start_time to BotState` | `src/bot/state.rs` | `cargo test --lib bot::state` |
| 2 | `feat: wire InstanceManager, PortPool, health checks, and callback handler into main.rs` | `src/main.rs` | `cargo build && cargo test` |
| 3 | `feat(new): wire /new handler to spawn OpenCode processes via InstanceManager` | `src/bot/handlers/new.rs` | `cargo test --lib bot::handlers::new` |
| 4 | `feat(disconnect): wire /disconnect to stop processes via InstanceManager` | `src/bot/handlers/disconnect.rs` | `cargo test --lib bot::handlers::disconnect` |
| 5 | `feat(status): wire /status to real metrics from InstanceManager` | `src/bot/handlers/status.rs` | `cargo test --lib bot::handlers::status` |
| 6 | `fix: pass real session_id to permission requests and use correct instance port` | `src/integration.rs`, `src/bot/handlers/permissions.rs` | `cargo test` |
| 7 | `fix(manager): implement auto-restart in health check loop on crash detection` | `src/orchestrator/manager.rs` | `cargo test --lib orchestrator::manager` |
| 8 | `fix(config): serialize config tests to prevent env var leakage` | `Cargo.toml`, `src/config.rs` | `cargo test --lib config` |
| 9 | `cleanup: remove dead SSE types from types/opencode.rs` | `src/types/opencode.rs` | `cargo test --lib types::opencode` |
| 10 | `cleanup: remove unnecessary #[allow(dead_code)] annotations after wiring` | 21 files | `cargo build && cargo clippy && cargo test` |
| 11 | `cleanup: fix all remaining clippy warnings` | Various | `cargo clippy && cargo test` |
| 12 | `docs: fix project structure in README to match actual layout` | `README.md` | — |

---

## Success Criteria

### Verification Commands
```bash
cargo test                    # Expected: ALL tests pass (0 failures)
cargo clippy                  # Expected: 0 warnings
cargo build --release         # Expected: success
```

### Final Checklist
- [ ] All "Must Have" present (InstanceManager wired, health checks running, handlers using real lifecycle)
- [ ] All "Must NOT Have" absent (no new features, no API changes, no signature changes)
- [ ] All tests pass (346+ pass, 0 fail)
- [ ] Clippy clean (0 warnings)
- [ ] No unnecessary `#[allow(dead_code)]`
- [ ] README accurate
