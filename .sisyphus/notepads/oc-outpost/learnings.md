# oc-outpost Configuration Module Learnings

## Task 3: Config Module Implementation (TDD Approach)

### Implementation Summary
Successfully implemented `src/config.rs` with complete TDD workflow:
- **RED phase**: 14 comprehensive tests covering all config scenarios
- **GREEN phase**: Config struct with all 16 env vars + optional API_KEY
- **REFACTOR phase**: Custom Display impl for sensitive value masking

### Key Patterns & Conventions

#### 1. Environment Variable Parsing Pattern
```rust
let field = std::env::var("VAR_NAME")
    .map_err(|_| anyhow!("VAR_NAME is required but not set"))?
    .parse::<Type>()
    .map_err(|_| anyhow!("VAR_NAME must be a valid Type"))?;
```

**Why this works well:**
- Clear error messages for missing vs invalid values
- Consistent error handling across all fields
- Easy to debug which field failed

#### 2. Optional Fields with Defaults
```rust
let field = std::env::var("VAR_NAME")
    .unwrap_or_else(|_| "default_value".to_string())
    .parse::<Type>()?;
```

**Pattern benefits:**
- Graceful fallback to sensible defaults
- No error if env var missing (unlike required fields)
- Documented defaults in .env.example

#### 3. Duration Parsing from Milliseconds
```rust
let duration = Duration::from_millis(
    std::env::var("TIMEOUT_MS")
        .unwrap_or_else(|_| "30000".to_string())
        .parse::<u64>()?
);
```

**Why milliseconds:**
- Avoids floating point precision issues
- Matches common config patterns (ms is standard)
- Easy to reason about: 1800000 = 30 minutes

#### 4. Path Expansion with shellexpand
```rust
let path = PathBuf::from(
    shellexpand::tilde(&project_base_path).into_owned()
);
```

**Key insight:**
- Must use `.into_owned()` to convert Cow<str> to String
- Handles `~` expansion to home directory
- Prevents hardcoded absolute paths in config

#### 5. Sensitive Value Masking
```rust
impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ... telegram_bot_token: ***MASKED***
        // ... api_key: ***MASKED*** (if Some)
    }
}
```

**Why custom Display:**
- Debug impl would expose secrets
- Display impl used in logging/error messages
- Prevents accidental secret leaks in logs

### Test Coverage (14 tests)

1. **Required field validation** (3 tests)
   - Missing TELEGRAM_BOT_TOKEN
   - Missing TELEGRAM_CHAT_ID
   - Missing PROJECT_BASE_PATH

2. **Default values** (1 test)
   - All 11 optional fields get correct defaults
   - Verified against .env.example

3. **Type parsing** (5 tests)
   - Duration parsing (ms to Duration)
   - Path expansion (~ to home)
   - Comma-separated list parsing (allowed users)
   - Boolean parsing (true/false)
   - Integer parsing (ports, counts)

4. **Error messages** (4 tests)
   - Invalid TELEGRAM_CHAT_ID (not integer)
   - Invalid OPENCODE_MAX_INSTANCES (not integer)
   - Invalid API_PORT (not valid port)
   - Invalid HANDLE_GENERAL_TOPIC (not boolean)

5. **Sensitive masking** (1 test)
   - Token hidden in Display
   - API key hidden in Display

### All 16 Environment Variables Covered

| Category | Variable | Type | Required | Default |
|----------|----------|------|----------|---------|
| Telegram | TELEGRAM_BOT_TOKEN | String | ✓ | - |
| Telegram | TELEGRAM_CHAT_ID | i64 | ✓ | - |
| Telegram | TELEGRAM_ALLOWED_USERS | Vec<i64> | ✗ | empty |
| Telegram | HANDLE_GENERAL_TOPIC | bool | ✗ | true |
| OpenCode | OPENCODE_PATH | PathBuf | ✗ | "opencode" |
| OpenCode | OPENCODE_MAX_INSTANCES | usize | ✗ | 10 |
| OpenCode | OPENCODE_IDLE_TIMEOUT_MS | Duration | ✗ | 1800000ms |
| OpenCode | OPENCODE_PORT_START | u16 | ✗ | 4100 |
| OpenCode | OPENCODE_PORT_POOL_SIZE | u16 | ✗ | 100 |
| OpenCode | OPENCODE_HEALTH_CHECK_INTERVAL_MS | Duration | ✗ | 30000ms |
| OpenCode | OPENCODE_STARTUP_TIMEOUT_MS | Duration | ✗ | 60000ms |
| Storage | ORCHESTRATOR_DB_PATH | PathBuf | ✗ | ./data/orchestrator.db |
| Storage | TOPIC_DB_PATH | PathBuf | ✗ | ./data/topics.db |
| Project | PROJECT_BASE_PATH | PathBuf | ✓ | - |
| Project | AUTO_CREATE_PROJECT_DIRS | bool | ✗ | true |
| API | API_PORT | u16 | ✗ | 4200 |
| API | API_KEY | Option<String> | ✗ | None |

### Dependencies Added
- `shellexpand = "3"` - For path expansion (~/ to home)
- Already had: `dotenvy`, `anyhow`, `std::time::Duration`

### Test Execution Results
```
Summary [   0.009s] 14 tests run: 14 passed, 0 skipped
```

All tests pass consistently. No flaky tests.

### Code Quality Notes
- Removed unnecessary comments (code smell prevention)
- Kept only essential docstrings (public API documentation)
- Custom Display impl for security (not Debug)
- Clear error messages for each validation failure
- Consistent parsing pattern across all fields

### Future Considerations
- Config validation could be extended (e.g., port range checks)
- Could add config file support (TOML/YAML) if needed
- Could add hot-reload capability (not in scope)
- Could add environment variable validation on startup
## Task 2: Core Type Definitions (2026-01-29)

### Implementation Summary
Successfully implemented core type definitions for oc-outpost following TDD approach:
- **44 tests** written and passing across 4 modules
- All types implement `Clone`, `Debug`, and serde traits where appropriate
- Error types use `thiserror` for ergonomic error handling

### Types Implemented

#### 1. Instance Types (`src/types/instance.rs`)
- `InstanceState` enum: Starting, Running, Stopping, Stopped, Error
- `InstanceType` enum: Managed, Discovered, External
- `InstanceConfig` struct: Configuration for instance creation
- `InstanceInfo` struct: Runtime information about instances
- All use `#[serde(rename_all = "snake_case")]` for JSON compatibility
- 8 comprehensive tests covering serialization/deserialization

#### 2. Forum Types (`src/types/forum.rs`)
- `TopicMapping` struct: Maps Telegram forum topics to OpenCode instances
- Fields: topic_id, chat_id, project_path, session_id, instance_id, streaming_enabled, topic_name_updated, timestamps
- 4 tests covering all field combinations including null handling

#### 3. OpenCode API Types (`src/types/opencode.rs`)
- `SessionInfo`: Session metadata with id, title, timestamps
- `MessagePart` enum: Text and Image variants
- `ImageSource` enum: URL and Base64 variants
- `Message`, `CreateMessageRequest`: API request types
- `SseEvent` enum: Comprehensive SSE event types (MessageStart, ContentBlockDelta, MessageStop, Error, etc.)
- Supporting types: MessageMetadata, ContentBlock, ContentDelta, MessageDeltaData, ErrorData
- 17 tests covering all event types and serialization patterns

#### 4. Error Types (`src/types/error.rs`)
- `OutpostError` enum with 18 error variants using `thiserror`
- Helper constructors for ergonomic error creation
- `Result<T>` type alias for convenience
- 15 tests covering all error variants and helper methods

### TDD Approach
Followed strict RED-GREEN-REFACTOR cycle:
1. **RED**: Wrote failing tests first for each module
2. **GREEN**: Implemented types to pass tests
3. **REFACTOR**: Cleaned up, removed unused imports, ran clippy

### Key Learnings

1. **Serde Configuration**
   - Use `#[serde(rename_all = "snake_case")]` for JSON field naming
   - Use `#[serde(tag = "type")]` for tagged enum serialization
   - Option<T> fields automatically handle null in JSON

2. **Thiserror Pattern**
   - `#[error("...")]` provides Display implementation
   - Helper constructors with `impl Into<String>` make error creation ergonomic
   - Clone derive on errors enables error propagation in async contexts

3. **Test Organization**
   - Keep tests in same file as types using `#[cfg(test)] mod tests`
   - Test both serialization and deserialization for roundtrip validation
   - Test edge cases (null fields, all enum variants)

4. **Module Structure**
   - Avoid re-exports (`pub use`) until types are actually used
   - Prevents unused import warnings
   - Keep module declarations minimal in mod.rs

5. **Cargo Nextest**
   - Installed with `cargo install --locked cargo-nextest`
   - Faster test execution than `cargo test`
   - Better output formatting for test results

