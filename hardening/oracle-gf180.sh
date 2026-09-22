#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# One layout through gdscheck and through GlobalFoundries' own KLayout deck for
# GF180MCU, rule by rule - the GF180 twin of oracle-ihp.sh.
#
#   hardening/oracle-gf180.sh <layout.gds[.gz]> [topcell] [tile_um ...]
#
# Prints one line per rule either side reported: the rule, gdscheck's marker count at
# each tile size asked for (default 20 and 7), and KLayout's.  KLayout is the upstream
# `gf180mcu.drc` runset (every deck, variant D, flat) from the ciel checkout, run in the
# elements container - its klayout 0.30.9 has the `nets` and `evaluate_nets` the runset
# wants, the machine's 0.30.0 has not.  Marker counts need not agree - the two tools cut
# a violation into markers differently - but a rule one side reports and the other does
# not is a finding, as is a count that changes with the tile size.  The runset reads the
# density rules (M1.4-M5.4, MT.3, PL.8, DCF.1b ...) on any layout, a tiny one included:
# those are noise unless the layout is about them.
#
# Environment:
#   PROCESS    gdscheck process (default: gf180mcuD)
#   SUITE      gdscheck suite (default: main)
#   DRC        the upstream deck directory holding gf180mcu.drc
#              (default: the newest ~/.ciel/ciel/gf180mcu/versions/*/gf180mcuD/libs.tech/klayout/tech/drc)
#   VARIANT    the runset's variant (default: D)
#   DECKS      the runset's `decks` option (default: all)
#   IMAGE      container image (default: localhost/dnltz/elements:v2.8)
#   GDSCHECK   binary (default: target/release/gdscheck)
#   KEEP       a directory to keep the KLayout run (log, .lyrdb) in
set -euo pipefail

layout=${1:?usage: $0 <layout.gds[.gz]> [topcell] [tile_um ...]}
topcell=${2:-TOP}
shift $(( $# >= 2 ? 2 : $# ))
tiles=("$@")
[[ ${#tiles[@]} -gt 0 ]] || tiles=(20 7)

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/.." && pwd)
process=${PROCESS:-gf180mcuD}
suite=${SUITE:-main}
drc=${DRC:-$(ls -d "$HOME"/.ciel/ciel/gf180mcu/versions/*/gf180mcuD/libs.tech/klayout/tech/drc | tail -1)}
variant=${VARIANT:-D}
decks=${DECKS:-all}
image=${IMAGE:-localhost/dnltz/elements:v2.8}
gdscheck=${GDSCHECK:-$root/target/release/gdscheck}
work=${KEEP:-$(mktemp -d)}
mkdir -p "$work"

# The deck reads a plain GDS.
name=$(basename "$layout")
name=${name%.gz}
if [[ $layout == *.gz ]]; then
    gunzip -c "$layout" > "$work/$name"
else
    cp "$layout" "$work/$name"
fi

podman run --rm -v "$work:/work" -v "$drc:/drc:ro" "$image" \
    klayout -b -r /drc/gf180mcu.drc \
        -rd input="/work/$name" -rd topcell="$topcell" -rd report=/work/gf180.lyrdb \
        -rd variant="$variant" -rd decks="$decks" -rd run_mode=flat -rd threads=4 \
        > "$work/gf180.log" 2>&1 || true

# KLayout's counts: every item's category.
klayout_counts() {
    cat "$work"/gf180.lyrdb 2>/dev/null \
        | grep -o "<category>[^<]*</category>" \
        | sed "s/<category>'\{0,1\}//; s/'\{0,1\}<\/category>//" \
        | sort | uniq -c | awk '{print $2, $1}'
}

# gdscheck's counts at one tile size: every marker line's rule.
gdscheck_counts() {
    "$gdscheck" run --process "$process" --suite "$suite" --topcell "$topcell" \
        --input "$layout" --tile "$1" -v 2>/dev/null \
        | grep -E '^\s*\[[^]]+\] .*µm' | grep -v Checking | sed -E 's/^\s*\[([^]]+)\].*/\1/' \
        | sort | uniq -c | awk '{print $2, $1}'
}

declare -A kl gc
while read -r rule n; do kl[$rule]=$n; done < <(klayout_counts)
for t in "${tiles[@]}"; do
    while read -r rule n; do gc["$rule@$t"]=$n; done < <(gdscheck_counts "$t")
done
rules=$( { for k in "${!kl[@]}"; do echo "$k"; done; for k in "${!gc[@]}"; do echo "${k%@*}"; done; } | sort -u)

printf '%-22s' rule
for t in "${tiles[@]}"; do printf '%10s' "gdscheck@$t"; done
printf '%10s  %s\n' klayout note
for rule in $rules; do
    printf '%-22s' "$rule"
    first=; same=1
    for t in "${tiles[@]}"; do
        n=${gc["$rule@$t"]:-0}
        printf '%10s' "$n"
        [[ -z $first ]] && first=$n
        [[ $n == "$first" ]] || same=0
    done
    k=${kl[$rule]:-0}
    note=
    [[ $same == 1 ]] || note="TILE-DEPENDENT"
    if [[ ${first:-0} -gt 0 && $k -eq 0 ]]; then note="$note gdscheck-only"; fi
    if [[ ${first:-0} -eq 0 && $k -gt 0 ]]; then note="$note klayout-only"; fi
    printf '%10s  %s\n' "$k" "$note"
done
[[ -n ${KEEP:-} ]] || rm -rf "$work"
