#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Two builds' rule traces of one design, compared: the best run of each rule on each
side, summed per check family, and the rules that moved most either way.

    ci/bench-compare.py <name> <label-a> <label-b> <trace-a>... -- <trace-b>...
"""

import re
import sys


def best(paths):
    rules = {}
    totals = []
    for p in paths:
        text = open(p, errors="replace").read()
        for m in re.finditer(r"^rule (\S+) (\S+) ([\d.]+)s cpu=([\d.]+)s", text, re.M):
            rid, check, wall, cpu = m.group(1), m.group(2), float(m.group(3)), float(m.group(4))
            cur = rules.get(rid)
            if cur is None or wall < cur[1]:
                rules[rid] = (check, wall, cpu)
        m = re.search(r"^phase rules wall=([\d.]+)s cpu=([\d.]+)s", text, re.M)
        if m:
            totals.append((float(m.group(1)), float(m.group(2))))
    return rules, (min(totals) if totals else (0.0, 0.0))


def main():
    name, la, lb = sys.argv[1:4]
    rest = sys.argv[4:]
    cut = rest.index("--")
    ra, ta = best(rest[:cut])
    rb, tb = best(rest[cut + 1:])
    print(f"--- {name}: {la} -> {lb} (best of runs)")
    print(f"    rules phase wall {ta[0]:.1f}s -> {tb[0]:.1f}s   cpu {ta[1]:.0f}s -> {tb[1]:.0f}s")
    fam = {}
    for rid, (check, w, c) in ra.items():
        f = fam.setdefault(check, [0.0, 0.0, 0.0, 0.0])
        f[0] += w
        f[2] += c
        if rid in rb:
            f[1] += rb[rid][1]
            f[3] += rb[rid][2]
    print("    per check          wall a -> b       cpu a -> b")
    for check, f in sorted(fam.items(), key=lambda kv: -kv[1][0])[:12]:
        print(f"      {check:16} {f[0]:6.1f} -> {f[1]:6.1f}   {f[2]:6.0f} -> {f[3]:6.0f}")
    moved = [
        (rb[r][1] - w, r, w, rb[r][1]) for r, (_, w, _) in ra.items() if r in rb and abs(rb[r][1] - w) >= 0.2
    ]
    moved.sort()
    if moved:
        print("    rules that moved 0.2 s or more (wall a -> b):")
        shown = moved if len(moved) <= 12 else moved[:6] + [None] + moved[-6:]
        for m in shown:
            if m is None:
                print("      ...")
            else:
                _, r, w, w2 = m
                print(f"      {r:16} {w:6.1f} -> {w2:6.1f}")
    only_a = sorted(set(ra) - set(rb))
    only_b = sorted(set(rb) - set(ra))
    if only_a or only_b:
        print(f"    rules only in {la}: {len(only_a)}, only in {lb}: {len(only_b)}")


if __name__ == "__main__":
    main()