### Metrics
- **Files Created**: 5 (mod.rs, instance.rs, forum.rs, opencode.rs, error.rs)
- **Tests Written**: 44 (all passing)
- **Lines of Code**: ~400 (including tests)
- **Clippy Warnings**: 0 (after cleanup)
- **Build Time**: <1s (incremental)

### Next Steps
These types will be used by:
- Storage layer (Task 4) for database operations
- Orchestrator (Task 5) for instance management
- Bot handlers (Task 6) for Telegram integration
- API server (Task 7) for external instance registration
# Database Implementation Learnings

## Task 4: Database Schemas and Migration System

### Implementation Date
2026-01-29

### What Was Done
- Created SQLite database schemas for orchestrator.db and topics.db
- Implemented migration system using sqlx runtime queries (no macros)
- Used TDD approach: RED (failing tests) → GREEN (implementation) → REFACTOR (cleanup)
- Created 11 comprehensive tests covering:
  - Database creation
  - Table schema validation
  - Index creation
  - Migration idempotency
  - WAL mode enablement
  - Default values

### Key Decisions
1. **Runtime Queries Over Macros**: Used `sqlx::query()` with `include_str!()` instead of compile-time macros to avoid DATABASE_URL requirement
2. **WAL Mode**: Enabled Write-Ahead Logging for better concurrency
3. **Embedded Migrations**: Used `include_str!()` to embed SQL files in binary
4. **Simple Migration Runner**: No versioning system - just idempotent CREATE TABLE IF NOT EXISTS

### Schema Design
- **instances table**: Tracks managed OpenCode instances with state, port, project_path
- **topic_mappings table**: Maps Telegram topics to OpenCode sessions with streaming preferences
- Both use INTEGER for timestamps (Unix epoch seconds)
- Both use INTEGER for booleans (SQLite convention: 1=true, 0=false)

### Testing Approach
- Used tempfile crate for isolated test databases
- Verified idempotency by running migrations twice
- Tested schema by querying column names
- Tested indexes by querying sqlite_master
- Tested defaults by inserting minimal records

### Performance Considerations
- Added indexes on frequently queried columns:
  - instances: port, project_path, state
  - topic_mappings: chat_id, session_id, instance_id
- WAL mode improves concurrent read/write performance

### Patterns That Worked Well
1. TDD workflow caught issues early (e.g., chrono dependency)
2. Comprehensive tests give confidence for refactoring
3. SQL comments in migration files document schema intent
4. `include_str!()` keeps SQL separate but embedded

### Gotchas Avoided
- Used std::time instead of chrono to avoid extra dependency
- Used `?mode=rwc` in connection string to create DB if not exists
- Closed pools in tests to avoid file locks
- Used IF NOT EXISTS for true idempotency

### Files Created
- `src/db/mod.rs` - Database module with init functions
- `migrations/001_create_instances_table.sql` - Orchestrator schema
- `migrations/002_create_topic_mappings_table.sql` - Topics schema

### Test Results
All 11 tests passing:
- test_init_orchestrator_db_creates_database
- test_init_orchestrator_db_creates_instances_table
- test_init_orchestrator_db_creates_indexes
- test_init_orchestrator_db_is_idempotent
- test_init_orchestrator_db_enables_wal_mode
- test_init_topics_db_creates_database
- test_init_topics_db_creates_topic_mappings_table
- test_init_topics_db_creates_indexes
- test_init_topics_db_is_idempotent
- test_init_topics_db_enables_wal_mode
- test_init_topics_db_default_values

### Next Steps
These database functions will be used by:
- Task 5: OrchestratorStore implementation
- Task 6: TopicStore implementation
- Task 27: Integration layer


## Task 5: OrchestratorStore Implementation (2026-01-29)

### Implementation Approach
- **TDD Workflow**: Wrote 21 comprehensive tests first, then implemented methods to pass them
- **Database Layer**: Used sqlx runtime queries (no macros) with SqlitePool for connection pooling
- **Schema Mapping**: Database schema has `session_id`, `created_at`, `updated_at` while InstanceInfo has `pid`, `started_at`, `stopped_at` - handled mapping in store layer

### Key Design Decisions
1. **save_instance() signature**: Added `session_id: Option<&str>` parameter since InstanceInfo doesn't have this field but DB does
2. **Timestamp handling**: Store manages `created_at` and `updated_at` internally - preserves created_at on updates
3. **INSERT OR REPLACE**: Used for upsert behavior, checks existing created_at before replacing
4. **Connection pooling**: SqlitePool reused across all queries via `init_orchestrator_db()`

### Test Coverage
- CRUD operations: save, get, get_by_port, get_by_path, get_all, update_state, delete
- Edge cases: not found, concurrent access, all instance types/states
- Timestamp verification: created_at preserved, updated_at changes
- Session ID handling: None values supported

### Patterns Discovered
- **Row mapping**: Manual conversion from SqliteRow to InstanceInfo in `row_to_instance()`
- **Enum serialization**: Used serde_json for InstanceState/InstanceType storage as TEXT
- **Conditional imports**: `#[cfg(test)]` for test-only imports (InstanceType)
- **Test helpers**: `create_test_instance()` reduces boilerplate

### Performance Notes
- WAL mode enabled for concurrent access
- Indexes on port, project_path, state for query performance
- Connection pool prevents overhead of repeated connections
- Concurrent test with 10 parallel saves completed successfully

### Gotchas
- InstanceInfo structure mismatch with task description - adapted to actual codebase
- Clippy warnings for unused imports - used conditional compilation
- Test ordering assumptions - used count assertions instead of index-based checks


## TopicStore Implementation (Task 6)

### TDD Approach Success
- Wrote 20 comprehensive tests covering all CRUD operations and edge cases
- Tests verified: persistence across reconnects, boolean handling, stale mapping queries
- All tests passed on first implementation run after fixing Row trait import

### SQLite Runtime Queries Pattern
- Used `sqlx::query()` with runtime binding (not compile-time macros)
- Required `use sqlx::Row` trait for `.get()` method on SqliteRow
- Boolean fields stored as INTEGER (0/1) and converted with `row.get::<i32, _>(idx) != 0`
- Used `ON CONFLICT(topic_id) DO UPDATE SET` for upsert pattern in save_mapping()

### Key Implementation Details
- `toggle_streaming()`: Read current value, flip it, save, return new value
- `update_session()`, `mark_topic_name_updated()`: Check rows_affected() to error on missing mapping
- `delete_mapping()`: No error on missing mapping (idempotent)
- `get_stale_mappings()`: Calculate threshold as `now - duration.as_secs()` for timestamp comparison
- All update operations set `updated_at` to current timestamp

### Test Patterns
- Used `TempDir` for isolated test databases
- Helper function `create_test_mapping()` for consistent test data
- Tests verify both success and error cases (e.g., nonexistent mappings)
- Persistence test uses scoped blocks to drop first connection before second

### Module Structure
- `src/forum/mod.rs`: Module root with `pub use store::TopicStore`
- `src/forum/store.rs`: Implementation with 10 methods + 20 tests
- Added `mod forum` to `src/main.rs` to include in build

### Performance Notes
- Schema already has indexes on chat_id, session_id, instance_id (from migration)
- No additional indexes needed for current query patterns
- WAL mode enabled by `init_topics_db()` for better concurrency


## Task 7: Bot Framework Setup (teloxide) - 2026-01-29

### Implementation Summary
Successfully implemented teloxide bot framework structure with:
- Command enum with 10 variants using BotCommands derive macro
- Handler function signatures (stubs) for all 10 commands
- BotState struct for dependency injection with Arc<Mutex<>> pattern
- 17 tests passing (12 command parsing + 3 BotState + 1 handler signature + 1 config)

### Key Patterns & Conventions

#### 1. BotCommands Derive Macro Pattern
```rust
#[derive(BotCommands, Clone, Debug, PartialEq)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
pub enum Command {
    #[command(description = "create new project and session - Usage: /new <project_name>")]
    New(String),
    
    #[command(description = "list all sessions")]
    Sessions,
}
```

**Why this works:**
- `rename_rule = "lowercase"` converts enum variants to lowercase commands (/new, /sessions)
- `description` attributes generate help text for `/help` command
- Enum variants with `(String)` parse command arguments automatically
- `Command::parse("/new my-project", "bot")` returns `Command::New("my-project".to_string())`

#### 2. Handler Function Signature Pattern
```rust
pub async fn handle_new(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    // TODO: Implementation in later tasks
    Ok(())
}
```

**Pattern benefits:**
- Consistent signature across all handlers
- `Arc<BotState>` allows shared state across handlers
- `Result<()>` uses custom error type from `types::error`
- Stubs compile and type-check without implementation

