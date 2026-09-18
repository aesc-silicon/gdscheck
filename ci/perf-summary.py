#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""The performance figures of one gdscheck run, from its trace.

Reads the stderr of a run made under `GDSCHECK_RULE_TRACE=1`, the `/usr/bin/time -v`
accounting and the run's stdout - any number of files, concatenated - and prints what
a nightly log should carry: wall and CPU time, the cores those amount to, peak memory,
net extraction, the time in rules and between them, and the slowest rules.  With
`--markdown` the same as one table row plus a list, for a job summary.

    ci/perf-summary.py <name> <file>... [--markdown]
"""

import re
import sys


def load(paths):
    text = "\n".join(open(p, errors="replace").read() for p in paths)
    rules = [
        (m.group(1), m.group(2), float(m.group(3)), float(m.group(4)))
        for m in re.finditer(r"^rule (\S+) (\S+) ([\d.]+)s cpu=([\d.]+)s", text, re.M)
    ]
    phases = {
        m.group(1): (float(m.group(2)), float(m.group(3)))
        for m in re.finditer(r"^phase (\S+) wall=([\d.]+)s cpu=([\d.]+)s", text, re.M)
    }
    plan = sum(float(m.group(1)) for m in re.finditer(r"^plan \S+ ([\d.]+)s", text, re.M))
    grab = lambda pat, conv=float: (lambda m: conv(m.group(1)) if m else None)(re.search(pat, text, re.M))
    return {
        "rules": rules,
        "phases": phases,
        "plan": plan,
        "wall": grab(r"Elapsed \(wall clock\) time.*?: (\S+)$", elapsed),
        "user": grab(r"User time \(seconds\): ([\d.]+)"),
        "sys": grab(r"System time \(seconds\): ([\d.]+)"),
        "rss_gb": grab(r"Maximum resident set size \(kbytes\): (\d+)", lambda x: int(x) / 1e6),
        "nets": grab(r"Connecting nets \.\.\. done \(([\d.]+)s\)"),
        "drc": grab(r"DRC completed in ([\d.]+)s"),
    }


def elapsed(s):
    parts = [float(p) for p in s.strip().split(":")]
    return sum(p * 60**i for i, p in enumerate(reversed(parts)))


def summarise(name, d):
    rules = d["rules"]
    rule_wall = sum(r[2] for r in rules)
    rule_cpu = sum(r[3] for r in rules)
    cpu = (d["user"] or 0) + (d["sys"] or 0)
    wall = d["wall"] or d["drc"] or 0
    cores = cpu / wall if wall else 0
    starved = [r for r in rules if r[2] >= 0.5 and r[3] / r[2] < 2]
    slowest = sorted(rules, key=lambda r: -r[2])[:5]
    return {
        "name": name,
        "wall": wall,
        "cpu": cpu,
        "cores": cores,
        "rss": d["rss_gb"] or 0,
        "nets": d["nets"] or 0,
        "rules": d["phases"].get("rules", (rule_wall, 0))[0],
        "between": d["plan"],
        "starved": starved,
        "slowest": slowest,
        "n": len(rules),
    }


def text(s):
    print(f"--- performance {s['name']}")
    print(
        f"    wall {s['wall']:.1f}s  cpu {s['cpu']:.0f}s  cores {s['cores']:.1f}  "
        f"peak rss {s['rss']:.1f} GB  nets {s['nets']:.1f}s  "
        f"rules {s['rules']:.1f}s over {s['n']}  between rules {s['between']:.1f}s"
    )
    print("    slowest rules:")
    for rid, check, w, c in s["slowest"]:
        print(f"      {rid:16} {check:16} {w:6.1f}s  cores {c / w if w else 0:5.1f}")
    if s["starved"]:
        print(f"    under 2 cores for 0.5 s or more: {len(s['starved'])} rule(s), "
              f"{sum(r[2] for r in s['starved']):.1f}s: "
              + ", ".join(r[0] for r in s["starved"][:8]))


def markdown(s):
    print(f"| {s['name']} | {s['wall']:.1f} | {s['cpu']:.0f} | {s['cores']:.1f} | "
          f"{s['rss']:.1f} | {s['nets']:.1f} | {s['rules']:.1f} | {s['between']:.1f} |")
    print()
    print("<details><summary>slowest rules</summary>")
    print()
    print("| rule | check | wall s | cores |")
    print("| --- | --- | ---: | ---: |")
    for rid, check, w, c in s["slowest"]:
        print(f"| {rid} | {check} | {w:.1f} | {c / w if w else 0:.1f} |")
    print()
    print("</details>")
    print()


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) < 2:
        sys.exit(__doc__)
    s = summarise(args[0], load(args[1:]))
    (markdown if "--markdown" in sys.argv else text)(s)
