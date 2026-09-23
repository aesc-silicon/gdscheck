#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Run gdscheck over every reference design of one PDK to catch crashes, hangs and
# false violations on real layouts. The designs are not golden references; every
# rule has its own synthetic fixture, so a real design is expected to be clean.
#
#   ci/run-designs.sh <process>
#
# Environment:
#   REFERENCE_DESIGNS  checkout of aesc-silicon/reference-designs (default: ../reference-designs)
#   GDSCHECK           binary to use (default: target/release/gdscheck, else from PATH)
#   SUITE              suite to run (default: main)
#   TIMEOUT            per-design wall-clock limit, timeout(1) syntax (default: 15m)
#   MEMORY_LIMIT       run every design in a transient cgroup with this memory.max
#                      (systemd-run syntax, e.g. 12G; needs root - sudo on a runner)
#   REPORTS            directory for .lyrdb reports of failed designs (default: ci/reports)
#
# Every run is traced (GDSCHECK_RULE_TRACE) under /usr/bin/time, and its performance
# figures - wall, CPU, cores, peak memory, net extraction, the slowest rules - are
# printed after the run and, under GitHub Actions, put in the job summary as a table,
# so a slower rule shows up in the log of the night it got slower.
set -euo pipefail

process=${1:?usage: $0 <process>}
here=$(cd "$(dirname "$0")" && pwd)
ref=${REFERENCE_DESIGNS:-$here/../../reference-designs}
gdscheck=${GDSCHECK:-$here/../target/release/gdscheck}
suite=${SUITE:-main}
limit=${TIMEOUT:-15m}
reports=${REPORTS:-$here/reports}
traces=$(mktemp -d)

[[ -x $gdscheck ]] || gdscheck=gdscheck
command -v "$gdscheck" >/dev/null || { echo "gdscheck not found" >&2; exit 1; }
[[ -x $ref/designs.py ]] || { echo "reference designs not found at $ref" >&2; exit 1; }
mkdir -p "$reports"
if [[ -n ${GITHUB_STEP_SUMMARY:-} ]]; then
    {
        echo "## $process"
        echo
        echo "| design | wall s | cpu s | cores | peak GB | nets s | rules s | between s |"
        echo "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
    } >> "$GITHUB_STEP_SUMMARY"
fi

# gdscheck exit codes: 0 clean (waived findings included), 1 error, 2 violations, 3 a
# rule not checked for its memory.  timeout(1) returns 124 on expiry; a Rust panic
# exits 101.
outcome() {
    case $1 in
        0)   echo PASS ;;
        2)   echo "FAIL (violations)" ;;
        3)   echo "FAIL (incomplete: a rule was not checked)" ;;
        124) echo "FAIL (timeout after $limit)" ;;
        101) echo "FAIL (crash)" ;;
        *)   echo "FAIL (exit $1)" ;;
    esac
}

# Under MEMORY_LIMIT every run is put in a transient scope with that memory.max: the
# run has to plan within it, and the kernel kills what does not.  A runner has sudo
# and no user session for systemd, so the scope is a system one, run as this user.
scope=()
if [[ -n ${MEMORY_LIMIT:-} ]]; then
    scope=(sudo systemd-run --scope --quiet -p "MemoryMax=$MEMORY_LIMIT" -p MemorySwapMax=0
           sudo -u "$(id -un)" --preserve-env=GDSCHECK_RULE_TRACE,HOME)
    echo "every run under a cgroup limit of $MEMORY_LIMIT"
fi

failed=()
count=0
while IFS=$'\t' read -r name _ path topcell; do
    count=$((count + 1))
    report=$reports/$name.lyrdb
    echo "=== $name ($process, suite $suite, top cell $topcell) ==="
    status=0
    trace=$traces/$name.trace
    # The trace goes to stderr, with the run's own diagnostics, and stdout stays the
    # log; the summary reads both, since the run prints its totals to stdout.
    GDSCHECK_RULE_TRACE=1 timeout --kill-after=30s "$limit" \
        "${scope[@]}" /usr/bin/time -v -o "$trace.time" \
        "$gdscheck" run --process "$process" --suite "$suite" \
        --topcell "$topcell" --input "$path" --report "$report" \
        2>"$trace" | tee "$trace.out" || status=${PIPESTATUS[0]}
    result=$(outcome "$status")
    echo "=== $name: $result"
    "$here/perf-summary.py" "$name" "$trace" "$trace.time" "$trace.out"
    if [[ -n ${GITHUB_STEP_SUMMARY:-} ]]; then
        "$here/perf-summary.py" "$name" "$trace" "$trace.time" "$trace.out" --markdown \
            >> "$GITHUB_STEP_SUMMARY"
    fi
    if [[ $status -eq 0 ]]; then
        rm -f "$report"
    else
        failed+=("$name: $result")
    fi
done < <("$ref/designs.py" --tool gdscheck --process "$process" --absolute)

echo
echo "$count design(s) run for $process, ${#failed[@]} failed"
[[ ${#failed[@]} -eq 0 ]] || printf '  %s\n' "${failed[@]}"
[[ ${#failed[@]} -eq 0 ]]
