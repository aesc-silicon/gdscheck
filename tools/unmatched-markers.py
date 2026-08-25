#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Per rule: how many of OUR markers land where the reference has nothing.

compare-lyrdb.py clusters markers by proximity and asks "did we find this violation",
which is the right question for coverage and the wrong one for false positives. Two
engines that agree on every marker can still cluster them differently: on GF180's
DF.4d_LV the reference emits 22 markers and we emit 6, every one of ours sitting on a
wall the reference also flags, and the clustering still reports two "extras" because the
22 group into 2 clusters and the 6 into 3.

This asks the direct question instead - does any vertex of this marker of ours fall
inside some reference marker's extent - so a rule only scores when we flag somewhere the
reference does not. The extent test matters: the two engines mark different things for
the same violation (an edge pair here, the whole offending region there), so comparing
vertex to vertex would call agreement a disagreement.

Neither view is the truth on its own. This one over-reports wherever we mark a point on
a region and the reference marks something else entirely; the clustered view
over-reports wherever markers are dense. Read them together, and look at the geometry
before believing either.

Usage: tools/unmatched-markers.py <golden.lyrdb> <ours.lyrdb> [tolerance-um]
"""
import re, sys, collections

def load(path):
    d = collections.defaultdict(list)
    for m in re.finditer(r'<item>(.*?)</item>', open(path).read(), re.S):
        b = m.group(1)
        c = re.search(r"<category>'?([A-Za-z0-9_.]+)'?</category>", b)
        v = re.search(r'<values>(.*?)</values>', b, re.S)
        if not c or not v:
            continue
        pts = [(float(x), float(y))
               for x, y in re.findall(r'([-\d.]+),([-\d.]+)', re.sub(r'\s+', ' ', v.group(1)))]
        if pts:
            d[c.group(1)].append(pts)
    return d

def near(pts, refboxes, tol):
    # A marker matches if any of its vertices falls inside some reference marker's
    # extent, grown by tol. The two engines mark different things for the same
    # violation - an edge pair here, the whole offending region there - so comparing
    # vertex to vertex would call agreement a disagreement.
    for x, y in pts:
        for x0, y0, x1, y1 in refboxes:
            if x0 - tol <= x <= x1 + tol and y0 - tol <= y <= y1 + tol:
                return True
    return False

ref, ours = load(sys.argv[1]), load(sys.argv[2])
tol = float(sys.argv[3]) if len(sys.argv) > 3 else 0.5
total = 0
print(f"{'rule':22} {'ours':>6} {'unmatched':>10}")
for rule in sorted(ours):
    boxes = [(min(p[0] for p in poly), min(p[1] for p in poly),
              max(p[0] for p in poly), max(p[1] for p in poly))
             for poly in ref.get(rule, [])]
    bad = [poly for poly in ours[rule] if not near(poly, boxes, tol)]
    if bad:
        total += len(bad)
        print(f"{rule:22} {len(ours[rule]):>6} {len(bad):>10}")
        for poly in bad[:3]:
            x, y = poly[0]
            print(f"{'':22} {'':>6}   at ({x:.3f}, {y:.3f})")
print(f"\n{total} of our markers have no reference marker within {tol} um.")
