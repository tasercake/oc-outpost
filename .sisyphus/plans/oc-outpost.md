# oc-outpost: Rust Port of opencode-telegram

## TL;DR

> **Quick Summary**: Port opencode-telegram to Rust using teloxide, with TDD approach, fixing original issues (no tests, cleanup, type safety) while maintaining full feature parity including all three instance types, SSE streaming, and permission flow.
> 
> **Deliverables**:
> - Complete Rust binary `oc-outpost` with all 10 commands
> - Proper Rust tooling (clippy, rustfmt, cargo-nextest, justfile)
> - SQLite persistence with sqlx (async, runtime queries)
> - Full SSE streaming bridge to Telegram
> - HTTP API server for external instance registration
> - Comprehensive test suite (TDD)
> 
> **Estimated Effort**: XL (Large project, 28 tasks)
> **Parallel Execution**: YES - 5 waves
> **Critical Path**: Tooling Setup -> Core Types -> Database -> Bot Framework -> Instance Management -> Streaming -> Integration

---

## Context

### Original Request
Create a Rust clone of https://github.com/huynle/opencode-telegram using teloxide. Direct port with fixes for identified issues. Set up proper Rust developer tooling.

### Interview Summary
**Key Discussions**:
- **Port Approach**: Direct port + fixes (faithful but fix issues)
- **Test Strategy**: TDD (tests first)
- **Platform**: Linux/macOS only
- **Database**: sqlx with runtime queries (async, no compile-time DATABASE_URL)
- **API Server**: Include with optional auth (matching original)
- **Telegram**: Polling only
- **Streaming**: Full SSE streaming with real-time updates
- **Cleanup**: Manual only (/clear command)
- **Docker**: Native only, no Docker
- **Instance Types**: All three (managed, discovered, external)
- **Permissions**: Full inline button flow
- **Project Name**: oc-outpost

**Research Findings**:
- Original is ~3000+ lines TypeScript/Bun across 15+ files
- Uses grammY (similar to teloxide conceptually)
- SSE streaming with 2-second throttled updates
- Two SQLite databases (orchestrator.db, topics.db)
- Process discovery via `ps aux` + `lsof -p`
- Port pool 4100-4199, kills orphaned processes

### Metis Review
**Identified Gaps** (addressed):
- Instance type scope: All three confirmed
- Permission flow: Inline buttons confirmed
- Database approach: sqlx runtime queries (no compile-time check)
- Process discovery: Will need `lsof` command, sysinfo alone insufficient
- Message deduplication: Implement standard approach from original
- Forum topics only: Confirmed (supergroups)

---

## Work Objectives

### Core Objective
Build a production-ready Rust Telegram bot that orchestrates multiple OpenCode AI instances through forum topics, with full feature parity to the original TypeScript implementation.

### Concrete Deliverables
- `oc-outpost` binary (release build)
- 10 bot commands: /new, /sessions, /connect, /disconnect, /link, /stream, /session, /status, /clear, /help
- HTTP API server on configurable port (default 4200)
- SQLite databases: orchestrator.db, topics.db
- Test suite with >80% coverage on core logic
- justfile with dev/test/build tasks
- Configuration via environment variables

### Definition of Done
- [x] `cargo build --release` produces working binary
- [x] `cargo nextest run` passes all tests
- [ ] `cargo clippy -- -D warnings` has no warnings
- [ ] Bot responds to all 10 commands correctly
- [ ] SSE streaming shows real-time OpenCode progress
- [ ] Permission requests show inline buttons
- [ ] Process discovery finds existing TUI sessions
- [ ] API server accepts external registrations

### Must Have
- All 10 commands from original
- Three instance types (managed, discovered, external)
- SSE streaming with throttled Telegram updates
- Permission flow with inline buttons
- SQLite persistence surviving restarts
- Process discovery via ps/lsof
- Port pool management (4100-4199)
- Graceful shutdown with resource cleanup

### Must NOT Have (Guardrails)
- **No webhook mode** - Polling only as specified
- **No Docker** - Native binary only
- **No auto-cleanup** - Manual /clear only
- **No private chat support** - Forum topics only
- **No Redis** - SQLite only
- **No OAuth/JWT** - Simple API key only
- **No extra commands** - Only the 10 specified
- **No Windows support** - Linux/macOS only
- **No "improved" error messages** - Match original behavior

---

## Verification Strategy (MANDATORY)

### Test Decision
- **Infrastructure exists**: NO (blank directory)
- **User wants tests**: TDD
- **Framework**: cargo-nextest with built-in test framework

### TDD Workflow
Each TODO follows RED-GREEN-REFACTOR:

