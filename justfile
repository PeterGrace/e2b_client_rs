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
    {{just_executable()}} --justfile {{justfile()}} _stamp "$log"
    # --no-fail-fast: a failing binary must not stop the others from running.
    cargo test --no-fail-fast --test live_lifecycle --test live_commands --test live_files \
        -- --ignored --test-threads=4 --show-output 2>&1 | tee -a "$log"
    status=${PIPESTATUS[0]}
    cargo test --test live_sweep -- --ignored --show-output 2>&1 | tee -a "$log"
    sweep_status=${PIPESTATUS[0]}
    echo "run log: $log"
    # A failed sweep means leaked, billed sandboxes, so it fails the run too.
    if [ "$status" -ne 0 ]; then exit "$status"; fi
    exit "$sweep_status"

# For a server whose warm pool refills slowly (sandbawks). `gap` is in seconds; `filter`, an
# extended regex on test names, runs a subset: `just test-live-paced sandbawks 30 'clm_0007|clm_0033'`.
# A test that fails on server readiness is rerun once, after the gap. Writes target/live-logs/<run id>.log.
# Live tests one at a time with a gap between them: `just test-live-paced sandbawks`.
test-live-paced target gap="120" filter="":
    #!/usr/bin/env bash
    set -uo pipefail
    export E2B_TEST_TARGET="{{target}}"
    export E2B_TEST_RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-{{target}}-paced-$$"
    # quote() makes each a single-quoted shell word, so a `$(` or backtick in
    # the filter is matched literally rather than run.
    gap={{quote(gap)}}
    filter={{quote(filter)}}
    mkdir -p target/live-logs
    log="target/live-logs/${E2B_TEST_RUN_ID}.log"
    {{just_executable()}} --justfile {{justfile()}} _stamp "$log"
    echo "paced: one test at a time, ${gap}s apart; readiness failures rerun once" | tee -a "$log"
    # Messages that mean the server had no ready sandbox, so the test says
    # nothing about the feature (docs/verification/live-pacing.md).
    readiness='still starting up|Service Temporarily Unavailable|Unschedulable|will not become ready'
    binaries=(live_lifecycle live_commands live_files)
    # Build once up front, so a compile error stops the run instead of
    # showing up as a failure of every test.
    if ! cargo test --no-run "${binaries[@]/#/--test=}" >>"$log" 2>&1; then
        echo "build failed; see $log" >&2
        exit 1
    fi
    # Runs one test; its output goes to the file named by $3.
    run_one() {
        cargo test --test "$1" -- --ignored --exact "$2" --test-threads=1 --show-output >"$3" 2>&1
    }
    status=0
    first=1
    retry=()
    for bin in "${binaries[@]}"; do
        mapfile -t tests < <(cargo test --test "$bin" -- --ignored --list 2>/dev/null | sed -n 's/: test$//p')
        for test in "${tests[@]}"; do
            if [ -n "$filter" ] && ! grep -Eq -- "$filter" <<<"$test"; then continue; fi
            [ "$first" -eq 1 ] || sleep "$gap"
            first=0
            out="$(mktemp -p target/live-logs paced.XXXXXX)"
            if run_one "$bin" "$test" "$out"; then result=PASS
            elif grep -Eq -- "$readiness" "$out"; then result=FAIL-READINESS; retry+=("$bin $test")
            else result=FAIL; status=1; fi
            cat "$out" >>"$log"
            rm -f "$out"
            echo "RESULT $test $result" | tee -a "$log"
        done
    done
    # One rerun for each readiness failure, after the gap, so the pool can refill.
    for entry in "${retry[@]}"; do
        bin="${entry%% *}"
        test="${entry#* }"
        sleep "$gap"
        out="$(mktemp -p target/live-logs paced.XXXXXX)"
        if run_one "$bin" "$test" "$out"; then result=PASS-ON-RERUN
        else result=FAIL-READINESS; status=1; fi
        cat "$out" >>"$log"
        rm -f "$out"
        echo "RESULT $test $result (rerun)" | tee -a "$log"
    done
    cargo test --test live_sweep -- --ignored --show-output 2>&1 | tee -a "$log"
    sweep_status=${PIPESTATUS[0]}
    echo "summary: $(grep -c '^RESULT .* PASS$' "$log") pass, $(grep -c 'PASS-ON-RERUN' "$log") pass on rerun, $(grep -c '^RESULT .* FAIL$' "$log") fail, $(grep -c '^RESULT .* FAIL-READINESS (rerun)$' "$log") still not ready" | tee -a "$log"
    echo "run log: $log" | tee -a "$log"
    # A failed sweep means leaked sandboxes, so it fails the run too.
    if [ "$status" -ne 0 ]; then exit "$status"; fi
    exit "$sweep_status"

# Build the API docs with warnings as errors.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

# Writes the two lines that say which code produced a live log: the commit and
# whether the tree was dirty. Outside a git checkout, or without git, both say
# `unknown` rather than lying. Used by `test-live` and `test-live-paced`.
[private]
_stamp log:
    #!/usr/bin/env bash
    set -uo pipefail
    {
        echo "commit: $(git rev-parse --verify -q HEAD 2>/dev/null || echo unknown)"
        if ! porcelain="$(git status --porcelain 2>/dev/null)"; then echo "dirty: unknown"
        elif [ -n "$porcelain" ]; then echo "dirty: yes"
        else echo "dirty: no"; fi
    } | tee "{{log}}"