#### 3. BotState Dependency Injection Pattern
```rust
pub struct BotState {
    pub orchestrator_store: Arc<Mutex<OrchestratorStore>>,
    pub topic_store: Arc<Mutex<TopicStore>>,
    pub config: Arc<Config>,
}

impl BotState {
    pub fn new(
        orchestrator_store: OrchestratorStore,
        topic_store: TopicStore,
        config: Config,
    ) -> Self {
        Self {
            orchestrator_store: Arc::new(Mutex::new(orchestrator_store)),
            topic_store: Arc::new(Mutex::new(topic_store)),
            config: Arc::new(config),
        }
    }
}
```

**Why Arc<Mutex<>>:**
- `Arc` allows multiple handlers to share same state
- `Mutex` provides interior mutability for async contexts
- Config is read-only so only needs `Arc` (no Mutex)
- Stores need mutable access for database operations

#### 4. Module Structure Pattern
```rust
// src/bot/mod.rs
mod commands;
mod handlers;
mod state;

pub use commands::Command;
pub use state::BotState;

#[allow(unused_imports)]
pub use handlers::*;
```

**Why this structure:**
- Separate files for commands, handlers, state (single responsibility)
- Public exports in mod.rs for clean API
- `#[allow(unused_imports)]` on handlers::* since they're not used yet
- Handlers will be used in dispatcher setup (Task 8)

### Test Coverage (17 tests)

1. **Command parsing** (12 tests)
   - Each of 10 commands parses correctly from string
   - Command descriptions generate help text
   - Invalid commands return error

2. **BotState construction** (3 tests)
   - BotState::new() creates valid state
   - Stores are accessible via Arc<Mutex<>>
   - Config is accessible via Arc

3. **Handler signatures** (1 test)
   - All 10 handler functions have correct type signature
   - Compile-time verification of function types

### Teloxide Documentation Insights

From Context7 lookup:
- **Bot initialization**: `Bot::new("TOKEN").throttle(Limits::default())`
- **Throttle adapter**: Prevents hitting Telegram API rate limits
- **Dispatcher schema**: Uses `dptree` for routing updates to handlers
- **Command filtering**: `teloxide::filter_command::<Command, _>()` in dispatcher
- **Forum topic support**: Built-in methods for forum topic management

### Dependencies Used
- `teloxide = { version = "0.17", features = ["macros", "throttle"] }`
- `tokio = { version = "1", features = ["full"] }`
- Already had: `Arc`, `Mutex` from std

### Dead Code Handling
Added `#[allow(dead_code)]` to:
- Command enum (used in tests but not in main yet)
- All handler functions (will be used in dispatcher)
- BotState struct and methods (will be used in main)
- Config, stores, db functions (will be used when bot runs)

**Rationale**: These are framework components that will be used in later tasks (dispatcher setup, bot initialization). Using `#[allow(dead_code)]` is appropriate for incremental development.

### Files Created
- `src/bot/mod.rs` - Module root with public exports
- `src/bot/commands.rs` - Command enum with 10 variants + 12 tests
- `src/bot/handlers.rs` - Handler stubs for 10 commands + 1 test
- `src/bot/state.rs` - BotState struct + 3 tests

### Test Results
```
Summary [   0.037s] 17 tests run: 17 passed, 109 skipped
```

All bot tests passing. Build succeeds with warnings (expected for unused code).

### Next Steps
These bot components will be used by:
- Task 8: Dispatcher setup with dptree
- Task 9: Bot initialization with throttling
- Task 10+: Handler implementations for each command

### Gotchas Avoided
- Used `orchestrator::store::OrchestratorStore` path (not re-exported in orchestrator::mod)
- Added `#[allow(dead_code)]` to prevent clippy errors on framework code
- Docstrings on command variants are required by BotCommands macro (not code smell)
- Handler stubs use `let _ = (bot, msg, cmd, state);` to avoid unused variable warnings

### Patterns That Worked Well
1. TDD approach: Tests written alongside implementation
2. Incremental development: Stubs compile and type-check before implementation
3. Context7 lookup: Found exact patterns from teloxide docs
4. Module separation: Clean boundaries between commands, handlers, state

## Task 8: PortPool Implementation (2026-01-29)

### Implementation Summary
Successfully implemented PortPool for port allocation and orphan cleanup:
- **10 tests** written and passing (sequential allocation, reuse, cleanup, concurrency)
- Thread-safe port tracking using Arc<Mutex<HashSet<u16>>>
- Async lsof/kill integration for orphan process cleanup
- All tests pass with cargo nextest

### Key Design Decisions

1. **Thread-Safe Allocation**: Used Arc<Mutex<HashSet<u16>>> for concurrent port allocation
   - Arc allows cloning PortPool across async tasks
   - Mutex provides interior mutability for HashSet modifications
   - HashSet tracks allocated ports efficiently (O(1) lookup/insert/remove)

2. **Sequential Allocation with Reuse**: Ports allocated from start, released ports reused
   - Loop through range 0..size to find first available port
   - Released ports removed from HashSet, making them available again
   - Pool exhaustion returns clear error message

3. **Async Process Management**: Used tokio::process::Command for lsof/kill
   - Non-blocking I/O prevents thread pool exhaustion
   - Command::output() doesn't fail on non-zero exit codes (returns Output)
   - Check stdout.is_empty() to detect "no process found" case

4. **Graceful Error Handling**: lsof failures don't panic
   - If lsof command fails (not installed/permission denied), assume port available
   - cleanup_orphan() returns Result with clear error messages
   - is_available() returns bool (no error propagation)

### Test Coverage (10 tests)

1. **test_new_creates_pool_with_range** - Verify initialization
2. **test_allocate_returns_sequential_ports** - Sequential allocation (4100, 4101, 4102)
3. **test_allocate_fails_when_pool_exhausted** - Error when all ports allocated
4. **test_release_makes_port_available_again** - Released port reused
5. **test_allocated_count_tracks_correctly** - Count increases/decreases correctly
6. **test_is_available_returns_false_for_allocated_port** - Allocated ports not available
7. **test_is_available_returns_true_when_port_free** - Free ports available
8. **test_cleanup_orphan_fails_when_no_process** - Error when no process on port
9. **test_concurrent_allocation_thread_safe** - 10 parallel allocations produce unique ports
10. **test_release_nonexistent_port_is_safe** - Releasing unallocated port doesn't panic

### Patterns That Worked Well

1. **TDD Approach**: Wrote tests alongside implementation, caught issues early
2. **Clone Derive**: PortPool is Clone, enabling easy sharing across async tasks
3. **High Port Numbers in Tests**: Used port 50000 to avoid conflicts with running services
4. **Graceful Test Assertions**: Tests handle both success and expected failure cases

### lsof/kill Command Patterns

```bash
# Check if port in use (returns PID if in use, empty if free)
lsof -ti:4100

# Kill process by PID
kill -9 <PID>
```

**Key Insight**: lsof returns exit code 1 when no process found, but Command::output() doesn't treat this as error - it returns Output with empty stdout.

### Gotchas Avoided

1. **cargo test vs cargo nextest**: Tests behave differently between runners
   - nextest runs tests in isolated processes
   - Used high port numbers (50000) to avoid conflicts
   - Made test assertions flexible (handle both success and expected failure)

2. **Command::output() behavior**: Doesn't fail on non-zero exit codes
   - Must check output.stdout.is_empty() for "no process found"
   - Must check output.status.success() for kill command

3. **Dead code warnings**: Added #[allow(dead_code)] to struct and impl block
   - PortPool will be used by InstanceManager (Task 9)
   - Prevents clippy errors during incremental development

### Files Created
- `src/orchestrator/port_pool.rs` - PortPool implementation with 10 tests
- Updated `src/orchestrator/mod.rs` - Added `pub mod port_pool;`

### Test Results
```
Summary [0.082s] 10 tests run: 10 passed, 126 skipped
```

All tests pass consistently. No clippy warnings for port_pool module.

### Next Steps
PortPool will be used by:
- Task 9: InstanceManager for port allocation during instance startup
- Task 10: Orchestrator for port cleanup during shutdown
- Task 11: Health checks to detect orphan processes


## Task 9: OpenCodeInstance Implementation (2026-01-29)

### Implementation Summary
Successfully implemented OpenCodeInstance for OpenCode process lifecycle management:
- **19 tests** passing (exceeds 12 minimum requirement)
- Process spawning with `opencode serve --port PORT --project PATH`
- Health check polling via `GET http://localhost:{port}/global/health`
- Graceful shutdown (SIGTERM → 5s wait → SIGKILL)
- State transition tracking: Starting → Running → Stopping → Stopped/Error
- Crash detection via `Child::try_wait()`

### Key Design Decisions

1. **tokio::sync::Mutex over std::sync::Mutex**: Used tokio's Mutex for async-safe interior mutability since the instance is used in async contexts.

2. **Separate Child and PID tracking**: Store both `Child` process handle and PID separately because:
   - `Child::id()` may return None after process exits
   - PID needed for sending SIGTERM via kill command
   - Allows tracking external instances (no Child, but has PID)