1. **RED**: Write failing test first
   - Test command: `cargo nextest run -E 'test(module_name)'`
   - Expected: FAIL (test exists, implementation doesn't)
2. **GREEN**: Implement minimum code to pass
   - Command: `cargo nextest run -E 'test(module_name)'`
   - Expected: PASS
3. **REFACTOR**: Clean up while keeping green

### Test Setup Task
- [ ] 0. Setup test infrastructure in first task
  - Install: Part of Cargo.toml
  - Config: Create test utilities module
  - Verify: `cargo nextest run` shows test framework working

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 1 (Foundation - Start Immediately):
+-- Task 1: Project scaffolding & tooling
+-- Task 2: Core type definitions
+-- Task 3: Configuration module

Wave 2 (Storage & Framework - After Wave 1):
+-- Task 4: Database schemas & migrations
+-- Task 5: OrchestratorStore implementation
+-- Task 6: TopicStore implementation
+-- Task 7: Bot framework setup (teloxide)

Wave 3 (Instance Management - After Wave 2):
+-- Task 8: PortPool implementation
+-- Task 9: OpenCodeInstance (managed)
+-- Task 10: InstanceManager
+-- Task 11: Process discovery
+-- Task 12: OpenCode REST client

Wave 4 (Telegram Features - After Wave 3):
+-- Task 13: SSE StreamHandler
+-- Task 14: Telegram Markdown converter
+-- Task 15: Command: /new
+-- Task 16: Command: /sessions
+-- Task 17: Command: /connect
+-- Task 18: Command: /disconnect
+-- Task 19: Command: /link
+-- Task 20: Command: /stream
+-- Task 21: Command: /session
+-- Task 22: Command: /status
+-- Task 23: Command: /clear
+-- Task 24: Command: /help
+-- Task 25: Permission inline buttons

Wave 5 (API & Integration - After Wave 4):
+-- Task 26: HTTP API server
+-- Task 27: Integration layer
+-- Task 28: Main entry point & graceful shutdown
```

### Dependency Matrix

| Task | Depends On | Blocks | Can Parallelize With |
|------|------------|--------|---------------------|
| 1 | None | 2-28 | None (must be first) |
| 2 | 1 | 4-28 | 3 |
| 3 | 1 | 7, 27 | 2 |
| 4 | 2 | 5, 6 | None |
| 5 | 4 | 10 | 6, 7 |
| 6 | 4 | 15-24, 27 | 5, 7 |
| 7 | 3 | 15-28 | 5, 6 |
| 8 | 2 | 9 | 5, 6, 7 |
| 9 | 8 | 10 | 11, 12 |
| 10 | 5, 9 | 27 | 11, 12, 13 |
| 11 | 2 | 10, 16, 17 | 9, 12 |
| 12 | 2 | 13, 15-24 | 9, 11 |
| 13 | 12 | 27 | 14 |
| 14 | 2 | 13 | 12 |
| 15-24 | 6, 7, 12 | 27 | Each other |
| 25 | 7, 13 | 27 | 15-24 |
| 26 | 2, 10 | 27 | 15-25 |
| 27 | 6, 7, 10, 13, 15-26 | 28 | None |
| 28 | 27 | None | None (final) |

---

## TODOs

### Wave 1: Foundation

- [x] 1. Project Scaffolding & Rust Tooling

  **What to do**:
  - Initialize Cargo project with `cargo init --name oc-outpost`
  - Create workspace structure (single crate for now, can split later)
  - Set up Cargo.toml with all dependencies
  - Create rustfmt.toml with project formatting rules
  - Create clippy.toml with lint configuration
  - Create .cargo/config.toml for build optimizations
  - Create justfile with common tasks
  - Create .env.example with all config vars
  - Create .gitignore for Rust projects
  - Initialize git repository
  - Add README.md stub

  **Dependencies (Cargo.toml)**:
  ```toml
  [package]
  name = "oc-outpost"
  version = "0.1.0"
  edition = "2024"
  rust-version = "1.82"
  
  [dependencies]
  teloxide = { version = "0.17", features = ["macros", "throttle"] }
  tokio = { version = "1", features = ["full"] }
  sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
  reqwest = { version = "0.12", features = ["json", "stream"] }
  reqwest-eventsource = "0.7"
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  dotenvy = "0.15"
  tracing = "0.1"
  tracing-subscriber = { version = "0.3", features = ["env-filter"] }
  anyhow = "1"
  thiserror = "2"
  axum = "0.8"
  tower-http = { version = "0.6", features = ["cors"] }
  sysinfo = "0.32"
  futures = "0.3"
  async-stream = "0.3"
  
  [dev-dependencies]
  tokio-test = "0.4"
  tempfile = "3"
  
  [profile.release]
  opt-level = 3
  lto = true
  codegen-units = 1
  strip = true
  ```

  **Must NOT do**:
  - Don't add unnecessary dependencies
  - Don't set up Docker files
  - Don't add workspace structure yet (keep simple)

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Scaffolding is straightforward file creation
  - **Skills**: [`git-master`]
    - `git-master`: For proper git initialization and .gitignore

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 1 (first)
  - **Blocks**: All subsequent tasks
  - **Blocked By**: None

  **References**:
  - Original package.json: https://github.com/huynle/opencode-telegram/blob/main/package.json
  - Rust tooling research in interview notes
  - teloxide docs: https://docs.rs/teloxide/latest/teloxide/

  **Acceptance Criteria**:
  - [ ] `cargo build` compiles successfully (no code yet, just dependencies)
  - [ ] `cargo fmt --check` passes
  - [ ] `cargo clippy` runs without configuration errors
  - [ ] `just --list` shows available commands
  - [ ] `.env.example` contains all config vars from original
  - [ ] `git status` shows initialized repo

  **Commit**: YES
  - Message: `feat: initialize oc-outpost project with Rust tooling`
  - Files: `Cargo.toml, rustfmt.toml, clippy.toml, .cargo/config.toml, justfile, .env.example, .gitignore, README.md, src/main.rs`

---

- [x] 2. Core Type Definitions

  **What to do**:
  - Create `src/types/mod.rs` module structure
  - Define `InstanceState` enum (Starting, Running, Stopping, Stopped, Error)
  - Define `InstanceType` enum (Managed, Discovered, External)
  - Define `InstanceConfig` struct
  - Define `InstanceInfo` struct (runtime info)
  - Define `TopicMapping` struct
  - Define `SessionInfo` struct
  - Define OpenCode API types (messages, parts, events)
  - Define SSE event types
  - Define error types with thiserror
  - Write unit tests for type serialization/deserialization

  **Test cases**:
  - InstanceState serializes to expected strings
  - OpenCode API types deserialize from JSON samples
  - Error types implement Display and Error traits

  **Must NOT do**:
  - Don't implement business logic in types
  - Don't add database-specific code here

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Type definitions are straightforward Rust code
  - **Skills**: []
    - No special skills needed

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 3)
  - **Blocks**: Tasks 4-28
  - **Blocked By**: Task 1

  **References**:
  - Original types: https://github.com/huynle/opencode-telegram/blob/main/src/types/orchestrator.ts
  - Original forum types: https://github.com/huynle/opencode-telegram/blob/main/src/types/forum.ts
  - Original OpenCode types: https://github.com/huynle/opencode-telegram/blob/main/src/types/opencode/types.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(types)'` passes
  - [ ] All types implement Clone, Debug
  - [ ] Serializable types implement Serialize, Deserialize
  - [ ] Error types implement thiserror::Error

  **Commit**: YES
  - Message: `feat(types): add core type definitions`
  - Files: `src/types/*.rs`

---

- [x] 3. Configuration Module

  **What to do**:
  - Create `src/config.rs`
  - Define `Config` struct with all settings from original
  - Implement config loading from environment variables
  - Add validation for required fields
  - Add sensible defaults for optional fields
  - Implement sensitive value masking for logging
  - Write tests for config parsing and validation

  **Config fields** (from original):
  ```rust
  struct Config {
      // Telegram
      telegram_bot_token: String,       // Required
      telegram_chat_id: i64,            // Required
      telegram_allowed_users: Vec<i64>, // Optional
      handle_general_topic: bool,       // Default: true
      
      // OpenCode
      opencode_path: PathBuf,           // Default: "opencode"
      opencode_max_instances: usize,    // Default: 10
      opencode_idle_timeout: Duration,  // Default: 30min
      opencode_port_start: u16,         // Default: 4100
      opencode_port_pool_size: u16,     // Default: 100
      opencode_health_check_interval: Duration, // Default: 30s
      opencode_startup_timeout: Duration, // Default: 60s
      
      // Storage
      orchestrator_db_path: PathBuf,    // Default: ./data/orchestrator.db
      topic_db_path: PathBuf,           // Default: ./data/topics.db
      
      // Project
      project_base_path: PathBuf,       // Required
      auto_create_project_dirs: bool,   // Default: true
      
      // API
      api_port: u16,                    // Default: 4200
      api_key: Option<String>,          // Optional
  }
  ```

  **Test cases**:
  - Missing required field returns error
  - Defaults applied when optional field missing
  - Duration parsing works (ms suffix)
  - Path expansion works (~/)

  **Must NOT do**:
  - Don't read config files (env vars only, like original)
  - Don't add hot-reload capability

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Config module is standard pattern
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 1 (with Task 2)
  - **Blocks**: Tasks 7, 27
  - **Blocked By**: Task 1

  **References**:
  - Original config: https://github.com/huynle/opencode-telegram/blob/main/src/config.ts
  - Original .env.example: https://github.com/huynle/opencode-telegram/blob/main/.env.example

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(config)'` passes
  - [ ] Missing TELEGRAM_BOT_TOKEN causes error with clear message
  - [ ] `Config::masked_display()` hides token

  **Commit**: YES
  - Message: `feat(config): add configuration module`
  - Files: `src/config.rs`

---

### Wave 2: Storage & Framework

- [x] 4. Database Schemas & Migrations

  **What to do**:
  - Create `src/db/mod.rs` module
  - Create `migrations/` directory with SQL files
  - Define orchestrator schema (instances table)
  - Define topics schema (topic_mappings table)
  - Implement migration runner using sqlx
  - Create database initialization function
  - Write tests for schema creation

  **Orchestrator schema**:
  ```sql
  CREATE TABLE IF NOT EXISTS instances (
      id TEXT PRIMARY KEY,
      project_path TEXT NOT NULL,
      port INTEGER NOT NULL,
      state TEXT NOT NULL,
      instance_type TEXT NOT NULL,
      session_id TEXT,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL
  );
  ```

  **Topics schema**:
  ```sql
  CREATE TABLE IF NOT EXISTS topic_mappings (
      topic_id INTEGER PRIMARY KEY,
      chat_id INTEGER NOT NULL,
      project_path TEXT NOT NULL,
      session_id TEXT,
      instance_id TEXT,
      streaming_enabled INTEGER NOT NULL DEFAULT 1,
      topic_name_updated INTEGER NOT NULL DEFAULT 0,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL
  );
  ```

  **Must NOT do**:
  - Don't use sqlx macros (runtime queries only)
  - Don't add ORM abstractions

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Database setup is standard work
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 2 (first in wave)
  - **Blocks**: Tasks 5, 6
  - **Blocked By**: Task 2

  **References**:
  - Original state store: https://github.com/huynle/opencode-telegram/blob/main/src/orchestrator/state-store.ts
  - Original topic store: https://github.com/huynle/opencode-telegram/blob/main/src/forum/topic-store.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(db)'` passes
  - [ ] Databases created at configured paths
  - [ ] Tables exist after migration
  - [ ] Re-running migration is idempotent

  **Commit**: YES
  - Message: `feat(db): add database schemas and migrations`
  - Files: `src/db/*.rs, migrations/*.sql`

---

- [x] 5. OrchestratorStore Implementation

  **What to do**:
  - Create `src/orchestrator/store.rs`
  - Implement CRUD operations for instances
  - Implement query methods (by id, by port, by project_path)
  - Implement state update methods
  - Use sqlx runtime queries (not macros)
  - Write comprehensive tests with in-memory SQLite

  **Methods**:
  ```rust
  impl OrchestratorStore {
      async fn new(db_path: &Path) -> Result<Self>;
      async fn save_instance(&self, instance: &InstanceInfo) -> Result<()>;
      async fn get_instance(&self, id: &str) -> Result<Option<InstanceInfo>>;
      async fn get_instance_by_port(&self, port: u16) -> Result<Option<InstanceInfo>>;
      async fn get_instance_by_path(&self, path: &Path) -> Result<Option<InstanceInfo>>;
      async fn get_all_instances(&self) -> Result<Vec<InstanceInfo>>;
      async fn update_state(&self, id: &str, state: InstanceState) -> Result<()>;
      async fn delete_instance(&self, id: &str) -> Result<()>;
      async fn get_active_count(&self) -> Result<usize>;
  }
  ```

  **Must NOT do**:
  - Don't implement business logic here (just storage)
  - Don't use connection per query (use pool)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Standard database layer implementation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 6, 7)
  - **Blocks**: Task 10
  - **Blocked By**: Task 4

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/orchestrator/state-store.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(orchestrator::store)'` passes
  - [ ] CRUD operations work correctly
  - [ ] Concurrent access doesn't cause errors
  - [ ] Connection pool reused across queries

  **Commit**: YES
  - Message: `feat(orchestrator): add OrchestratorStore`
  - Files: `src/orchestrator/store.rs`

---

- [x] 6. TopicStore Implementation

  **What to do**:
  - Create `src/forum/store.rs`
  - Implement CRUD for topic mappings
  - Implement query methods (by topic_id, by chat_id, by session_id)
  - Implement streaming preference toggle
  - Implement topic name update tracking
  - Write comprehensive tests

  **Methods**:
  ```rust
  impl TopicStore {
      async fn new(db_path: &Path) -> Result<Self>;
      async fn save_mapping(&self, mapping: &TopicMapping) -> Result<()>;
      async fn get_mapping(&self, topic_id: i64) -> Result<Option<TopicMapping>>;
      async fn get_mappings_by_chat(&self, chat_id: i64) -> Result<Vec<TopicMapping>>;
      async fn get_mapping_by_session(&self, session_id: &str) -> Result<Option<TopicMapping>>;
      async fn update_session(&self, topic_id: i64, session_id: &str) -> Result<()>;
      async fn toggle_streaming(&self, topic_id: i64) -> Result<bool>;
      async fn mark_topic_name_updated(&self, topic_id: i64) -> Result<()>;
      async fn delete_mapping(&self, topic_id: i64) -> Result<()>;
      async fn get_stale_mappings(&self, older_than: Duration) -> Result<Vec<TopicMapping>>;
  }
  ```

  **Must NOT do**:
  - Don't implement Telegram topic operations here (just storage)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Standard database layer implementation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 7)
  - **Blocks**: Tasks 15-24, 27
  - **Blocked By**: Task 4

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/forum/topic-store.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(forum::store)'` passes
  - [ ] Mappings persist across reconnects
  - [ ] Streaming toggle returns new state

  **Commit**: YES
  - Message: `feat(forum): add TopicStore`
  - Files: `src/forum/store.rs`

---

- [x] 7. Bot Framework Setup (teloxide)

  **What to do**:
  - Create `src/bot/mod.rs` module structure
  - Set up teloxide Bot with throttling
  - Create dispatcher schema with dptree
  - Define Command enum with BotCommands derive
  - Set up update filtering for forum topics
  - Create handler function signatures (not implementations yet)
  - Add bot state struct for dependency injection
  - Write tests for command parsing

  **Command enum**:
  ```rust
  #[derive(BotCommands, Clone)]
  #[command(rename_rule = "lowercase")]
  enum Command {
      #[command(description = "Create new project and session")]
      New(String),
      #[command(description = "List all sessions")]
      Sessions,
      #[command(description = "Connect to existing session")]
      Connect(String),
      #[command(description = "Disconnect and delete topic")]
      Disconnect,
      #[command(description = "Link topic to directory")]
      Link(String),
      #[command(description = "Toggle streaming")]
      Stream,
      #[command(description = "Show session info")]
      Session,
      #[command(description = "Show orchestrator status")]
      Status,
      #[command(description = "Clear stale mappings")]
      Clear,
      #[command(description = "Show help")]
      Help,
  }
  ```

  **Must NOT do**:
  - Don't implement command handlers yet (just structure)
  - Don't add webhook support

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Framework setup following teloxide patterns
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 2 (with Tasks 5, 6)
  - **Blocks**: Tasks 15-28
  - **Blocked By**: Task 3

  **References**:
  - teloxide docs: https://docs.rs/teloxide/latest/teloxide/
  - teloxide examples: https://github.com/teloxide/teloxide/tree/master/crates/teloxide/examples
  - Original handlers: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(bot)'` passes
  - [ ] Commands parse correctly from strings
  - [ ] Bot compiles with dispatcher schema

  **Commit**: YES
  - Message: `feat(bot): set up teloxide bot framework`
  - Files: `src/bot/*.rs`

---

### Wave 3: Instance Management

- [x] 8. PortPool Implementation

  **What to do**:
  - Create `src/orchestrator/port_pool.rs`
  - Implement port allocation from configurable range
  - Implement port release
  - Implement orphan cleanup (kill process on port)
  - Use `lsof -ti:PORT` to check port usage
  - Write tests (mock lsof for unit tests)

  **Methods**:
  ```rust
  impl PortPool {
      fn new(start: u16, size: u16) -> Self;
      async fn allocate(&self) -> Result<u16>;
      async fn release(&self, port: u16);
      async fn is_available(&self, port: u16) -> bool;
      async fn cleanup_orphan(&self, port: u16) -> Result<()>;
      fn allocated_count(&self) -> usize;
  }
  ```

  **Must NOT do**:
  - Don't implement Windows support
  - Don't add port persistence (in-memory tracking is fine)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: System-level port management
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 9-12)
  - **Blocks**: Task 9
  - **Blocked By**: Task 2

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/orchestrator/port-pool.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(port_pool)'` passes
  - [ ] Ports allocated sequentially from start
  - [ ] Released ports can be reused
  - [ ] Orphan cleanup kills process (integration test)

  **Commit**: YES
  - Message: `feat(orchestrator): add PortPool`
  - Files: `src/orchestrator/port_pool.rs`

