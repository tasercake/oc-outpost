# Development task automation for oc-outpost

# Default recipe
default:
    @just --list

# Run all checks (format, lint, test)
check: fmt-check clippy test
    @echo "✓ All checks passed"

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt -- --check

# Run clippy linter
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run tests
test:
    cargo test --lib

# Build release binary
build:
    cargo build --release

# Run in development mode
run:
    cargo run

# Clean build artifacts
clean:
    cargo clean

# Update dependencies
update:
    cargo update

# Generate documentation
doc:
    cargo doc --no-deps --open

# Check for security vulnerabilities
audit:
    cargo audit

# Format and lint in one command
lint: fmt clippy
    @echo "✓ Linting complete"
