#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Two commits against each other on the reference designs, on this machine.  Both are
# built in worktrees of their own, and each design is run under each build, in turns,
# as many times as RUNS says, with the rule trace on; the best of each rule's runs is
# what counts, since the worst is whatever else the machine was doing.  The comparison
# is per check family and per rule, wall and CPU alike.
#
#   ci/bench.sh <ref-a> <ref-b> [process ...]
#
# Environment:
#   REFERENCE_DESIGNS  checkout of aesc-silicon/reference-designs (default: ../reference-designs)
#   RUNS               runs per build and design (default: 2)
#   THREADS            threads per run (default: every core)
#   SUITE              suite to run (default: main)
#   DESIGN             one more design, as process:topcell:path (may repeat with ;)
#   BENCH_DIR          where builds and traces go (default: target/bench)
set -euo pipefail

a=${1:?usage: $0 <ref-a> <ref-b> [process ...]}
b=${2:?usage: $0 <ref-a> <ref-b> [process ...]}
shift 2
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
ref=${REFERENCE_DESIGNS:-$root/../reference-designs}
runs=${RUNS:-2}
threads=${THREADS:-0}
suite=${SUITE:-main}
bench=${BENCH_DIR:-$root/target/bench}
mkdir -p "$bench"

build() {
    local rev=$1 sha dir
    sha=$(git -C "$root" rev-parse --short "$rev")
    dir=$bench/$sha
    if [[ ! -x $dir/bin/gdscheck ]]; then
        echo "building $rev ($sha)" >&2
        rm -rf "$dir/src"
        git -C "$root" worktree add -q --detach "$dir/src" "$rev"
        (cd "$dir/src" && cargo build --release --target-dir "$dir/target" >"$dir/build.log" 2>&1)
        mkdir -p "$dir/bin"
        cp "$dir/target/release/gdscheck" "$dir/bin/gdscheck"
        git -C "$root" worktree remove --force "$dir/src"
        rm -rf "$dir/target"
    fi
    echo "$dir/bin/gdscheck"
}

bin_a=$(build "$a")
bin_b=$(build "$b")

designs=()
if [[ $# -eq 0 ]]; then
    set -- ihp-sg13g2 ihp-sg13cmos5l gf180mcuD
fi
for process in "$@"; do
    while IFS=$'\t' read -r name _ path topcell; do
        designs+=("$process:$topcell:$path:$name")
    done < <("$ref/designs.py" --tool gdscheck --process "$process" --absolute)
done
if [[ -n ${DESIGN:-} ]]; then
    IFS=';' read -ra extra <<<"$DESIGN"
    for d in "${extra[@]}"; do
        designs+=("$d:$(basename "${d##*:}" .gds)")
    done
fi

traces=$bench/traces
rm -rf "$traces"
mkdir -p "$traces"
for d in "${designs[@]}"; do
    IFS=':' read -r process topcell path name <<<"$d"
    for ((i = 1; i <= runs; i++)); do
        for side in a b; do
            bin=$bin_a
            [[ $side = b ]] && bin=$bin_b
            echo "run $i/$runs $side $name"
            GDSCHECK_RULE_TRACE=1 "$bin" run --process "$process" --suite "$suite" \
                --topcell "$topcell" --input "$path" --threads "$threads" \
                >"$traces/$name.$side.$i.out" 2>"$traces/$name.$side.$i.trace" || true
        done
    done
    "$here/bench-compare.py" "$name" "$a" "$b" "$traces"/"$name".a.*.trace -- "$traces"/"$name".b.*.trace
    diff <(grep -E '^\[|violation' "$traces/$name.a.1.out" | grep -v Checking) \
         <(grep -E '^\[|violation' "$traces/$name.b.1.out" | grep -v Checking) >/dev/null \
        && echo "  findings: same" || echo "  findings: DIFFER (see $traces/$name.*.out)"
done
