# Development tasks. Run `just --list` to see them.

# Format the code.
fmt:
    cargo fmt --all

# Check formatting without changing files.
fmt-check:
    cargo fmt --all -- --check

# Formatting check plus clippy with warnings as errors.
lint: fmt-check
    cargo clippy --all-targets -- -D warnings

# Unit, mock-server and doc tests (integration tests are ignored by default).
test:
    cargo test

# End-to-end tests; needs E2B_API_KEY (and optionally E2B_API_URL, E2B_DOMAIN, E2B_TEMPLATE).
test-integration:
    cargo test --test integration -- --ignored --test-threads=1

# Build the API docs with warnings as errors.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
