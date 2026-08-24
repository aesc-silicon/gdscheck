#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 aesc silicon
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Generate the Metal1 slotting test pattern for the gf180mcuD `mslot` deck.

The PDK ships no unit test case for metal slotting, so this writes one: nine 40x40 um
Metal1 plates on a 100 um pitch, each isolating a single MSLOT1 rule.  Every plate is
wide enough that the deck's "needs a slot" derivation would keep it, so a plate that
should *not* report MSLOT1.1 proves the slot broke the 30 um opening.

    A  no slot at all                        -> MSLOT1.1
    B  one legal 2 x 20 um slot              -> clean
    C  slot 1 um wide and 5 um long          -> MSLOT1.2, MSLOT1.3
    D  two slot marks merging into an L      -> MSLOT1.0
    E  two legal slots 5 um apart            -> MSLOT1.4
    F  slot 5 um from the metal's left edge  -> MSLOT1.5
    G  contact 0.1 um from the slot          -> MSLOT1.8
    H  via1 0.1 um from the slot             -> MSLOT1.7
    I  a pad-over-top-metal keep-out nearby  -> MSLOT1.9

Usage: tools/gen-mslot-pattern.py tests/data/gf180mcuD/generated/mslot.gds.gz
"""

import gzip
import struct
import sys

DBU = 1e-9  # 1 nm database unit, as the GF180 test cases use
UU = 1e-6  # user unit is 1 um
UM = 1000  # DBU per um

CELL = "MSLOT_PATTERN"

METAL1, METAL1_DT = 34, 0
SLOT, SLOT_DT = 34, 3
CONTACT, VIA1, PAD, METAL5 = 33, 35, 37, 81


def um(v):
    return int(round(v * UM))


def rect(x0, y0, x1, y1):
    """A closed axis-aligned boundary, in um."""
    x0, y0, x1, y1 = um(x0), um(y0), um(x1), um(y1)
    return [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]


def shapes():
    """(layer, datatype, points) for the whole pattern."""
    out = []
    plate = lambda i: 100.0 * i  # noqa: E731 - plate origin, 100 um pitch
    for i in range(9):
        x = plate(i)
        w = 35.0 if i == 5 else 40.0  # F is narrower, see below
        out.append((METAL1, METAL1_DT, rect(x, 0, x + w, 40)))

    # B: a legal slot, 2 x 20 um, 10 um clear of every metal edge.
    out.append((SLOT, SLOT_DT, rect(119, 10, 121, 30)))
    # C: too narrow (1 um) and too short (5 um).
    out.append((SLOT, SLOT_DT, rect(219, 10, 220, 15)))
    # D: two marks that merge into an L, which is not a rectangle.
    out.append((SLOT, SLOT_DT, rect(310, 10, 315, 30)))
    out.append((SLOT, SLOT_DT, rect(315, 10, 330, 15)))
    # E: two legal slots only 5 um apart.
    out.append((SLOT, SLOT_DT, rect(410, 10, 412, 30)))
    out.append((SLOT, SLOT_DT, rect(417, 10, 419, 30)))
    # F: 5 um from the plate's left edge.  The plate is 35 um wide so that the 28 um
    # of metal left to the slot's right cannot hold a 30 um square, keeping MSLOT1.1
    # quiet and the case about the enclosure alone.
    out.append((SLOT, SLOT_DT, rect(505, 10, 507, 30)))
    # G, H: a legal slot with a via 0.1 um away, below and above the metal.
    out.append((SLOT, SLOT_DT, rect(615, 10, 617, 30)))
    out.append((CONTACT, 0, rect(617.1, 15, 617.3, 15.2)))
    out.append((SLOT, SLOT_DT, rect(715, 10, 717, 30)))
    out.append((VIA1, 0, rect(717.1, 15, 717.3, 15.2)))
    # I: pad over top metal is a slot keep-out, grown by 5 um.  The overlap sits 6 um
    # from the slot, so the grown keep-out lands 1 um away - inside the 5 um rule.
    out.append((SLOT, SLOT_DT, rect(815, 10, 817, 30)))
    out.append((METAL5, 0, rect(823, 15, 833, 25)))
    out.append((PAD, 0, rect(823, 15, 833, 25)))
    return out


# --- GDSII writing ---------------------------------------------------------


def rec(code, payload=b""):
    return struct.pack(">HH", len(payload) + 4, code) + payload


def i16(code, *vals):
    return rec(code, struct.pack(">" + "h" * len(vals), *vals))


def i32(code, *vals):
    return rec(code, struct.pack(">" + "i" * len(vals), *vals))


def real8(v):
    """GDSII 8-byte excess-64 base-16 float."""
    if v == 0:
        return b"\x00" * 8
    sign = 0x80 if v < 0 else 0
    v = abs(v)
    exp = 64
    while v >= 1:
        v /= 16.0
        exp += 1
    while v < 1 / 16.0:
        v *= 16.0
        exp -= 1
    mant = int(v * (1 << 56))
    return struct.pack(">B", sign | exp) + struct.pack(">Q", mant)[1:]


def name_rec(code, s):
    b = s.encode("ascii")
    if len(b) % 2:
        b += b"\x00"
    return rec(code, b)


def build():
    zero = (0,) * 12
    out = [i16(0x0002, 600), i16(0x0102, *zero) + b"", ]
    out[1] = rec(0x0102, struct.pack(">12h", *zero))  # BGNLIB, zeroed timestamps
    out.append(name_rec(0x0206, "MSLOT_PATTERN.DB"))  # LIBNAME
    out.append(rec(0x0305, real8(DBU / UU) + real8(DBU)))  # UNITS
    out.append(rec(0x0502, struct.pack(">12h", *zero)))  # BGNSTR
    out.append(name_rec(0x0606, CELL))  # STRNAME
    for layer, dt, pts in shapes():
        out.append(rec(0x0800))  # BOUNDARY
        out.append(i16(0x0D02, layer))  # LAYER
        out.append(i16(0x0E02, dt))  # DATATYPE
        flat = [c for p in pts for c in p]
        out.append(i32(0x1003, *flat))  # XY
        out.append(rec(0x1100))  # ENDEL
    out.append(rec(0x0700))  # ENDSTR
    out.append(rec(0x0400))  # ENDLIB
    return b"".join(out)


if __name__ == "__main__":
    path = sys.argv[1] if len(sys.argv) > 1 else "mslot.gds.gz"
    data = build()
    if path.endswith(".gz"):
        with gzip.GzipFile(path, "wb", mtime=0) as f:
            f.write(data)
    else:
        open(path, "wb").write(data)
    print(f"wrote {path} ({len(data)} bytes uncompressed)")