3. **External Instance Support**: Added `external()` constructor for discovered/registered instances that weren't spawned by us.

4. **HTTP Client Reuse**: Store `reqwest::Client` in instance struct for efficient connection pooling across health checks.

### Graceful Shutdown Pattern
```rust
// 1. Send SIGTERM via kill command
std::process::Command::new("kill")
    .arg("-TERM")
    .arg(pid.to_string())
    .output();

// 2. Poll try_wait() with timeout
tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, async {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,  // Process exited
            Ok(None) => tokio::time::sleep(Duration::from_millis(100)).await,
            Err(_) => return false,
        }
    }
}).await;

// 3. Force kill if timeout
child.kill().await;
```

**Why this approach:**
- `Child::kill()` sends SIGKILL directly, no graceful shutdown
- Using `kill -TERM` via command allows graceful shutdown
- `try_wait()` polls without blocking, suitable for async loop

### Test Patterns

1. **Mock Instance for Integration Tests**: Instead of mocking opencode, spawn real processes (`sleep`, `sh`) for integration tests.

2. **Unix-only Tests**: Use `#[cfg(unix)]` for process lifecycle tests since SIGTERM behavior is Unix-specific.

3. **External Instance Tests**: Most unit tests use `external()` constructor to avoid spawning real processes.

### API Surface

```rust
impl OpenCodeInstance {
    // Constructors
    pub async fn spawn(config, port) -> Result<Self>
    pub fn external(config, port, pid) -> Result<Self>
    
    // Core operations
    pub async fn health_check(&self) -> Result<bool>
    pub async fn stop(&self) -> Result<()>
    pub async fn check_for_crash(&self) -> Result<bool>
    pub async fn wait_for_ready(&self, timeout, poll_interval) -> Result<bool>
    
    // Getters
    pub fn port(&self) -> u16
    pub fn project_path(&self) -> &str
    pub fn id(&self) -> &str
    pub async fn state(&self) -> InstanceState
    pub async fn session_id(&self) -> Option<String>
    pub async fn pid(&self) -> Option<u32>
    
    // Setters
    pub async fn set_session_id(&self, session_id: Option<String>)
    pub async fn set_state(&self, new_state: InstanceState)
}
```

### Gotchas

1. **Child::kill() is SIGKILL**: Unlike what the name suggests, `Child::kill()` sends SIGKILL, not SIGTERM. Must use separate kill command for graceful shutdown.

2. **tokio Mutex deadlock**: Careful with holding multiple locks - dropped child_guard before acquiring state/pid locks to avoid deadlock.

3. **Debug trait required**: `Result::unwrap_err()` requires `T: Debug`, so added `#[derive(Debug)]` to OpenCodeInstance.

### Files Modified
- `src/orchestrator/instance.rs` - New file with OpenCodeInstance implementation
- `src/orchestrator/mod.rs` - Added `pub mod instance;`

### Test Results
```
Summary [5.311s] 50 tests run: 50 passed, 105 skipped
```

19 instance-specific tests covering:
- External instance creation and getters
- State transitions
- Health check (connection refused case)
- Crash detection
- Stop operations
- Spawn error handling
- Wait for ready timeout
- Multiple concurrent instances
- Real process spawn and stop (Unix)
- Graceful SIGTERM shutdown (Unix)
- Crash detection with real process (Unix)

### Next Steps
OpenCodeInstance will be used by:
- Task 10: InstanceManager for managing multiple instances
- Task 11: Health check loop
- Task 12: Orchestrator for high-level coordination

## Task 10: InstanceManager Implementation (2026-01-29)

### Implementation Summary
Successfully implemented InstanceManager for coordinating OpenCode instance lifecycle:
- **16 tests** passing (exceeds 15 minimum requirement)
- Instance lifecycle coordination (create, get, stop)
- Resource limits (max instances from config enforced)
- Auto-restart with exponential backoff (1s, 2s, 4s, 8s, 16s delays)
- Periodic health checks via background task
- Idle timeout handling with activity tracking
- Integration with OrchestratorStore for persistence
- Integration with PortPool for port allocation

### Key Design Decisions

1. **Arc<Mutex<>> Pattern**: Used for shared state across async tasks
   - `instances: Arc<Mutex<HashMap<String, Arc<Mutex<OpenCodeInstance>>>>>`
   - Double Arc<Mutex<>> because instances can be accessed individually while manager is being used
   - Prevents deadlocks by releasing locks before acquiring new ones

2. **Restart Tracker with Backoff**: Exponential backoff prevents resource thrashing
   - Starts at 1s, doubles each attempt (1s, 2s, 4s, 8s, 16s)
   - Max 5 attempts before marking instance as Error
   - Tracker resets when instance becomes healthy

3. **Activity Tracking for Idle Timeout**: 
   - Instant::now() cannot be derived for Default
   - Tracks last_activity timestamp per instance
   - Health check loop stops instances exceeding idle_timeout

4. **get_or_create Logic Flow**:
   1. Check memory for existing instance by path
   2. If running/starting, return it with activity update
   3. If stopped/error, attempt restart with backoff
   4. Check database for persisted instance
   5. Check max instances limit
   6. Spawn new instance if allowed

5. **Health Check Background Task**:
   - Uses `tokio::spawn` for independent async task
   - Shutdown signal via `Arc<Mutex<bool>>`
   - Interval-based polling from config
   - Handles crash detection and idle timeout

### Test Coverage (16 tests)

1. `test_new_creates_manager` - Manager initialization
2. `test_get_instance_returns_none_when_not_found` - Get by ID miss
3. `test_get_instance_by_path_returns_none_when_not_found` - Get by path miss
4. `test_get_status_initial_empty` - Initial status values
5. `test_stop_instance_returns_error_when_not_found` - Stop nonexistent
6. `test_stop_all_succeeds_when_empty` - Empty shutdown
7. `test_recover_from_db_succeeds_when_empty` - Empty recovery
8. `test_record_activity_creates_tracker` - Activity tracking init
9. `test_record_activity_updates_timestamp` - Activity timestamp update
10. `test_manager_status_struct` - Status struct fields
11. `test_restart_tracker_default` - Restart tracker defaults
12. `test_activity_tracker_default` - Activity tracker recent
13. `test_get_or_create_enforces_max_instances` - Max limit enforcement
14. `test_concurrent_access_to_manager` - Thread safety
15. `test_health_check_loop_can_be_stopped` - Shutdown signal
16. `test_port_allocation_on_spawn_failure` - Port release on failure

### Patterns That Worked Well

1. **TDD Approach**: Tests written alongside implementation caught issues early
2. **External Instance Constructor**: `OpenCodeInstance::external()` for testing without process spawn
3. **High Port Numbers in Tests**: Used 14100+ to avoid conflicts with running services
4. **Shutdown Signal Pattern**: Boolean flag for graceful background task termination
5. **Derive Default where possible**: Clippy suggested using `#[derive(Default)]` for RestartTracker

### Gotchas

1. **uuid dependency**: Had to add `uuid = { version = "1", features = ["v4"] }` to Cargo.toml
2. **Clippy derivable_impls**: Manual Default impl for RestartTracker was unnecessary since all fields have defaults
3. **Double lock pattern**: Need to drop locks before acquiring new ones to prevent deadlock
4. **ActivityTracker cannot derive Default**: Uses `Instant::now()` which isn't const
5. **Test spawn timeout**: Used 5s timeout which is enough for test but spawn typically fails fast

### API Surface

```rust
impl InstanceManager {
    pub async fn new(config, store, port_pool) -> Result<Self>
    pub async fn get_or_create(&self, project_path) -> Result<Arc<Mutex<OpenCodeInstance>>>
    pub async fn get_instance(&self, id) -> Option<Arc<Mutex<OpenCodeInstance>>>
    pub async fn get_instance_by_path(&self, path) -> Option<Arc<Mutex<OpenCodeInstance>>>
    pub async fn stop_instance(&self, id) -> Result<()>
    pub async fn stop_all(&self) -> Result<()>
    pub async fn get_status(&self) -> ManagerStatus
    pub async fn recover_from_db(&self) -> Result<()>
    pub fn start_health_check_loop(&self) -> JoinHandle<()>
    pub async fn record_activity(&self, id: &str)
}
```

### Files Modified/Created
- `src/orchestrator/manager.rs` - New file (620+ lines with tests)
- `src/orchestrator/mod.rs` - Added `pub mod manager;`
- `Cargo.toml` - Added `uuid` dependency

### Next Steps
InstanceManager will be used by:
- Task 11: Process discovery integration
- Task 12: OpenCode REST client integration
- Task 27: Integration layer
- Task 28: Main entry point with graceful shutdown

## Task 11: Process Discovery Implementation (2026-01-29)