---

- [x] 9. OpenCodeInstance Implementation

  **What to do**:
  - Create `src/orchestrator/instance.rs`
  - Implement process spawning (`opencode serve --port PORT --project PATH`)
  - Implement health check polling (`GET /global/health`)
  - Implement graceful shutdown (SIGTERM, then SIGKILL)
  - Implement crash detection
  - Track instance state transitions
  - Write tests with mock process

  **Methods**:
  ```rust
  impl OpenCodeInstance {
      async fn spawn(config: InstanceConfig, port: u16) -> Result<Self>;
      async fn health_check(&self) -> Result<bool>;
      async fn stop(&mut self) -> Result<()>;
      fn state(&self) -> InstanceState;
      fn port(&self) -> u16;
      fn project_path(&self) -> &Path;
      fn session_id(&self) -> Option<&str>;
  }
  ```

  **Must NOT do**:
  - Don't implement auto-restart here (InstanceManager does that)
  - Don't implement SSE subscription here

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Process management is complex
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 8, 10-12)
  - **Blocks**: Task 10
  - **Blocked By**: Task 8

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/orchestrator/instance.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(instance)'` passes
  - [ ] Process spawns and responds to health check
  - [ ] Graceful shutdown works
  - [ ] State transitions tracked correctly

  **Commit**: YES
  - Message: `feat(orchestrator): add OpenCodeInstance`
  - Files: `src/orchestrator/instance.rs`

