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

# Unit, mock-server and doc tests (live tests are ignored by default).
test:
    cargo test

# Keys come from E2B_API_KEY_<TARGET> (env or the nearest .env). Writes target/live-logs/<run id>.log.
# Live tests against a real server: `just test-live hosted` or `just test-live sandbawks`.
test-live target:
    #!/usr/bin/env bash
    set -uo pipefail
    export E2B_TEST_TARGET="{{target}}"
    export E2B_TEST_RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-{{target}}-$$"
    mkdir -p target/live-logs
    log="target/live-logs/${E2B_TEST_RUN_ID}.log"
    cargo test --test live_lifecycle --test live_commands --test live_files \
        -- --ignored --test-threads=4 --show-output 2>&1 | tee "$log"
    status=${PIPESTATUS[0]}
    cargo test --test live_sweep -- --ignored --show-output 2>&1 | tee -a "$log"
    sweep_status=${PIPESTATUS[0]}
    echo "run log: $log"
    # A failed sweep means leaked, billed sandboxes, so it fails the run too.
    if [ "$status" -ne 0 ]; then exit "$status"; fi
    exit "$sweep_status"

# Build the API docs with warnings as errors.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