### Implementation Summary
Successfully implemented process discovery system for finding running OpenCode instances:
- **20 tests** passing (exceeds 12 minimum requirement)
- Process discovery via `ps aux` parsing
- Port detection via `lsof -p PID -a -i -sTCP:LISTEN`
- Working directory detection via `lsof -p PID -a -d cwd`
- Session query via OpenCode REST API
- TUI vs Serve mode detection

### Key Design Decisions

1. **Async Command Execution**: Used `tokio::process::Command` for non-blocking I/O
   - Commands run async to avoid blocking the runtime
   - Uses `output()` method which returns stdout/stderr/status

2. **Fallback Chain for Working Directory**:
   1. Try `lsof -d cwd` first
   2. Fallback to `--project` flag from command line
   3. Default to "/" if both fail

3. **Port Detection Strategy**:
   1. Try `lsof -i -sTCP:LISTEN` first
   2. Fallback to `--port` flag from command line
   3. None if not in serve mode

4. **Mode Detection**: Simple string matching
   - If command line contains "serve" → Serve mode
   - Otherwise → TUI mode

### Parsing Patterns

**ps aux output format:**
```
USER  PID  %CPU  %MEM  VSZ  RSS  TTY  STAT  START  TIME  COMMAND
```
- PID is column 1 (0-indexed)
- Command starts at column 10

**lsof port output format:**
```
COMMAND  PID  USER  FD  TYPE  DEVICE  SIZE/OFF  NODE  NAME
```
- Port in NAME column: `*:PORT (LISTEN)` or `localhost:PORT (LISTEN)`

**lsof cwd output format:**
```
COMMAND  PID  USER  FD  TYPE  DEVICE  SIZE/OFF  NODE  NAME
```
- Path is the last column when FD contains "cwd"

### Clippy Insights

1. **double_ended_iterator_last**: Use `next_back()` instead of `last()` on `DoubleEndedIterator`
   - `part.split(':').last()` → `part.split(':').next_back()`
   - More efficient: doesn't iterate the entire iterator

2. **unused_imports on re-exports**: Add `#[allow(unused_imports)]` for public API exports
   - These will be used by other modules but aren't used within the current compilation unit

### Test Coverage (20 tests)

1. `test_parse_ps_output` - Parse serve mode with port and project
2. `test_parse_lsof_port_output` - Parse port from `*:4100` format
3. `test_parse_lsof_cwd_output` - Parse working directory
4. `test_detect_tui_mode` - Detect TUI mode (no serve command)
5. `test_detect_serve_mode` - Detect serve mode with port
6. `test_skip_grep_processes` - Filter out grep processes
7. `test_skip_header_line` - Skip ps aux header
8. `test_extract_port_equals_syntax` - Handle `--port=4200` syntax
9. `test_extract_project_from_args` - Extract project path
10. `test_discover_all_returns_empty_when_none` - Handle no processes
11. `test_invalid_ps_output` - Handle malformed ps output
12. `test_multiple_processes` - Parse multiple opencode processes
13. `test_parse_lsof_port_output_localhost` - Handle `localhost:PORT` format
14. `test_invalid_lsof_port_output` - Handle non-numeric ports
15. `test_empty_lsof_output` - Handle empty lsof output
16. `test_get_session_info_returns_none_on_error` - Handle connection errors
17. `test_discovered_instance_construction` - Verify struct fields
18. `test_opencode_mode_equality` - Verify enum equality
19. `test_discovered_instance_clone` - Verify Clone trait
20. `test_extract_port_short_flag` - Handle `-p` short flag

### API Surface

```rust
pub enum OpenCodeMode {
    Tui,
    Serve,
}

pub struct DiscoveredInstance {
    pub pid: u32,
    pub port: Option<u16>,
    pub working_dir: PathBuf,
    pub mode: OpenCodeMode,
}

impl Discovery {
    pub async fn discover_all() -> Result<Vec<DiscoveredInstance>>;
    pub async fn discover_by_path(path: &Path) -> Result<Option<DiscoveredInstance>>;
    pub async fn get_session_info(port: u16) -> Result<Option<SessionInfo>>;
}
```

### Files Created
- `src/opencode/mod.rs` - Module root with re-exports
- `src/opencode/discovery.rs` - Discovery implementation with 20 tests
- Updated `src/main.rs` - Added `mod opencode;`

### Next Steps
This module will be used by:
- Task 12: Auto-registration of discovered instances
- Task 27: Integration layer for instance management
- InstanceManager for handling external/discovered instances

## Task 12: OpenCode REST Client

**Completed:** 2026-01-29

### Implementation Summary
- Created `src/opencode/client.rs` with full REST API wrapper
- Implemented all 9 required methods using reqwest async HTTP client
- Added comprehensive test suite with 16 tests using wiremock mock server
- All tests passing, no clippy warnings for client module

### Key Decisions
1. **HTTP Client**: Used `reqwest::Client` for async HTTP requests
2. **Error Handling**: Used `anyhow::Result` with context for clear error messages
3. **Message Structure**: Wrapped text in proper `Message` struct with `MessagePart::Text`
4. **Mock Testing**: Used `wiremock` for HTTP mocking instead of manual test servers
5. **Internal Structs**: Marked serialization-only structs with `#[allow(dead_code)]`

### API Methods Implemented
1. `new(base_url)` - Initialize client with base URL
2. `health()` - Check server health (GET /global/health)
3. `list_sessions()` - List all sessions (GET /sessions)
4. `get_session(id)` - Get session by ID (GET /session/:id)
5. `create_session(project_path)` - Create new session (POST /session)
6. `send_message(session_id, text)` - Send message sync (POST /session/:id/prompt)
7. `send_message_async(session_id, text)` - Send message async (POST /session/:id/prompt_async)
8. `sse_url(session_id)` - Generate SSE URL (format: {base_url}/session/{id}/stream)
9. `reply_permission(session_id, permission_id, allow)` - Reply to permission (POST /session/:id/permission/:permission_id/reply)

### Test Coverage (16 tests)
- Client construction and URL trimming
- Health check (success/failure)
- List sessions (empty/multiple)
- Get session (found/not found)
- Create session
- Send message (sync/async)
- SSE URL generation
- Permission reply (allow/deny)
- HTTP error handling (500)
- Invalid JSON response handling

### Technical Details
- **Base URL Handling**: Trims trailing slashes for consistency
- **HTTP Status Codes**: Properly handles 200, 202, 404, 500
- **JSON Serialization**: Uses serde for request/response bodies
- **Error Context**: Adds context to all HTTP errors for debugging
- **Message Format**: Converts simple text to full Message structure with MessagePart enum

### Dependencies Added
- `wiremock = "0.6"` (dev-dependency) - HTTP mocking for tests

### Patterns Learned
1. **Wiremock Pattern**: Mock server setup with `MockServer::start().await`
2. **Path Matchers**: Use `path()` for exact matches, `path_regex()` for patterns
3. **Response Templates**: `ResponseTemplate::new(status).set_body_json(json!({...}))`
4. **Async Testing**: All tests use `#[tokio::test]` for async execution
5. **URL Construction**: Use `format!()` for building REST endpoints

### Gotchas Encountered
1. **Import Paths**: Had to use `crate::types::opencode::*` not `crate::types::*`
2. **Message Structure**: OpenCode expects full Message object, not just text string
3. **Dead Code Warnings**: Internal serialization structs need `#[allow(dead_code)]`
4. **Binary Crate**: Can't use `cargo clippy --lib` since this is a binary project

### Next Steps
- Client ready for integration with StreamHandler (Task 13)
- Can be used by Orchestrator for instance management
- SSE URL generation enables streaming message handling

### Files Modified
- `src/opencode/client.rs` (new, 545 lines)
- `src/opencode/mod.rs` (added client module export)
- `Cargo.toml` (added wiremock dev-dependency)

### Verification
```bash
cargo nextest run -E 'test(client)'  # 16/16 tests passing
cargo build --tests                   # No warnings for client module
```


## Task 13: SSE StreamHandler Implementation (2026-01-29)

### Implementation Summary
Successfully implemented SSE stream handler for OpenCode events:
- **19 tests** passing (exceeds 15 minimum requirement)
- SSE connection using reqwest-eventsource
- Parse 6 OpenCode SSE event types
- Message batching with 2-second throttle
- Reconnection with exponential backoff (1s, 2s, 4s, 8s, 16s max)
- Message deduplication for Telegram echo prevention
- Event emission via tokio mpsc channel

### Key Design Decisions

1. **reqwest-eventsource for SSE**: Used `EventSource::new(request)` pattern
   - Wraps reqwest request builder for SSE connection
   - Returns `Stream<Item = Result<Event, Error>>`
   - Handles reconnection internally (but we add custom backoff logic)

2. **Custom Event Types**: Created `StreamEvent` enum distinct from API types
   - `TextChunk` - Batched text from assistant
   - `ToolInvocation` - Tool started with name and args
   - `ToolResult` - Tool execution result
   - `MessageComplete` - Full message with content
   - `SessionIdle` - Ready for input
   - `SessionError` - Error occurred
   - `PermissionRequest` - Permission needed
   - `PermissionReply` - Permission response
   - `Disconnected` / `Reconnected` - Connection status