---

- [x] 10. InstanceManager Implementation

  **What to do**:
  - Create `src/orchestrator/manager.rs`
  - Implement instance lifecycle coordination
  - Implement resource limits (max instances)
  - Implement auto-restart with exponential backoff
  - Implement periodic health checks
  - Implement idle timeout handling
  - Integrate with OrchestratorStore for persistence
  - Integrate with PortPool
  - Write comprehensive tests

  **Methods**:
  ```rust
  impl InstanceManager {
      async fn new(config: &Config, store: OrchestratorStore, port_pool: PortPool) -> Result<Self>;
      async fn get_or_create(&self, project_path: &Path) -> Result<Arc<OpenCodeInstance>>;
      async fn get_instance(&self, id: &str) -> Option<Arc<OpenCodeInstance>>;
      async fn get_instance_by_path(&self, path: &Path) -> Option<Arc<OpenCodeInstance>>;
      async fn stop_instance(&self, id: &str) -> Result<()>;
      async fn stop_all(&self) -> Result<()>;
      async fn get_status(&self) -> ManagerStatus;
      async fn recover_from_db(&self) -> Result<()>;
  }
  ```

  **Must NOT do**:
  - Don't implement discovered/external instance handling here
  - Don't implement Telegram integration

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Complex coordination logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (after 5, 9)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 5, 9

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/orchestrator/manager.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(manager)'` passes
  - [ ] Max instances limit enforced
  - [ ] Auto-restart works with backoff
  - [ ] Recovery from DB works after restart

  **Commit**: YES
  - Message: `feat(orchestrator): add InstanceManager`
  - Files: `src/orchestrator/manager.rs`

