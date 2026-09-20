#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# One layout through gdscheck and through IHP's own KLayout decks, rule by rule.
#
#   ci/hardening/oracle-ihp.sh <layout.gds[.gz]> [topcell] [tile_um ...]
#
# Prints one line per rule either side reported: the rule, gdscheck's marker count at
# each tile size asked for (default 20 and 7), and KLayout's.  KLayout is the driver
# (`run_drc.py`, every FEOL and BEOL table) plus the maximal rule set, run in the
# elements container over the IHP-Open-PDK checkout.  Marker counts need not agree - the
# two tools cut a violation into markers differently - but a rule one side reports and
# the other does not is a finding, as is a count that changes with the tile size.
#
# Environment:
#   PROCESS    gdscheck process (default: ihp-sg13g2)
#   SUITE      gdscheck suite (default: core - no antenna, no density)
#   PDKS       IHP PDK checkouts, holding ihp-sg13g2/ (default: ~/work/aesc/ElemRV/pdks)
#   IMAGE      container image (default: localhost/dnltz/elements:v2.8)
#   GDSCHECK   binary (default: target/release/gdscheck)
#   KEEP       a directory to keep the KLayout run (logs, .lyrdb) in
set -euo pipefail

layout=${1:?usage: $0 <layout.gds[.gz]> [topcell] [tile_um ...]}
topcell=${2:-TOP}
shift $(( $# >= 2 ? 2 : $# ))
tiles=("$@")
[[ ${#tiles[@]} -gt 0 ]] || tiles=(20 7)

here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
process=${PROCESS:-ihp-sg13g2}
suite=${SUITE:-core}
pdks=${PDKS:-$HOME/work/aesc/ElemRV/pdks}
image=${IMAGE:-localhost/dnltz/elements:v2.8}
gdscheck=${GDSCHECK:-$root/target/release/gdscheck}
work=${KEEP:-$(mktemp -d)}
mkdir -p "$work"

# The container reads a plain GDS from the work directory.
name=$(basename "$layout")
name=${name%.gz}
if [[ $layout == *.gz ]]; then
    gunzip -c "$layout" > "$work/$name"
else
    cp "$layout" "$work/$name"
fi

drc=/pdk/ihp-sg13g2/libs.tech/klayout/tech/drc
podman run --rm -v "$work:/work" -v "$pdks:/pdk:ro" "$image" \
    python3 $drc/run_drc.py --path="/work/$name" --topcell="$topcell" \
        --run_dir=/work/driver --no_density --mp=4 > "$work/driver.out" 2>&1 || true
podman run --rm -v "$work:/work" -v "$pdks:/pdk:ro" "$image" \
    klayout -b -r $drc/rule_decks/sg13g2_maximal.drc \
        -rd input="/work/$name" -rd topcell="$topcell" -rd report=/work/maximal.lyrdb \
        -rd log=/work/maximal.log -rd threads=4 -rd run_mode=flat \
        -rd fillerRules=false -rd latchUpRules=false -rd recommendedRules=false \
        > "$work/maximal.out" 2>&1 || true

# KLayout's counts: every item's category, from both reports.
klayout_counts() {
    cat "$work"/driver/*.lyrdb "$work"/maximal.lyrdb 2>/dev/null \
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
