# oc-outpost

A Telegram bot that orchestrates multiple OpenCode instances through forum topics.

## Overview

oc-outpost is a Rust implementation of the OpenCode Telegram bot, enabling users to interact with multiple isolated OpenCode environments through a single Telegram bot interface. Each forum topic in a Telegram supergroup corresponds to a separate OpenCode instance.

## Features

- **Multi-instance orchestration**: Manage multiple OpenCode instances concurrently
- **Forum topic integration**: One OpenCode instance per forum topic
- **Idle timeout management**: Automatically stop idle instances to save resources
- **Health monitoring**: Periodic health checks for running instances
- **API server**: External instance registration and management
- **SQLite persistence**: Track instance state and topic metadata

## Requirements

- Rust 1.82+
- OpenCode binary available in PATH or configured via `OPENCODE_PATH`
- Telegram bot token from @BotFather
- Supergroup with forum topics enabled

## Quick Start

### 1. Clone and Setup

```bash
git clone https://github.com/huynle/opencode-telegram.git
cd opencode-telegram
cp .env.example .env
# Edit .env with your configuration
```

### 2. Configure Environment

Required variables:
- `TELEGRAM_BOT_TOKEN`: Your bot token from @BotFather
- `TELEGRAM_CHAT_ID`: Your supergroup ID (negative number)
- `PROJECT_BASE_PATH`: Base directory for project files

### 3. Build and Run

```bash
# Development
just run

# Production build
just build
./target/release/oc-outpost
```

## Development

### Available Commands

```bash
just check      # Run all checks (format, lint, test)
just fmt        # Format code
just clippy     # Run linter
just test       # Run tests
just build      # Build release binary
just run        # Run in dev mode
just doc        # Generate documentation
just audit      # Check for security vulnerabilities
```

### Project Structure

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

## Configuration

See `.env.example` for all available configuration options:

- **Telegram**: Bot token, chat ID, allowed users
- **OpenCode**: Instance limits, timeouts, port ranges
- **Storage**: Database paths
- **API**: Server port and authentication

## Architecture

- **Teloxide**: Telegram bot framework
- **Tokio**: Async runtime
- **SQLx**: Type-safe database queries
- **Axum**: HTTP API server
- **Reqwest**: HTTP client for OpenCode communication

## License

MIT

## Contributing

Contributions welcome! Please ensure:
- Code passes `cargo fmt` and `cargo clippy`
- Tests pass with `cargo test`
- Commit messages follow conventional commits