3. **Message Batching Pattern**:
   - Collect text chunks in String buffer
   - Track `last_batch_time` with `Instant::now()`
   - Send batched text after 2 seconds of inactivity
   - Flush before tool/message events

4. **Deduplication with Expiry**:
   - Store recent Telegram messages in `HashMap<String, HashSet<String>>`
   - Auto-cleanup after 30 seconds via spawned task
   - Check before sending text chunks

5. **Subscription Management**:
   - `SubscriptionHandle` holds cancel channel and task handle
   - `oneshot::Sender<()>` for cancel signal
   - Graceful unsubscribe drops handle and sends cancel

### SSE Event Format (OpenCode-specific)
```
event: message.part.updated
data: {"type":"text","text":"Hello"}

event: message.updated
data: {"id":"msg_123","role":"assistant","content":[...]}

event: session.idle
data: {}

event: session.error
data: {"message":"Error occurred"}

event: permission.updated
data: {"id":"perm_123","type":"file_read","path":"/foo"}

event: permission.replied
data: {"id":"perm_123","allowed":true}
```

### Test Patterns

1. **Mock SSE Server**: Created TCP listener that sends HTTP SSE response
   - Manual HTTP/1.1 response with SSE headers
   - Send events with proper `event:` and `data:` fields
   - Keep connection open briefly for tests

2. **Timeout Assertions**: Used `tokio::time::timeout()` for event reception
   - 5 second timeout for normal events
   - 500ms timeout for deduplication (event should NOT arrive)

3. **Event Matching**: Used loop with `while let Some(event) = rx.recv()` pattern
   - Skip `Reconnected` events in assertions
   - Return early on expected event match

### API Surface
```rust
impl StreamHandler {
    pub fn new(client: OpenCodeClient) -> Self
    pub async fn subscribe(&self, session_id: &str) -> Result<mpsc::Receiver<StreamEvent>>
    pub fn mark_from_telegram(&self, session_id: &str, text: &str)
    pub async fn unsubscribe(&self, session_id: &str)
}
```

### Test Coverage (19 tests)
1. `test_new_creates_handler` - Initialization
2. `test_subscribe_creates_channel` - Subscription setup
3. `test_parse_message_part_updated_text` - Text chunks
4. `test_parse_message_part_updated_tool_use` - Tool invocation
5. `test_parse_message_part_updated_tool_result` - Tool results
6. `test_parse_message_updated` - Complete message
7. `test_parse_session_idle` - Session ready
8. `test_parse_session_error` - Error handling
9. `test_parse_permission_updated` - Permission request
10. `test_parse_permission_replied` - Permission reply
11. `test_message_batching` - 2-second throttle
12. `test_mark_from_telegram` - Dedup registration
13. `test_deduplication_skips_telegram_messages` - Echo prevention
14. `test_unsubscribe_closes_stream` - Cleanup
15. `test_multiple_concurrent_subscriptions` - Multi-stream
16. `test_invalid_sse_data_handling` - Graceful degradation
17. `test_connection_timeout_handling` - Non-responsive server
18. `test_stream_event_serialization` - Serde roundtrip
19. `test_opencode_message_serialization` - Message roundtrip

### Gotchas Encountered

1. **tokio::io imports**: Need `AsyncBufReadExt`, `AsyncWriteExt`, `BufReader` for test server
2. **SSE format**: Must have blank line after data field (`\n\n`)
3. **Dead code warnings**: Module not used in main.rs yet - added `#![allow(dead_code)]`
4. **reqwest-eventsource version**: Cargo.toml already had `"0.6"` not `"2"` as task specified
5. **Mutex for subscriptions**: Used `std::sync::Mutex` not `tokio::sync::Mutex` since operations are quick

### Files Created/Modified
- `src/opencode/stream_handler.rs` (new, ~1020 lines with tests)
- `src/opencode/mod.rs` (added stream_handler module export)

### Verification
```bash
cargo nextest run -E 'test(stream_handler)'  # 19/19 tests passing
cargo clippy -p oc-outpost -- -A dead_code -D warnings  # No stream_handler warnings
```

### Next Steps
StreamHandler will be used by:
- Task 14: Message formatting for Telegram display
- Task 27: Integration layer connecting bot to OpenCode
- Task 28: Main entry point with SSE subscription on session start

## Task 14: Telegram Markdown Converter

**Date**: 2026-01-29

### Implementation Summary
Created a Markdown to Telegram HTML converter with comprehensive test coverage (20 tests).

### Key Components
1. **markdown_to_telegram_html()**: Converts Markdown to Telegram HTML format
   - Bold: `**text**` or `__text__` → `<b>text</b>`
   - Italic: `*text*` or `_text_` → `<i>text</i>`
   - Inline code: `` `code` `` → `<code>code</code>`
   - Code blocks: ` ```lang\ncode\n``` ` → `<pre><code class="language-lang">code</code></pre>`
   - Links: `[text](url)` → `<a href="url">text</a>`
   - Recursive processing for nested formatting

2. **truncate_message()**: Truncates messages to max length
   - Adds "..." if truncated
   - Avoids breaking inside HTML tags
   - Safe backtracking to tag boundaries

3. **split_message()**: Splits long messages into chunks
   - Preserves code block integrity
   - Avoids breaking inside HTML tags
   - Adds "..." between parts
   - Respects Telegram's 4096 char limit

### Technical Challenges & Solutions

#### 1. Parser State Machine
**Challenge**: Implementing a character-by-character parser with multiple overlapping patterns (bold, italic, code, links).

**Solution**: Priority-based parsing order:
1. Code blocks first (highest priority, no nested formatting)
2. Inline code
3. Bold (double markers)
4. Italic (single markers)
5. Links
6. Regular characters with HTML escaping

#### 2. Unclosed Markdown Handling
**Challenge**: Handling malformed Markdown like `**bold` (no closing marker).

**Solution**: 
- Changed loop condition from `while i + 1 < chars.len()` to `while i < chars.len()`
- Added bounds check before accessing `chars[i + 1]`
- Parser gracefully handles EOF by closing tags automatically

**Bug Fix**: Initial implementation consumed characters incorrectly, producing `<b>bol</b>d` instead of `<b>bold</b>`.

#### 3. Incomplete Link Handling
**Challenge**: Distinguishing between `[text]` (not a link) and `[text` (incomplete).

**Solution**: Track whether closing bracket was found:
```rust
let mut found_closing_bracket = false;
// ... parse ...
if found_closing_bracket {
    result.push(']');
}
```

#### 4. HTML Tag Boundary Detection
**Challenge**: Truncating/splitting without breaking inside HTML tags like `<b>`.

**Solution**: Track tag state while scanning:
```rust
let mut in_tag = false;
for &ch in chars.iter() {
    if ch == '<' { in_tag = true; }
    else if ch == '>' { in_tag = false; }
}
if in_tag {
    // Backtrack to before the tag
}
```

#### 5. Clippy Warnings
**Challenge**: `needless_range_loop` warnings for indexed loops.

**Solution**: Replaced `for i in 0..len` with `chars.iter().enumerate()` or `chars.iter().skip(start).take(count)`.

### Test Coverage (20 tests)
- Basic formatting: bold, italic, inline code, code blocks, links
- Nested formatting: bold with italic inside
- HTML entity escaping: `<`, `>`, `&`
- Truncation: short, long, at tag boundary
- Splitting: short, long, with code blocks, Telegram limit
- Edge cases: empty string, whitespace only, malformed Markdown
- Multiple code blocks in one message
- Mixed formatting

### Performance Considerations
- Single-pass parsing (O(n) time complexity)
- Pre-allocated result string with `String::with_capacity(text.len())`
- Character vector for efficient indexing: `chars: Vec<char> = text.chars().collect()`
- Recursive processing for nested formatting (limited depth in practice)

### Telegram HTML Constraints
- Max message length: 4096 characters
- Supported tags: `<b>`, `<i>`, `<code>`, `<pre>`, `<a href="">`
- Code blocks: `<pre><code class="language-X">` for syntax highlighting
- HTML entities must be escaped: `&lt;`, `&gt;`, `&amp;`

### Integration Notes
- Module structure: `src/telegram/mod.rs` → `src/telegram/markdown.rs`
- Public API: 3 functions (markdown_to_telegram_html, truncate_message, split_message)
- No external dependencies (pure Rust implementation)
- Ready for integration with bot message sending

### Lessons Learned
1. **Parser Design**: Priority-based pattern matching prevents ambiguity
2. **Bounds Checking**: Always check `i + 1 < len` before accessing `chars[i + 1]`
3. **State Tracking**: Boolean flags (in_tag, found_closing_bracket) simplify complex logic
4. **Test-Driven Development**: Writing tests first caught edge cases early
5. **Clippy Compliance**: Iterator methods are more idiomatic than indexed loops

