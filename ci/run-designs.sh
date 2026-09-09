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
#   REPORTS            directory for .lyrdb reports of failed designs (default: ci/reports)
set -euo pipefail

process=${1:?usage: $0 <process>}
here=$(cd "$(dirname "$0")" && pwd)
ref=${REFERENCE_DESIGNS:-$here/../../reference-designs}
gdscheck=${GDSCHECK:-$here/../target/release/gdscheck}
suite=${SUITE:-main}
limit=${TIMEOUT:-15m}
reports=${REPORTS:-$here/reports}

[[ -x $gdscheck ]] || gdscheck=gdscheck
command -v "$gdscheck" >/dev/null || { echo "gdscheck not found" >&2; exit 1; }
[[ -x $ref/designs.py ]] || { echo "reference designs not found at $ref" >&2; exit 1; }
mkdir -p "$reports"

# gdscheck exit codes: 0 clean, 1 error, 2 violations. timeout(1) returns 124 on
# expiry; a Rust panic exits 101.
outcome() {
    case $1 in
        0)   echo PASS ;;
        2)   echo "FAIL (violations)" ;;
        124) echo "FAIL (timeout after $limit)" ;;
        101) echo "FAIL (crash)" ;;
        *)   echo "FAIL (exit $1)" ;;
    esac
}

failed=()
count=0
while IFS=$'\t' read -r name _ path topcell; do
    count=$((count + 1))
    report=$reports/$name.lyrdb
    echo "=== $name ($process, suite $suite, top cell $topcell) ==="
    status=0
    timeout --kill-after=30s "$limit" \
        "$gdscheck" run --process "$process" --suite "$suite" \
        --topcell "$topcell" --input "$path" --report "$report" || status=$?
    result=$(outcome "$status")
    echo "=== $name: $result"
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
