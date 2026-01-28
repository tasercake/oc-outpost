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