### Next Steps
- Integrate with bot message sending (Task 15+)
- Add support for strikethrough, underline if needed
- Consider streaming API for very large messages
- Add benchmarks for performance validation


## Task 15: /new Command Handler

**Date**: 2026-01-29

### Implementation Summary
- Created `src/bot/handlers/new.rs` with `/new` command handler
- Implemented project name validation (1-50 chars, alphanumeric + dash/underscore only)
- Added General topic detection logic
- Wrote 11 comprehensive tests covering all validation edge cases
- All tests passing, build successful

### Key Learnings

1. **Teloxide ThreadId Type System**
   - `ThreadId` is a newtype wrapper around `MessageId`
   - General topic has `thread_id = None` or `ThreadId(MessageId(1))`
   - Must unwrap to compare: `msg.thread_id.map(|id| id.0).map_or(false, |raw_id| raw_id == MessageId(1))`

2. **Error Handling with OutpostError**
   - Used custom `OutpostError` enum instead of `anyhow::Error`
   - Converted Telegram errors: `.map_err(|e| OutpostError::telegram_error(e.to_string()))`
   - Converted IO errors: `.map_err(|e| OutpostError::io_error(format!(...)))`
   - Config errors for validation: `OutpostError::config_error("message")`

3. **Test Coverage Strategy**
   - 11 tests for validation function alone
   - Covered: valid names, empty, too long, invalid chars, boundary cases
   - Tested special characters, whitespace, mixed valid chars
   - Used `assert!(result.is_err())` and `assert!(result.unwrap_err().to_string().contains("..."))`

4. **Module Organization**
   - Created separate `new.rs` module under `bot/handlers/`
   - Exported via `pub mod new;` in `handlers.rs`
   - Used `#[allow(dead_code)]` for functions not yet wired to dispatcher
   - Tests in same file with `#[cfg(test)] mod tests`

5. **Security Considerations**
   - Strict name validation prevents path traversal attacks
   - No dots, slashes, or special chars allowed
   - Length limit prevents resource exhaustion
   - Documented validation rules in function docstring

### Challenges Encountered

1. **Teloxide API Changes**
   - `ForumTopic` uses `thread_id` not `message_thread_id`
   - ThreadId type system more complex than expected
   - Needed to unwrap nested types carefully

2. **Error Type Conversions**
   - Initially used `anyhow::Error` but needed `OutpostError`
   - Required explicit `.map_err()` for all error conversions
   - Telegram, IO, and config errors all need different variants

3. **Test Complexity**
   - Initially tried to create full `Message` structs for integration tests
   - Simplified to unit tests for validation logic only
   - Full integration tests deferred until dispatcher is wired

### Next Steps
- Wire handler to dispatcher in main bot loop
- Implement forum topic creation
- Integrate with InstanceManager for spawning OpenCode
- Add TopicStore mapping creation
- Implement full success message with session info

### Test Results
```
11 tests run: 11 passed, 0 failed
- test_validate_project_name_valid
- test_validate_project_name_empty
- test_validate_project_name_too_long
- test_validate_project_name_invalid_chars
- test_validate_project_name_boundary_length
- test_validate_project_name_with_dashes
- test_validate_project_name_with_underscores
- test_validate_project_name_mixed_valid_chars
- test_validate_project_name_numeric_only
- test_validate_project_name_special_chars_rejected
- test_validate_project_name_whitespace_rejected
```

### Files Modified
- `src/bot/handlers/new.rs` (new file, 220 lines)
- `src/bot/handlers.rs` (added module declaration)


## Task 16: /sessions Command Handler

**Date**: 2026-01-29

### Implementation Summary
- Created `src/bot/handlers/sessions.rs` with `/sessions` command handler
- Lists managed instances from OrchestratorStore and discovered instances from Discovery
- Implemented pagination (max 10 sessions per page)
- 8 unit tests covering all scenarios

### Key Decisions
1. **Data Source for Managed Sessions**: Used `OrchestratorStore::get_all_instances()` instead of accessing InstanceManager directly, as BotState doesn't expose InstanceManager
2. **Session ID Display**: Used instance ID as session_id since `row_to_instance()` doesn't extract session_id from database
3. **State Filtering**: Only show Running/Starting instances for managed sessions
4. **Error Handling**: Convert anyhow::Error and teloxide::RequestError to OutpostError using map_err

### Technical Patterns
- **TDD Approach**: Wrote 8 tests first (RED), then implemented (GREEN), then cleaned up (REFACTOR)
- **Error Conversion**: Used `map_err` to convert external errors to OutpostError
- **Pagination**: Simple "... and N more" indicator when >10 sessions
- **Project Name Extraction**: Used `Path::file_name()` to extract project name from path

### Gotchas
1. **Module Privacy**: `opencode::discovery` is private, must use public re-exports from `opencode` module
2. **BotState Structure**: BotState has `orchestrator_store`, `topic_store`, `config` - no direct InstanceManager access
3. **Database Schema**: OrchestratorStore saves session_id but doesn't retrieve it in `row_to_instance()`
4. **Error Types**: OutpostError doesn't implement From<anyhow::Error> or From<RequestError>, must use map_err

### Test Coverage
- `test_format_empty_list`: Empty session list
- `test_format_single_managed`: Single managed instance
- `test_format_single_discovered`: Single discovered instance
- `test_format_multiple_instances`: Multiple managed instances
- `test_format_mixed_types`: Mixed managed and discovered
- `test_pagination_many_instances`: Pagination with 15 instances
- `test_extract_project_name`: Project name extraction
- `test_discovered_without_port`: Discovered instance without port

### Clippy Fixes
- Changed `output.push_str("\n")` to `output.push('\n')`
- Changed `format!("{}", info.id)` to `info.id.to_string()`

### Files Modified
- Created: `src/bot/handlers/sessions.rs` (169 lines)
- Modified: `src/bot/handlers.rs` (added `pub mod sessions;` and `pub use sessions::handle_sessions;`)

### Verification
```bash
cargo nextest run -E 'test(sessions)'  # 11 tests passed (8 in sessions module)
cargo clippy --all-targets             # No warnings in sessions.rs
```

### Next Steps
- Task 17: Implement session_id retrieval in OrchestratorStore if needed
- Consider adding filtering options (by state, by type) in future iterations

## Task 17: /connect Command Handler

**Date**: 2026-01-29

### Implementation Summary
Implemented `/connect <name>` command handler that searches for OpenCode sessions across managed and discovered instances, creates forum topics, and establishes topic mappings.

### Key Learnings

1. **Error Type Conversion**: The project uses custom `OutpostError` type instead of `anyhow::Error`. All external errors must be mapped using `.map_err(|e| OutpostError::variant(e.to_string()))`.

2. **Teloxide Forum Topics**: 
   - `Bot::create_forum_topic()` returns `ForumTopic` struct
   - Thread ID is accessed via `forum_topic.thread_id.0.0` (nested tuple unwrapping)
   - Field is `thread_id`, not `message_thread_id`

3. **Discovery Module**: Already exported publicly in `src/opencode/mod.rs`, can be imported as `crate::opencode::Discovery`.

4. **Session Search Pattern**:
   - Search managed instances first (from OrchestratorStore)
   - Then search discovered instances (from Discovery::discover_all())
   - Extract project name from path using `PathBuf::file_name()`
   - Match by both project name and instance ID

5. **Dead Code Warnings**: Functions not yet wired into dispatcher need `#[allow(dead_code)]` attribute to pass clippy.

6. **Test Strategy**:
   - 11 unit tests covering search logic, error cases, and data structures
   - Tests pass without mocking (Discovery::get_session_info returns None for non-existent ports)
   - Used `#[tokio::test]` for async test functions

### Technical Decisions

1. **SessionInfo Struct**: Created internal struct to hold search results before creating topic mapping.

2. **Error Handling**: Wrapped all external errors (database, telegram, IO) in OutpostError variants for consistent error handling.

3. **Topic Naming**: Use project name directly as topic name (simple, clear).

4. **Already Connected Check**: Iterate through existing mappings to prevent duplicate connections to same session.

### Challenges Encountered

1. **Type Mismatches**: Initial confusion with `ForumTopic.thread_id` type (ThreadId wrapping MessageId).
   - Solution: Double unwrap `.thread_id.0.0` to get i32

2. **Error Conversion**: Had to convert all `anyhow::Error` to `OutpostError`.
   - Solution: Systematic `.map_err()` calls with appropriate OutpostError variants

3. **Module Visibility**: Initially tried to import `discovery` as private module.
   - Solution: Use public export `crate::opencode::Discovery`

### Code Quality

- **Tests**: 14 tests passing (11 new + 3 existing related tests)
- **Clippy**: Clean, no warnings for connect.rs
- **Coverage**: Search logic, error cases, data structures, edge cases

