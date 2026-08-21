#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Compare a gdscheck report against a reference KLayout .lyrdb, by marker location.

Usage:  tools/compare-lyrdb.py <golden.lyrdb> <ours.lyrdb> [-v]

For porting a foundry deck: run gdscheck over the PDK's own test case, then diff the
result against the .lyrdb the foundry ships beside it.

Marker *counts* are not comparable between the two engines. KLayout reports a width or
space violation as an edge-pair spanning the offending gap and an enclosure violation as
the protruding polygon; gdscheck reports a single edge per violating pair. One physical
violation therefore lands as one marker on one side and several on the other, and a
count diff says nothing about correctness.

So compare extent instead: a golden marker counts as covered when any of ours for the
same rule has a bounding box within TOL of it. That answers "did we flag this spot",
which is the question that matters. Read the two columns differently:

  missed  a violation the reference found and we did not - a false clean, the
          serious direction. Every one needs an explanation.
  extra   something we report that the reference does not - a false positive.
          Cheaper, but it still costs a deck author time.

Each miss is attributed to the shape of the marker, because the useful distinction when
porting is "our rule is wrong" versus "our engine cannot see this shape":

  45deg   the marker rides a non-axis-aligned segment. KLayout measures width, space
          and enclosure with the euclidian metric, which sees corner-to-corner
          distances; gdscheck's scans project onto facing axis-aligned edges.
  pinch   a zero-extent marker - a polygon touching itself at a single vertex, so
          there is no facing edge pair to find.
  ortho   plain axis-aligned geometry we should have caught. These are the ones that
          point at the deck rather than the engine.
"""

import collections
import re
import sys

TOL = 0.05  # um of slack allowed between the two bounding boxes


def markers(path):
    """Rule id -> [(bbox, is_diagonal, is_degenerate)] for every marker in the file."""
    out = collections.defaultdict(list)
    text = open(path).read()
    for m in re.finditer(
        r"<category>'?([^<']+)'?</category>.*?<values>(.*?)</values>", text, re.S
    ):
        rule, body = m.group(1), m.group(2)
        # Geometry only: a gdscheck marker also carries a human-readable `text:` value
        # whose numbers would otherwise be read as coordinates.
        geo = " ".join(re.findall(r"(?:edge-pair|edge|polygon|box):([^<]*)", body))
        nums = [float(x) for x in re.findall(r"-?\d+\.?\d*", geo)]
        if len(nums) < 4:
            continue
        xs, ys = nums[0::2], nums[1::2]
        pts = list(zip(xs, ys))
        diagonal = any(
            abs(a[0] - b[0]) > 1e-9 and abs(a[1] - b[1]) > 1e-9
            for a, b in zip(pts, pts[1:])
        )
        bbox = (min(xs), min(ys), max(xs), max(ys))
        degenerate = (bbox[2] - bbox[0]) < 1e-9 or (bbox[3] - bbox[1]) < 1e-9
        out[rule].append((bbox, diagonal, degenerate))
    return out


def close(a, b):
    """True if the two bounding boxes are within TOL of each other on both axes."""
    return (
        a[0] - TOL <= b[2]
        and b[0] - TOL <= a[2]
        and a[1] - TOL <= b[3]
        and b[1] - TOL <= a[3]
    )


def cause(diagonal, degenerate):
    if diagonal:
        return "45deg"
    return "pinch" if degenerate else "ortho"


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    verbose = "-v" in sys.argv
    if len(args) != 2:
        sys.exit(__doc__.strip().splitlines()[2])

    gold, ours = markers(args[0]), markers(args[1])
    totals = collections.Counter()
    missed_total = extra_total = 0

    print(f"{'rule':16}{'gold':>6}{'ours':>6}{'missed':>8}{'extra':>7}  {'why missed':>22}")
    for rule in sorted(set(gold) | set(ours)):
        g, o = gold.get(rule, []), ours.get(rule, [])
        obox = [b for b, _, _ in o]
        gbox = [b for b, _, _ in g]
        missed = [(b, d, z) for b, d, z in g if not any(close(b, q) for q in obox)]
        extra = [(b, d, z) for b, d, z in o if not any(close(b, q) for q in gbox)]
        missed_total += len(missed)
        extra_total += len(extra)

        by_cause = collections.Counter(cause(d, z) for _, d, z in missed)
        totals.update(by_cause)
        why = " ".join(f"{k}:{v}" for k, v in sorted(by_cause.items())) or "-"
        print(f"{rule:16}{len(g):>6}{len(o):>6}{len(missed):>8}{len(extra):>7}  {why:>22}")

        if verbose:
            for b, d, z in missed[:5]:
                print(
                    f"     missed [{cause(d, z):5}] "
                    f"({b[0]:.3f},{b[1]:.3f})-({b[2]:.3f},{b[3]:.3f})"
                )
            for b, _, _ in extra[:5]:
                print(f"     extra          ({b[0]:.3f},{b[1]:.3f})-({b[2]:.3f},{b[3]:.3f})")

    print(f"\n{missed_total} golden markers unmatched, {extra_total} of ours unmatched")
    if totals:
        print("misses by cause: " + ", ".join(f"{k} {v}" for k, v in totals.most_common()))
    # A miss is a false clean; an extra is a false positive. Neither is fatal to a
    # port in progress, so report rather than exit non-zero.


if __name__ == "__main__":
    main()
