#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""The functions a samply profile spent the most time in, without a browser.

    samply record --save-only -o prof.json.gz target/profiling/gdscheck run ...
    ci/profile-top.py prof.json.gz [--inclusive] [-n 30]

Self time is the samples whose leaf frame is the function; inclusive time counts every
sample the function is anywhere on the stack of.  Frames from the standard library and
the allocator are folded into their caller, so a function that allocates a lot is
charged for it.
"""

import gzip
import json
import sys
from collections import Counter


def load(path):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as f:
        return json.load(f)


def symbolicate(prof):
    """Resolve each frame's address through addr2line against the library it lies in.
    samply names frames only when the browser asks; saved to a file, they are addresses
    in the library's `resourceTable` entry."""
    import subprocess

    libs = prof["libs"]
    for thread in prof["threads"]:
        strings = thread["stringArray"] if "stringArray" in thread else thread["stringTable"]
        funcs = thread["funcTable"]
        frames = thread["frameTable"]
        resources = thread["resourceTable"]
        by_lib = {}
        for fi in range(frames["length"]):
            func = frames["func"][fi]
            res = funcs["resource"][func]
            if res is None or res < 0:
                continue
            lib = resources["lib"][res]
            if lib is None:
                continue
            by_lib.setdefault(lib, set()).add((frames["address"][fi], func))
        for lib, pairs in by_lib.items():
            path = libs[lib]["path"]
            addrs = sorted({a for a, _ in pairs})
            try:
                out = subprocess.run(
                    ["addr2line", "-f", "-C", "-e", path] + [hex(a) for a in addrs],
                    capture_output=True, text=True, check=True,
                ).stdout.split("\n")
            except (OSError, subprocess.CalledProcessError):
                continue
            # Two lines per address: the function, then its file and line.
            names = {a: out[2 * i] for i, a in enumerate(addrs) if 2 * i < len(out)}
            for a, func in pairs:
                name = names.get(a, "??")
                if name and name != "??":
                    strings.append(name)
                    funcs["name"][func] = len(strings) - 1


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    inclusive = "--inclusive" in sys.argv
    n = 30
    if "-n" in sys.argv:
        n = int(sys.argv[sys.argv.index("-n") + 1])
    prof = load(args[0])
    symbolicate(prof)
    self_t = Counter()
    incl_t = Counter()
    total = 0
    folded = ("core::", "alloc::", "std::", "__", "_int_", "malloc", "free", "memcpy", "memmove")
    for thread in prof["threads"]:
        strings = thread["stringArray"] if "stringArray" in thread else thread["stringTable"]
        frames = thread["frameTable"]
        stacks = thread["stackTable"]
        samples = thread["samples"]
        funcs = thread["funcTable"]
        fname = funcs["name"]
        ffunc = frames["func"]
        sprefix = stacks["prefix"]
        sframe = stacks["frame"]
        weights = samples.get("weight") or [1] * len(samples["stack"])
        for stack, w in zip(samples["stack"], weights):
            if stack is None:
                continue
            total += w
            seen = set()
            leaf = None
            s = stack
            while s is not None:
                name = strings[fname[ffunc[sframe[s]]]]
                if not name.startswith(folded):
                    if leaf is None:
                        leaf = name
                    if name not in seen:
                        seen.add(name)
                        incl_t[name] += w
                s = sprefix[s]
            if leaf is not None:
                self_t[leaf] += w
    table = incl_t if inclusive else self_t
    print(f"{'inclusive' if inclusive else 'self'} time, {total} samples over {len(prof['threads'])} threads")
    for name, w in table.most_common(n):
        print(f"  {100.0 * w / max(total, 1):5.1f}%  {name[:110]}")


if __name__ == "__main__":
    main()