---

- [x] 11. Process Discovery Implementation

  **What to do**:
  - Create `src/opencode/discovery.rs`
  - Implement process discovery via `ps aux | grep opencode`
  - Implement port detection via `lsof -p PID`
  - Implement working directory detection
  - Implement session query via OpenCode REST API
  - Distinguish TUI vs serve mode
  - Write tests with mock commands

  **Methods**:
  ```rust
  impl Discovery {
      async fn discover_all() -> Result<Vec<DiscoveredInstance>>;
      async fn discover_by_path(path: &Path) -> Result<Option<DiscoveredInstance>>;
      async fn get_session_info(port: u16) -> Result<Option<SessionInfo>>;
  }
  
  struct DiscoveredInstance {
      pid: u32,
      port: Option<u16>,
      working_dir: PathBuf,
      mode: OpenCodeMode, // Tui or Serve
  }
  ```

  **Must NOT do**:
  - Don't implement Windows support
  - Don't add persistent caching

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: System command parsing is tricky
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 9, 12)
  - **Blocks**: Tasks 10, 16, 17
  - **Blocked By**: Task 2

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/opencode/discovery.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(discovery)'` passes
  - [ ] Discovers running opencode processes
  - [ ] Extracts port and working directory
  - [ ] Distinguishes TUI from serve mode

  **Commit**: YES
  - Message: `feat(opencode): add process discovery`
  - Files: `src/opencode/discovery.rs`

---

- [x] 12. OpenCode REST Client

  **What to do**:
  - Create `src/opencode/client.rs`
  - Implement REST API wrapper for OpenCode
  - Implement session management (create, list, get)
  - Implement message sending (sync and async)
  - Implement SSE subscription URL generation
  - Handle HTTP errors appropriately
  - Write tests with mock server

  **Methods**:
  ```rust
  impl OpenCodeClient {
      fn new(base_url: &str) -> Self;
      async fn health(&self) -> Result<bool>;
      async fn list_sessions(&self) -> Result<Vec<SessionInfo>>;
      async fn get_session(&self, id: &str) -> Result<SessionInfo>;
      async fn create_session(&self, project_path: &Path) -> Result<SessionInfo>;
      async fn send_message(&self, session_id: &str, text: &str) -> Result<MessageResponse>;
      async fn send_message_async(&self, session_id: &str, text: &str) -> Result<()>;
      fn sse_url(&self, session_id: &str) -> String;
      async fn reply_permission(&self, session_id: &str, permission_id: &str, allow: bool) -> Result<()>;
  }
  ```

  **Must NOT do**:
  - Don't implement SSE handling here (StreamHandler does that)
  - Don't implement retry logic (caller handles)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Standard HTTP client wrapper
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 3 (with Tasks 9, 11)
  - **Blocks**: Tasks 13, 15-24
  - **Blocked By**: Task 2

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/opencode/client.ts
  - OpenCode API (discovered from original): /session, /session/:id, /session/:id/prompt, /session/:id/prompt_async

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(client)'` passes
  - [ ] All API methods work with mock server
  - [ ] Error responses handled correctly

  **Commit**: YES
  - Message: `feat(opencode): add REST client`
  - Files: `src/opencode/client.rs`

---

### Wave 4: Telegram Features

- [x] 13. SSE StreamHandler Implementation

  **What to do**:
  - Create `src/opencode/stream_handler.rs`
  - Implement SSE connection using reqwest-eventsource
  - Parse OpenCode SSE events (message.part.updated, tool.execute, etc.)
  - Implement message batching (2-second throttle)
  - Implement reconnection with exponential backoff
  - Implement message deduplication (prevent Telegram echo)
  - Emit events via channel for Telegram integration
  - Write tests with mock SSE server

  **Events to handle**:
  - `message.part.updated` - Text chunk, tool invocation, tool result
  - `message.updated` - Complete message
  - `session.idle` - Session ready for input
  - `session.error` - Session error
  - `permission.updated` - Permission request
  - `permission.replied` - Permission response

  **Methods**:
  ```rust
  impl StreamHandler {
      fn new(client: OpenCodeClient) -> Self;
      async fn subscribe(&self, session_id: &str) -> Result<Receiver<StreamEvent>>;
      fn mark_from_telegram(&self, session_id: &str, text: &str);
      async fn unsubscribe(&self, session_id: &str);
  }
  ```

  **Must NOT do**:
  - Don't send to Telegram directly (emit events)
  - Don't implement message formatting here

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: SSE + async event handling is complex
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (start early)
  - **Blocks**: Task 27
  - **Blocked By**: Task 12

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/opencode/stream-handler.ts
  - reqwest-eventsource docs: https://docs.rs/reqwest-eventsource

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(stream_handler)'` passes
  - [ ] SSE events parsed correctly
  - [ ] Reconnection works after disconnect
  - [ ] Deduplication prevents echo

  **Commit**: YES
  - Message: `feat(opencode): add SSE StreamHandler`
  - Files: `src/opencode/stream_handler.rs`

