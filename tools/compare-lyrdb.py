#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Compare a gdscheck report against a reference KLayout .lyrdb, by marker location.

Usage:  tools/compare-lyrdb.py <golden.lyrdb> <ours.lyrdb> [-v]

For porting a foundry deck: run gdscheck over the PDK's own test case, then diff the
result against the .lyrdb the foundry ships beside it.

The two engines report at different granularity, and matching KLayout's violation
*count* is explicitly not the goal - matching the set of *logical* violations is.

gdscheck's spacing engine walks region pairs and emits one violation at their closest
approach; KLayout emits one edge-pair per facing side under the limit. Two interlocking
combs are one violation here and four there. Both point at the same structure, and one
marker per structure is the intended behaviour, not a shortfall.

So the table below counts VIOLATIONS: markers clustered by proximity, so everything
belonging to one violating structure collapses to a single entry on both sides. Read the
two columns differently:

  missed  a violation the reference found and we did not - a false clean, the
          serious direction. Every one needs an explanation.
  extra   a violation we report that the reference does not - a false positive.
          Cheaper, but it still costs a deck author time.

The raw marker tallies are printed underneath for reference only. A marker shortfall
against the reference is expected and means nothing on its own.

The cause column below describes the *shape* of the marker that went unmatched. It is a
hint about where to look, not a diagnosis - reading it as one led this port down a wrong
path once already (a "45deg" column was blamed on the euclidian metric, when the actual
defect was min_width not pairing an oblique edge with an axis-aligned one, and spacing
was euclidian all along).

  45deg   the marker rides a non-axis-aligned segment.
  pinch   a zero-extent marker - typically a polygon pinching to zero width where it
          touches itself, or two shapes meeting at one vertex. Touching shapes merge
          into a single region here, so there is no facing pair left to measure.
  ortho   plain axis-aligned geometry.

To inspect a spot on a hierarchical fixture, flatten it first - most of the gf180mcu
static cases are hierarchical, so raw GDS coordinates are cell-local and will not line
up with a marker position:

  klayout -b -r flatten.rb -rd src=in.gds -rd dst=flat.gds -rd top=<cell>
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


def clusters(items, radius=8.0):
    """Group markers into logical violations by single-link proximity clustering.

    `items` is [(centre, payload)]; returns [(centre, [payload, ...])] so a cluster can
    still describe the markers it came from. The radius is a device-scale span - the
    markers KLayout scatters around one structure sit well inside it, and two genuinely
    separate violations sit well outside."""
    items = list(items)
    seen = [False] * len(items)
    out = []
    for i in range(len(items)):
        if seen[i]:
            continue
        stack, group = [i], []
        seen[i] = True
        while stack:
            k = stack.pop()
            group.append(items[k])
            for j in range(len(items)):
                if (not seen[j]
                        and abs(items[k][0][0] - items[j][0][0]) <= radius
                        and abs(items[k][0][1] - items[j][0][1]) <= radius):
                    seen[j] = True
                    stack.append(j)
        cx = sum(p[0][0] for p in group) / len(group)
        cy = sum(p[0][1] for p in group) / len(group)
        out.append(((cx, cy), [p[1] for p in group]))
    return out


def centre(box):
    return ((box[0] + box[2]) / 2, (box[1] + box[3]) / 2)


def cause(diagonal, degenerate):
    if diagonal:
        return "45deg"
    return "pinch" if degenerate else "ortho"


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    verbose = "-v" in sys.argv
    if len(args) != 2:
        sys.exit("usage: compare-lyrdb.py <golden.lyrdb> <ours.lyrdb> [-v]")

    gold, ours = markers(args[0]), markers(args[1])
    RADIUS = 8.0

    print(f"{'rule':16}{'ref':>5}{'ours':>6}{'missed':>8}{'extra':>7}"
          f"   {'markers ref/ours':>17}  {'shape of misses':>18}")
    g_tot = o_tot = m_tot = e_tot = 0
    causes = collections.Counter()

    for rule in sorted(set(gold) | set(ours)):
        g, o = gold.get(rule, []), ours.get(rule, [])
        # Cluster to logical violations, carrying each cluster's marker shapes along so a
        # missed one can still be described.
        gc = clusters([(centre(b), (d, z)) for b, d, z in g], RADIUS)
        oc = clusters([(centre(b), (d, z)) for b, d, z in o], RADIUS)
        near = lambda p, q: abs(p[0] - q[0]) <= RADIUS and abs(p[1] - q[1]) <= RADIUS

        missed = [c for c in gc if not any(near(c[0], k[0]) for k in oc)]
        extra = [c for c in oc if not any(near(c[0], k[0]) for k in gc)]
        g_tot += len(gc)
        o_tot += len(oc)
        m_tot += len(missed)
        e_tot += len(extra)

        by_cause = collections.Counter(cause(*t) for c in missed for t in c[1])
        causes.update(by_cause)
        why = " ".join(f"{k}:{v}" for k, v in sorted(by_cause.items())) or "-"
        print(f"{rule:16}{len(gc):>5}{len(oc):>6}{len(missed):>8}{len(extra):>7}"
              f"   {len(g):>7}/{len(o):<9}{why:>18}")

        if verbose:
            for c in missed[:5]:
                print(f"     missed at ({c[0][0]:.3f}, {c[0][1]:.3f})")
            for c in extra[:5]:
                print(f"     extra  at ({c[0][0]:.3f}, {c[0][1]:.3f})")

    print(f"\n{g_tot} logical violations in the reference; we flag {o_tot}.")
    print(f"{m_tot} missed (false clean), {e_tot} extra (false positive).")
    if causes:
        print("shape of the missed markers: "
              + ", ".join(f"{k} {v}" for k, v in causes.most_common())
              + "   (a hint about where to look, not a cause)")
    # A miss is a false clean; an extra is a false positive. Neither is fatal to a port
    # in progress, so report rather than exit non-zero.


if __name__ == "__main__":
    main()
