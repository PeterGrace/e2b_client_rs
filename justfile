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

# Each fixture is one test's output; its name before `--` is the label expected.
# Tests the paced recipe's result classifier against tests/fixtures/live-pacing/.
test-recipes:
    #!/usr/bin/env bash
    set -uo pipefail
    status=0
    for fixture in tests/fixtures/live-pacing/*.txt; do
        name="$(basename "$fixture" .txt)"
        expected="${name%%--*}"
        # A passing (or skipped) test exits 0; a failing cargo test exits 101.
        code=101
        case "$expected" in PASS|SKIP) code=0 ;; esac
        actual="$({{just_executable()}} --justfile {{justfile()}} _classify "$code" "$fixture")"
        if [ "$actual" = "$expected" ]; then echo "ok   $name"
        else echo "FAIL $name: got $actual"; status=1; fi
    done
    exit "$status"

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
# A readiness or sandbox-failed result is rerun once, after the gap. Writes target/live-logs/<run id>.log.
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
    echo "paced: one test at a time, ${gap}s apart; readiness and sandbox-failed results rerun once" \
        | tee -a "$log"
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
    # Prints the label for one run from its exit status ($1) and output file
    # ($2); see `_classify`.
    classify() {
        {{just_executable()}} --justfile {{justfile()}} _classify "$1" "$2"
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
            run_one "$bin" "$test" "$out"
            result="$(classify "$?" "$out")"
            # Anything but a pass or a rerun class fails the run, including an
            # unexpected or empty label (say `just` failed to start).
            case "$result" in
                PASS) ;;
                FAIL-READINESS|FAIL-SANDBOX-FAILED) retry+=("$bin $test") ;;
                *) status=1 ;;
            esac
            cat "$out" >>"$log"
            rm -f "$out"
            echo "RESULT $test $result" | tee -a "$log"
        done
    done
    # One rerun for each readiness or sandbox-failed result, after the gap, so
    # the pool can refill. A rerun that fails keeps its new label: the same
    # class again, or FAIL if it failed some other way.
    for entry in "${retry[@]}"; do
        bin="${entry%% *}"
        test="${entry#* }"
        sleep "$gap"
        out="$(mktemp -p target/live-logs paced.XXXXXX)"
        run_one "$bin" "$test" "$out"
        result="$(classify "$?" "$out")"
        case "$result" in
            PASS) result=PASS-ON-RERUN ;;
            *) status=1 ;;
        esac
        cat "$out" >>"$log"
        rm -f "$out"
        echo "RESULT $test $result (rerun)" | tee -a "$log"
    done
    cargo test --test live_sweep -- --ignored --show-output 2>&1 | tee -a "$log"
    sweep_status=${PIPESTATUS[0]}
    # Counts one kind of RESULT line in the log (an extended regex after `RESULT <test> `).
    count() { grep -Ec -- "^RESULT [^ ]+ ($1)\$" "$log"; }
    echo "summary: $(count PASS) pass, $(count PASS-ON-RERUN' \(rerun\)') pass on rerun," \
        "$(count 'FAIL( \(rerun\))?') fail, $(count 'FAIL-READINESS \(rerun\)') still not ready," \
        "$(count 'FAIL-SANDBOX-FAILED \(rerun\)') sandbox failed" | tee -a "$log"
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

# Prints the paced result label for one test run, from its exit status and
# its output file: PASS, FAIL-READINESS (no ready sandbox: capacity),
# FAIL-SANDBOX-FAILED (the sandbox failed outright: capacity or, as with an
# unknown template, a server defect; a person classifies it) or FAIL.
# Used by `test-live-paced`; `test-recipes` tests it.
[private]
_classify status out:
    #!/usr/bin/env bash
    set -uo pipefail
    # Messages that mean the server had no ready sandbox, so the test says
    # nothing about the feature (docs/verification/live-pacing.md). `will not
    # become ready` is not one: it also prefixes a sandbox that failed outright.
    readiness='still starting up|Service Temporarily Unavailable|Unschedulable'
    # A sandbox that failed outright (gfd/sandbawks#680 on an unknown
    # template; M2 also saw it on capacity).
    failed='the sandbox failed \(action_failure\)'
    if [ {{quote(status)}} -eq 0 ]; then echo PASS
    elif grep -Eq -- "$readiness" {{quote(out)}}; then echo FAIL-READINESS
    elif grep -Eq -- "$failed" {{quote(out)}}; then echo FAIL-SANDBOX-FAILED
    else echo FAIL; fi