---

- [x] 14. Telegram Markdown Converter

  **What to do**:
  - Create `src/telegram/markdown.rs`
  - Implement Markdown to Telegram HTML conversion
  - Handle code blocks with syntax highlighting hints
  - Handle inline code, bold, italic, links
  - Implement message truncation (4096 char limit)
  - Implement message splitting for long content
  - Preserve code block integrity when splitting
  - Write comprehensive tests

  **Methods**:
  ```rust
  fn markdown_to_telegram_html(text: &str) -> String;
  fn truncate_message(text: &str, max_len: usize) -> String;
  fn split_message(text: &str, max_len: usize) -> Vec<String>;
  ```

  **Must NOT do**:
  - Don't implement full CommonMark (Telegram subset only)
  - Don't add external Markdown parser dependency

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Text processing, well-defined scope
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with Task 13)
  - **Blocks**: Task 13
  - **Blocked By**: Task 2

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/opencode/telegram-markdown.ts
  - Telegram HTML format: https://core.telegram.org/bots/api#html-style

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(markdown)'` passes
  - [ ] Code blocks converted correctly
  - [ ] Messages split preserving code blocks
  - [ ] Output valid Telegram HTML

  **Commit**: YES
  - Message: `feat(telegram): add Markdown converter`
  - Files: `src/telegram/markdown.rs`

---

- [x] 15. Command: /new

  **What to do**:
  - Implement `/new <name>` command handler
  - Create project directory if auto_create_project_dirs enabled
  - Create Telegram forum topic
  - Spawn OpenCode instance via InstanceManager
  - Create topic mapping in TopicStore
  - Send confirmation message with session info
  - Handle errors (dir exists, max instances, etc.)
  - Write integration tests

  **Behavior**:
  1. Validate name (no special chars, reasonable length)
  2. Check if General topic (allowed if HANDLE_GENERAL_TOPIC)
  3. Create directory at PROJECT_BASE_PATH/name
  4. Create forum topic with name
  5. Get or create OpenCode instance
  6. Create topic mapping
  7. Send success message

  **Must NOT do**:
  - Don't auto-subscribe to SSE yet (user sends message first)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Command handler with defined flow
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 6, 7, 12

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleNewCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_new)'` passes
  - [ ] Directory created at correct path
  - [ ] Topic created with name
  - [ ] Mapping stored in database
  - [ ] Error on invalid name

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /new command`
  - Files: `src/bot/handlers/new.rs`

---

- [x] 16. Command: /sessions

  **What to do**:
  - Implement `/sessions` command handler
  - List managed instances from InstanceManager
  - List discovered instances from Discovery
  - List external instances from API registry
  - Format output with categories and details
  - Handle pagination if many sessions
  - Write integration tests

  **Output format**:
  ```
  Active Sessions (3)

  my-app (managed)
  ~/oc-bot/my-app
  ses_abc123...

  other-project (discovered)
  ~/projects/other-project
  ses_def456...

  external-proj (external)
  ~/external/proj
  ses_ghi789...
  ```

  **Must NOT do**:
  - Don't add filtering options (not in original)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Data aggregation and formatting
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 6, 7, 11, 12

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleSessionsCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_sessions)'` passes
  - [ ] Lists all three instance types
  - [ ] Correct categorization
  - [ ] Handles empty list gracefully

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /sessions command`
  - Files: `src/bot/handlers/sessions.rs`

---

- [x] 17. Command: /connect

  **What to do**:
  - Implement `/connect <name>` command handler
  - Search for session by name/ID across all instance types
  - Create forum topic for the connection
  - Create topic mapping
  - Subscribe to SSE stream
  - Send confirmation message
  - Handle not found, already connected errors
  - Write integration tests

  **Search order**:
  1. Managed instances (by project name or session ID)
  2. Discovered instances (by path or session ID)
  3. External instances (by project name or session ID)

  **Must NOT do**:
  - Don't create new OpenCode instance (use existing)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Search and connection logic
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 6, 7, 11, 12

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleConnectCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_connect)'` passes
  - [ ] Connects to managed instance
  - [ ] Connects to discovered TUI session
  - [ ] Error on not found

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /connect command`
  - Files: `src/bot/handlers/connect.rs`

---

- [x] 18. Command: /disconnect

  **What to do**:
  - Implement `/disconnect` command handler (topic-only)
  - Get topic mapping
  - Stop managed instance (if managed)
  - Unsubscribe from SSE
  - Delete topic mapping
  - Delete Telegram forum topic
  - Send confirmation (before deleting topic)
  - Write integration tests

  **Must NOT do**:
  - Don't stop discovered/external instances (just disconnect)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Cleanup operations
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 6, 7, 10

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleDisconnectCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_disconnect)'` passes
  - [ ] Managed instance stopped
  - [ ] Topic deleted
  - [ ] Mapping removed from DB

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /disconnect command`
  - Files: `src/bot/handlers/disconnect.rs`

---

- [x] 19. Command: /link

  **What to do**:
  - Implement `/link <path>` command handler (topic-only)
  - Validate path exists and is directory
  - Get or create OpenCode instance for path
  - Update topic mapping with new path
  - Send confirmation message
  - Handle path not found, permission errors
  - Write integration tests

  **Path handling**:
  - Expand ~ to home directory
  - Resolve relative paths to absolute
  - Validate directory exists

  **Must NOT do**:
  - Don't support symlinks specially

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple path validation and update
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 6, 7, 10

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleLinkCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_link)'` passes
  - [ ] Path validation works
  - [ ] ~ expansion works
  - [ ] Error on non-existent path

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /link command`
  - Files: `src/bot/handlers/link.rs`

---

- [x] 20. Command: /stream

  **What to do**:
  - Implement `/stream` command handler (topic-only)
  - Toggle streaming preference in TopicStore
  - If enabling, subscribe to SSE if not already
  - If disabling, unsubscribe from SSE
  - Send confirmation with new state
  - Write integration tests

  **Output**:
  ```
  Streaming: ON
  You will see real-time progress from OpenCode.
  ```
  or
  ```
  Streaming: OFF
  You will only see final responses.
  ```

  **Must NOT do**:
  - Don't add streaming quality options

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Simple toggle operation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 6, 7, 13

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleStreamCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_stream)'` passes
  - [ ] Toggle persists in database
  - [ ] SSE subscription managed correctly

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /stream command`
  - Files: `src/bot/handlers/stream.rs`

---

- [x] 21. Command: /session

  **What to do**:
  - Implement `/session` command handler (topic-only)
  - Get topic mapping
  - Get instance info (managed/discovered/external)
  - Query OpenCode API for session details
  - Format and send session info
  - Write integration tests

  **Output**:
  ```
  Session Info

  Type: Managed
  Session: ses_abc123456
  Project: ~/oc-bot/my-app
  Port: 4101
  Status: Running
  Streaming: ON
  Created: 2026-01-29 10:30:00
  ```

  **Must NOT do**:
  - Don't add message history

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Data retrieval and formatting
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 6, 7, 12

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleSessionCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_session)'` passes
  - [ ] Shows correct instance type
  - [ ] Shows all relevant details

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /session command`
  - Files: `src/bot/handlers/session.rs`

---

- [x] 22. Command: /status

  **What to do**:
  - Implement `/status` command handler (General topic)
  - Get InstanceManager status
  - Count managed/discovered/external instances
  - Show port pool usage
  - Show uptime
  - Format and send status
  - Write integration tests

  **Output**:
  ```
  Orchestrator Status

  Managed Instances: 3/10
  Discovered Sessions: 2
  External Instances: 1
  
  Port Pool: 4/100 used
  Uptime: 2h 15m
  Health: Healthy
  ```

  **Must NOT do**:
  - Don't add memory/CPU metrics

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Status aggregation
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 7, 10, 11

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleStatusCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_status)'` passes
  - [ ] Counts accurate
  - [ ] Port pool usage correct

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /status command`
  - Files: `src/bot/handlers/status.rs`

---

- [x] 23. Command: /clear

  **What to do**:
  - Implement `/clear` command handler (General topic)
  - Find stale topic mappings (no activity, topic deleted)
  - For each stale mapping:
    - Stop associated managed instance
    - Delete topic mapping
  - Report cleanup results
  - Write integration tests

  **Output**:
  ```
  Cleanup Complete

  Cleared 3 stale mappings:
  - my-old-project
  - test-session
  - abandoned-topic
  ```

  **Must NOT do**:
  - Don't delete Telegram topics (they may already be gone)
  - Don't add automatic scheduling

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Cleanup with multiple resource types
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 6, 7, 10

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleClearCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_clear)'` passes
  - [ ] Identifies stale mappings
  - [ ] Cleans up resources correctly

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /clear command`
  - Files: `src/bot/handlers/clear.rs`

---

- [x] 24. Command: /help

  **What to do**:
  - Implement `/help` command handler
  - Context-aware: different help in General vs topic
  - Use teloxide's Command::descriptions() as base
  - Add examples for each command
  - Write tests

  **General topic help**:
  ```
  OpenCode Telegram Bot

  General Commands:
  /new <name> - Create new project
  /sessions - List all sessions
  /connect <name> - Connect to session
  /status - Orchestrator status
  /clear - Clean stale mappings
  /help - This help

  In a topic:
  /session - Show session info
  /link <path> - Link to directory
  /stream - Toggle streaming
  /disconnect - Disconnect session
  ```

  **Topic help** (subset of commands):
  Show only topic-relevant commands.

  **Must NOT do**:
  - Don't add interactive help

  **Recommended Agent Profile**:
  - **Category**: `quick`
    - Reason: Text formatting
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with other commands)
  - **Blocks**: Task 27
  - **Blocked By**: Task 7

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/bot/handlers/forum.ts (handleHelpCommand)

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(cmd_help)'` passes
  - [ ] Context-aware output
  - [ ] All commands documented

  **Commit**: YES (group with other commands)
  - Message: `feat(bot): add /help command`
  - Files: `src/bot/handlers/help.rs`