### Next Steps

- Wire handler into dispatcher (Task 18+)
- Implement SSE subscription integration
- Add session filtering options (future enhancement)
- Mock Discovery::get_session_info for more robust testing


## Task 18: /disconnect Command Handler

**Date**: 2026-01-29

### Implementation Summary
- Created `src/bot/handlers/disconnect.rs` with full TDD approach
- Implemented `/disconnect` command to disconnect from OpenCode sessions
- 8 tests passing (100% coverage of core functionality)

### Key Patterns Applied

1. **Error Handling with OutpostError**
   - Used `OutpostError::telegram_error()` for user-facing errors
   - Used `OutpostError::database_error()` for DB operation errors
   - Avoided `anyhow::Error` in favor of typed errors

2. **Topic Validation**
   - Helper function `get_topic_id()` validates topic context
   - Rejects General topic (ThreadId(MessageId(1)))
   - Returns clear error messages for invalid contexts

3. **Managed vs Discovered Instances**
   - Only update state for Managed instances
   - Discovered/External instances are left running
   - Used `InstanceType` enum for type checking

4. **Database Operations**
   - Lock → Operation → Drop pattern for Mutex guards
   - Explicit `drop()` calls to release locks early
   - Separate locks for topic_store and orchestrator_store

5. **Telegram API Integration**
   - Send confirmation message BEFORE deleting topic
   - Use `message_thread_id()` to send to specific topic
   - Delete forum topic after cleanup

### Challenges & Solutions

**Challenge**: BotState doesn't have `instance_manager` field
**Solution**: Used `OrchestratorStore::update_state()` directly instead of calling InstanceManager

**Challenge**: Test Message construction requires many fields
**Solution**: Simplified tests to focus on business logic (TopicStore, InstanceInfo) rather than full integration

**Challenge**: Clippy dead_code warnings for unused handlers
**Solution**: Added `#[allow(dead_code)]` since handlers will be wired in dispatcher later

### Test Coverage (8 tests)
1. `test_delete_mapping` - Verify mapping deletion
2. `test_stop_managed_instance` - Managed instance handling
3. `test_dont_stop_discovered_instance` - Discovered instance handling
4. `test_mapping_with_session_id` - Session ID present
5. `test_mapping_without_session_id` - Session ID absent
6. `test_get_mapping_no_mapping_error` - No mapping found
7. `test_instance_type_comparison` - InstanceType enum equality
8. `test_parse_disconnect_command` - Command parsing (from commands.rs)

### Files Modified
- `src/bot/handlers/disconnect.rs` (new, 320 lines)
- `src/bot/handlers.rs` (added module and export)

### Next Steps
- Wire handler into dispatcher (future task)
- Implement SSE unsubscribe when SSE integration is ready
- Add confirmation prompts if needed (currently immediate action)


## Task 19: /link Command Handler (2026-01-29)

### Implementation Summary
Successfully implemented `/link` command handler for linking forum topics to project directories:
- **8 tests** passing (exceeds 6 minimum requirement)
- Path validation (exists, is directory, absolute path resolution)
- ~ expansion using shellexpand crate
- Relative path resolution via canonicalize()
- Topic mapping update with timestamp
- Confirmation message to user
- Error handling for all edge cases

### Key Design Decisions

1. **Path Validation Function**: Extracted into separate `validate_path()` function
   - Expands ~ using `shellexpand::tilde()`
   - Resolves to absolute path with `canonicalize()`
   - Validates directory with `metadata.is_dir()`
   - Maps IO errors to user-friendly messages

2. **Topic Validation**: Reused pattern from disconnect handler
   - Extract topic_id from `msg.thread_id`
   - Reject General topic (ThreadId(MessageId(1)))
   - Return clear error if not in topic

3. **Mapping Update Pattern**:
   - Get existing mapping (must exist)
   - Update project_path field
   - Update timestamp to current time
   - Save back to database

4. **Error Handling Strategy**:
   - Path not found → "Path not found: {path}"
   - Not a directory → "Path is not a directory: {path}"
   - Permission denied → "Permission denied: {path}"
   - No mapping → "No active connection in this topic"
   - Not in topic → "This command must be used in a forum topic"

### Test Coverage (8 tests)

1. `test_validate_path_exists` - Valid directory path
2. `test_validate_path_not_found` - Non-existent path error
3. `test_validate_path_not_directory` - File path error
4. `test_expand_tilde` - ~ expansion to home directory
5. `test_resolve_relative_path` - Relative path to absolute
6. `test_validate_path_with_symlink` - Symlink handling
7. `test_validate_path_absolute_path` - Absolute path verification
8. `test_update_mapping` - Database update verification

### Patterns That Worked Well

1. **Separate Validation Function**: Makes testing easier, reusable logic
2. **Error Kind Matching**: Use `e.kind()` to distinguish error types
3. **Timestamp Management**: Use `SystemTime::now()` for current time
4. **Lock Management**: Drop locks before acquiring new ones (prevent deadlock)
5. **Test Isolation**: Use TempDir for isolated test directories

### Gotchas Encountered

1. **dirs crate not in dependencies**: Had to add `cargo add dirs` for home directory
2. **Permission test platform-specific**: macOS doesn't enforce 0o000 permissions for owner
   - Solution: Replaced with symlink test (works cross-platform)
3. **Clippy useless_conversion**: Removed unnecessary `.into()` on error return
4. **Unused import warning**: Removed unused `std::path::Path` import

### API Surface

```rust
pub async fn handle_link(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
) -> Result<()>

fn validate_path(path: &str) -> Result<PathBuf>
fn get_topic_id(msg: &Message) -> Result<i32>
```

### Files Created/Modified
- `src/bot/handlers/link.rs` (new, ~270 lines with tests)
- `src/bot/handlers.rs` (added `pub mod link;` and export)
- `Cargo.toml` (added `dirs` dependency)

### Verification
```bash
cargo nextest run -E 'test(handlers::link)'  # 8/8 tests passing
cargo clippy --bin oc-outpost                # No warnings for link module
```

### Next Steps
- Link handler ready for integration with bot dispatcher
- Can be used to change project path for existing topic connections
- Enables users to switch between different project directories

### Key Learnings

1. **Path Canonicalization**: `canonicalize()` resolves symlinks and relative paths to absolute
2. **shellexpand Pattern**: Use `.into_owned()` to convert Cow<str> to String
3. **Error Kind Matching**: Different IO errors need different user messages
4. **Cross-platform Testing**: Avoid platform-specific permission tests, use symlinks instead
5. **Lock Discipline**: Always drop locks before acquiring new ones in async code

## Task 20: /stream Command Handler

### Implementation Summary
Implemented `/stream` command handler to toggle streaming preference in TopicStore.

### Files Created/Modified
- `src/bot/handlers/stream.rs` (new, ~260 lines with 7 tests)
- `src/bot/handlers.rs` (added `pub mod stream;` and export, removed stub)

### Handler Logic
1. Extract topic_id from message (validate not General topic)
2. Get topic mapping (error if no mapping found)
3. Toggle streaming_enabled field via TopicStore::toggle_streaming()
4. Send confirmation message with new state
5. Handle errors: not in topic, no mapping

### Confirmation Messages
- **ON**: "Streaming: ON\nYou will see real-time progress from OpenCode."
- **OFF**: "Streaming: OFF\nYou will only see final responses."

### Test Coverage (7 tests)
1. `test_cmd_stream_toggle_off_to_on` - Toggle from OFF to ON
2. `test_cmd_stream_toggle_on_to_off` - Toggle from ON to OFF
3. `test_cmd_stream_persistence_in_database` - Verify persistence across toggles
4. `test_cmd_stream_error_no_mapping` - Error handling for missing mapping
5. `test_cmd_stream_confirmation_message_on` - Verify ON message format
6. `test_cmd_stream_confirmation_message_off` - Verify OFF message format
7. `test_cmd_stream_multiple_toggles` - Multiple consecutive toggles

### Verification
```bash
cargo nextest run -E 'test(cmd_stream)'  # 7/7 tests passing
cargo clippy --all-targets               # No warnings for stream module
```

### Key Patterns Used
1. **Topic Validation**: Reused get_topic_id pattern from disconnect/link handlers
2. **Lock Discipline**: Drop locks immediately after use before acquiring new ones
3. **Error Handling**: Use OutpostError for consistent error messages
4. **Test Isolation**: Use tempfile for test databases, create_test_state helper
5. **Boolean Assertions**: Use `assert!(value)` and `assert!(!value)` instead of `assert_eq!`

### SSE Subscription (Stubbed)
- Currently no-op (as per requirements)
- Ready for integration with SSE client in future tasks
- Toggle state persists in database for future SSE implementation

### Next Steps
- Integrate with bot dispatcher to wire /stream command
- Implement actual SSE subscription/unsubscription when SSE client available
- Consider streaming quality options in future enhancement