---

- [x] 25. Permission Inline Buttons

  **What to do**:
  - Create `src/bot/handlers/permissions.rs`
  - Handle permission events from StreamHandler
  - Create inline keyboard with Allow/Deny buttons
  - Send message with permission request details
  - Handle callback query
  - Send reply to OpenCode via client
  - Update message to show result
  - Write integration tests

  **Permission message format**:
  ```
  Permission Request

  OpenCode wants to:
  [Delete file: src/old_module.rs]

  [Allow] [Deny]
  ```

  **Callback data format**: `perm:{session_id}:{permission_id}:{allow|deny}`

  **Must NOT do**:
  - Don't add timeout auto-deny
  - Don't add permission levels

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Callback handling with teloxide
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 4 (with commands)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 7, 13

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/opencode/stream-handler.ts (permission handling)
  - teloxide callbacks: https://github.com/teloxide/teloxide/blob/master/crates/teloxide/examples/buttons.rs

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(permissions)'` passes
  - [ ] Inline buttons appear
  - [ ] Callback handled correctly
  - [ ] Permission response sent to OpenCode

  **Commit**: YES
  - Message: `feat(bot): add permission inline buttons`
  - Files: `src/bot/handlers/permissions.rs`

---

### Wave 5: API & Integration

- [x] 26. HTTP API Server

  **What to do**:
  - Create `src/api/mod.rs` module
  - Implement axum HTTP server
  - Implement endpoints:
    - `POST /api/register` - Register external instance
    - `POST /api/unregister` - Unregister instance
    - `GET /api/status/:path` - Check registration status
    - `GET /api/instances` - List all external instances
    - `GET /api/health` - Health check
  - Implement optional API key middleware
  - Add CORS support
  - Write integration tests

  **Registration payload**:
  ```json
  {
    "projectPath": "/path/to/project",
    "port": 4096,
    "sessionId": "ses_abc123"
  }
  ```

  **Must NOT do**:
  - Don't add OAuth/JWT
  - Don't add rate limiting (trusted localhost)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Standard REST API with axum
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES
  - **Parallel Group**: Wave 5 (with Task 27)
  - **Blocks**: Task 27
  - **Blocked By**: Tasks 2, 10

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/api-server.ts
  - axum docs: https://docs.rs/axum/latest/axum/

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(api)'` passes
  - [ ] All endpoints work
  - [ ] API key auth works when configured
  - [ ] CORS headers present

  **Commit**: YES
  - Message: `feat(api): add HTTP API server`
  - Files: `src/api/*.rs`

---

- [x] 27. Integration Layer

  **What to do**:
  - Create `src/integration.rs`
  - Wire all components together
  - Implement message routing (Telegram -> OpenCode)
  - Implement stream bridging (OpenCode -> Telegram)
  - Implement topic name auto-update (after first response)
  - Handle rate limiting for Telegram API
  - Implement external instance routing
  - Write integration tests

  **Message routing flow**:
  1. Receive Telegram message in topic
  2. Get topic mapping
  3. Determine instance type (managed/discovered/external)
  4. Get OpenCode client for instance
  5. Mark message as from Telegram (dedup)
  6. Send to OpenCode async
  7. Subscribe to SSE if streaming enabled
  8. Forward events to Telegram with throttling

  **Must NOT do**:
  - Don't implement auto-cleanup here
  - Don't add message queuing

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
    - Reason: Complex coordination of all components
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 5 (near end)
  - **Blocks**: Task 28
  - **Blocked By**: Tasks 6, 7, 10, 13, 15-26

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/integration.ts

  **Acceptance Criteria**:
  - [ ] `cargo nextest run -E 'test(integration)'` passes
  - [ ] Messages route correctly
  - [ ] Streaming works end-to-end
  - [ ] Topic names update after first message

  **Commit**: YES
  - Message: `feat: add integration layer`
  - Files: `src/integration.rs`

---

- [x] 28. Main Entry Point & Graceful Shutdown

  **What to do**:
  - Implement `src/main.rs`
  - Load configuration
  - Initialize tracing/logging
  - Initialize databases
  - Start API server (background task)
  - Start bot with polling
  - Implement Ctrl+C handler
  - Implement graceful shutdown:
    - Stop accepting new messages
    - Complete in-flight requests
    - Stop all managed instances
    - Close database connections
  - Add startup banner with version
  - Write integration tests

  **Startup sequence**:
  1. Load config
  2. Init tracing
  3. Init databases (run migrations)
  4. Create stores
  5. Create InstanceManager (recover from DB)
  6. Start API server
  7. Create bot & integration
  8. Start polling
  9. Wait for shutdown signal

  **Must NOT do**:
  - Don't add daemon mode
  - Don't add systemd integration

  **Recommended Agent Profile**:
  - **Category**: `unspecified-low`
    - Reason: Entry point wiring
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO
  - **Parallel Group**: Wave 5 (final)
  - **Blocks**: None (final task)
  - **Blocked By**: Task 27

  **References**:
  - Original: https://github.com/huynle/opencode-telegram/blob/main/src/index.ts

  **Acceptance Criteria**:
  - [ ] `cargo build --release` succeeds
  - [ ] Binary starts and connects to Telegram
  - [ ] Ctrl+C triggers graceful shutdown
  - [ ] All resources cleaned up on exit

  **Commit**: YES
  - Message: `feat: add main entry point with graceful shutdown`
  - Files: `src/main.rs`

---

## Commit Strategy

| After Task | Message | Verification |
|------------|---------|--------------|
| 1 | `feat: initialize oc-outpost project with Rust tooling` | `cargo build` |
| 2 | `feat(types): add core type definitions` | `cargo nextest run -E 'test(types)'` |
| 3 | `feat(config): add configuration module` | `cargo nextest run -E 'test(config)'` |
| 4 | `feat(db): add database schemas and migrations` | `cargo nextest run -E 'test(db)'` |
| 5 | `feat(orchestrator): add OrchestratorStore` | `cargo nextest run -E 'test(orchestrator::store)'` |
| 6 | `feat(forum): add TopicStore` | `cargo nextest run -E 'test(forum::store)'` |
| 7 | `feat(bot): set up teloxide bot framework` | `cargo nextest run -E 'test(bot)'` |
| 8 | `feat(orchestrator): add PortPool` | `cargo nextest run -E 'test(port_pool)'` |
| 9 | `feat(orchestrator): add OpenCodeInstance` | `cargo nextest run -E 'test(instance)'` |
| 10 | `feat(orchestrator): add InstanceManager` | `cargo nextest run -E 'test(manager)'` |
| 11 | `feat(opencode): add process discovery` | `cargo nextest run -E 'test(discovery)'` |
| 12 | `feat(opencode): add REST client` | `cargo nextest run -E 'test(client)'` |
| 13 | `feat(opencode): add SSE StreamHandler` | `cargo nextest run -E 'test(stream_handler)'` |
| 14 | `feat(telegram): add Markdown converter` | `cargo nextest run -E 'test(markdown)'` |
| 15-24 | `feat(bot): add commands` | `cargo nextest run -E 'test(cmd_)'` |
| 25 | `feat(bot): add permission inline buttons` | `cargo nextest run -E 'test(permissions)'` |
| 26 | `feat(api): add HTTP API server` | `cargo nextest run -E 'test(api)'` |
| 27 | `feat: add integration layer` | `cargo nextest run -E 'test(integration)'` |
| 28 | `feat: add main entry point with graceful shutdown` | `cargo build --release` |

---

## Success Criteria

### Verification Commands
```bash
# Build
cargo build --release  # Should succeed

# Lint
cargo clippy -- -D warnings  # No warnings

# Format
cargo fmt --check  # No changes needed

# Test
cargo nextest run  # All pass

# Run
./target/release/oc-outpost  # Starts and connects
```

### Final Checklist
- [ ] All 10 commands working
- [ ] SSE streaming shows real-time progress
- [ ] Permission buttons work
- [ ] Process discovery finds TUI sessions
- [ ] API server accepts registrations
- [ ] Graceful shutdown cleans up resources
- [ ] State persists across restarts
- [ ] Rate limiting prevents Telegram errors
- [ ] Tests cover >80% of core logic
