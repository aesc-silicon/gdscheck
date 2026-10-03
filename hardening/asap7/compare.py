#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 aesc silicon
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Compare the pinned ASAP7 KLayout port with gdscheck; Python stdlib only."""

import argparse
from collections import Counter, defaultdict
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests/data/asap7"
REVISION = "ef77f080381cab3993428492a3dcb41044c3f518"
# Hash the original macro bytes as well as checking HEAD: local edits must not
# silently change the reference. No macro patches or rule suppression are used.
DECK_SHA256 = "b614f48ffebf07e371b0b9bdc7abfb21ae09a206b29e9e9149ea2740ff998cf5"
TOLERANCE_UM = 0.00025  # one input DBU; accommodates gdscheck's report rounding
NUMBER = r"[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?"
POINT = re.compile(rf"({NUMBER})\s*,\s*({NUMBER})")


def canonical(category, description, engine):
    """Explicit aliases for this revision only; retain raw labels in JSON."""
    rule = category.strip("'")
    # Missing commas in the macro concatenate the ID and description.
    rule = re.sub(r"^([A-Z][A-Z0-9_.-]+?)\1\s*:.*$", r"\1", rule)
    if engine == "klayout":
        if rule == "GATE.S.2" and "at least one other GATE" in description:
            return "GATE.S.3"
        if rule == "GATE.S.3" and "Min. horizontal spacing" in description:
            return "GATE.S.2"
        rule = {
            "SRAM.ACTIVE.A.1A": "SRAM.ACTIVE.A.2A",
            "SRAM.ACTIVE.A.1B": "SRAM.ACTIVE.A.2B",
            "ACTIVE.AUX.1_edges": "ACTIVE.AUX.1",
            **{f"V{n}.S.1-2-3": f"V{n}.S.2" for n in range(4, 8)},
        }.get(rule, rule)
    # Grouped checks cannot honestly be split into their constituent rules.
    if rule in ("LIG.S.4", "LIG.S.5"):
        return "LIG.S.4-5"
    return rule


def read_report(path, engine, top):
    root = ET.parse(path).getroot()
    if root.tag != "report-database":
        raise ValueError(f"Not a report database: {path}")
    descriptions = {}

    def categories(node, prefix=""):
        for cat in node.findall("categories/category"):
            name = prefix + cat.findtext("name", "")
            descriptions[name] = cat.findtext("description", "")
            categories(cat, name + ".")

    categories(root)
    if root.find("items") is None:
        raise ValueError(f"Missing items in {path}")
    markers = []
    for item in root.findall("items/item"):
        if item.findtext("tags", "").strip():
            raise ValueError(
                f"Tagged/skipped/waived item in {path}; comparison is incomplete"
            )
        if item.findtext("cell") != top:
            raise ValueError(
                f"Non-flat marker in {path}; refusing to compare local coordinates"
            )
        if int(item.findtext("multiplicity", "1")) != 1:
            raise ValueError(
                f"Unexpected marker multiplicity after flattening in {path}"
            )
        category = item.findtext("category", "")
        if not category:
            raise ValueError(f"Uncategorized marker in {path}")
        desc = descriptions.get(category.strip("'"), "")
        values = [v.text or "" for v in item.findall("values/value")]
        points = []
        for value in values:
            if value.split(":", 1)[0].strip() in (
                "edge",
                "edge-pair",
                "polygon",
                "box",
            ):
                points.extend((float(x), float(y)) for x, y in POINT.findall(value))
        if not points or not all(math.isfinite(v) for p in points for v in p):
            raise ValueError(f"Missing/unsupported marker geometry in {path}: {values}")
        xs, ys = zip(*points)
        markers.append(
            {
                "rule": canonical(category, desc, engine),
                "category": category,
                "bbox_um": [min(xs), min(ys), max(xs), max(ys)],
                "values": values,
            }
        )
    return markers


def overlaps(a, b):
    return (
        a[0] <= b[2] + TOLERANCE_UM
        and b[0] <= a[2] + TOLERANCE_UM
        and a[1] <= b[3] + TOLERANCE_UM
        and b[1] <= a[3] + TOLERANCE_UM
    )


def compare(gc, kl):
    groups = [defaultdict(list), defaultdict(list)]
    for group, markers in zip(groups, (gc, kl)):
        for marker in markers:
            group[marker["rule"]].append(marker["bbox_um"])
    rows = []
    for rule in sorted(groups[0].keys() | groups[1].keys()):
        a, b = (g[rule] for g in groups)
        unmatched_a = sum(not any(overlaps(x, y) for y in b) for x in a)
        unmatched_b = sum(not any(overlaps(y, x) for x in a) for y in b)
        note = (
            "klayout-only"
            if not a
            else "gdscheck-only"
            if not b
            else "location-difference"
            if unmatched_a or unmatched_b
            else "count-difference"
            if len(a) != len(b)
            else "same-locations"
        )
        rows.append(
            {
                "rule": rule,
                "gdscheck": len(a),
                "klayout": len(b),
                "unmatched_gdscheck": unmatched_a,
                "unmatched_klayout": unmatched_b,
                "note": note,
            }
        )
    return rows


def signature(markers):
    return Counter((m["rule"], tuple(m["bbox_um"])) for m in markers)


def run(command, log, allowed=(0,)):
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen")
    with log.open("w") as output:
        result = subprocess.run(
            [str(x) for x in command],
            cwd=ROOT,
            env=env,
            stdout=output,
            stderr=subprocess.STDOUT,
        )
    if result.returncode not in allowed:
        raise RuntimeError(f"Command exited {result.returncode}; see {log}")


def corpus():
    cases = {
        "standard_cells": {
            "input": FIXTURES / "static/asap7sc7p5t_28_L.gds.gz",
            "top": "ALLCELLS",
            "expectations": [],
        },
        "sram_bank": {
            "input": FIXTURES / "static/srambank_32b.gds.gz",
            "top": "srambank_32b",
            "expectations": [],
        },
    }
    for line in (FIXTURES / "generated/cases.tsv").read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        name, rule, verdict, reason = line.split("\t")
        if verdict not in ("pass", "fail"):
            raise ValueError(f"Invalid verdict: {line}")
        case = cases.setdefault(
            name,
            {
                "input": FIXTURES / f"generated/{name}.gds.gz",
                "top": "TOP",
                "expectations": [],
            },
        )
        case["expectations"].append(
            {"rule": rule, "bad": verdict == "fail", "reason": reason}
        )
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "layout", nargs="?", type=Path, help="default: run the complete fixture corpus"
    )
    parser.add_argument("--top", default="TOP")
    parser.add_argument(
        "--case", action="append", help="select named corpus case (repeatable)"
    )
    parser.add_argument(
        "--pdk",
        type=Path,
        default=os.environ.get("ASAP7_KLAYOUT"),
        required=not bool(os.environ.get("ASAP7_KLAYOUT")),
        help="ASAP7_for_KLayout checkout",
    )
    parser.add_argument(
        "--gdscheck",
        type=Path,
        default=os.environ.get("GDSCHECK", ROOT / "target/release/gdscheck"),
    )
    parser.add_argument("--klayout", default=os.environ.get("KLAYOUT", "klayout"))
    parser.add_argument("--tiles", nargs="+", type=float, default=[20, 7])
    parser.add_argument(
        "--output",
        type=Path,
        help="new/empty artifact directory; default: persistent temporary directory",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="also fail on cross-tool discrepancies (known ones exist)",
    )
    args = parser.parse_args()
    if any(not math.isfinite(t) or t <= 0 for t in args.tiles) or len(
        set(args.tiles)
    ) != len(args.tiles):
        parser.error("tiles must be distinct positive finite numbers")
    if args.layout and args.case:
        parser.error("--case cannot be used with a layout")
    pdk = args.pdk.expanduser().resolve()
    revision = subprocess.check_output(
        ["git", "-C", str(pdk), "rev-parse", "HEAD"], text=True
    ).strip()
    deck = pdk / "drc/drc_ASAP7.lydrc"
    if (
        revision != REVISION
        or hashlib.sha256(deck.read_bytes()).hexdigest() != DECK_SHA256
    ):
        raise ValueError(
            f"Reference must be unmodified ASAP7_for_KLayout revision {REVISION}"
        )
    work = (
        args.output.resolve()
        if args.output
        else Path(tempfile.mkdtemp(prefix="asap7-parity-"))
    )
    if work.exists() and any(work.iterdir()):
        raise ValueError(f"Output directory is not empty: {work}")
    work.mkdir(parents=True, exist_ok=True)
    binary = args.gdscheck.expanduser().resolve()
    selected = (
        {
            "custom": {
                "input": args.layout.resolve(),
                "top": args.top,
                "expectations": [],
            }
        }
        if args.layout
        else corpus()
    )
    if args.case:
        unknown = set(args.case) - selected.keys()
        if unknown:
            parser.error(f"unknown cases: {sorted(unknown)}")
        selected = {k: v for k, v in selected.items() if k in args.case}
    result = {
        "reference_revision": revision,
        "reference_sha256": DECK_SHA256,
        "klayout_version": subprocess.check_output(
            [args.klayout, "-v"], text=True
        ).strip(),
        "gdscheck_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "source_revision": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip(),
        "source_dirty": bool(
            subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT)
        ),
        "tiles_um": args.tiles,
        "bbox_tolerance_um": TOLERANCE_UM,
        "cases": {},
    }
    errors, differences = [], []
    for name, case in selected.items():
        print(f"Comparing {name} ...", flush=True)
        directory = work / name
        directory.mkdir()
        flat = directory / "input.gds"
        top = case["top"]
        run(
            [
                args.klayout,
                "-b",
                "-r",
                Path(__file__).with_name("flatten.rb"),
                "-rd",
                f"input={case['input']}",
                "-rd",
                f"topcell={top}",
                "-rd",
                f"output={flat}",
            ],
            directory / "flatten.log",
        )
        kl_report = directory / "klayout.lyrdb"
        run(
            [
                args.klayout,
                "-b",
                "-r",
                deck,
                "-rd",
                f"input={flat}",
                "-rd",
                f"topcell={top}",
                "-rd",
                f"output={kl_report}",
            ],
            directory / "klayout.log",
        )
        kl = read_report(kl_report, "klayout", top)
        gc_runs = {}
        for tile in args.tiles:
            report = directory / f"gdscheck-{tile:g}.lyrdb"
            run(
                [
                    binary,
                    "run",
                    "--process",
                    "asap7",
                    "--suite",
                    "main",
                    "--input",
                    flat,
                    "--topcell",
                    top,
                    "--tile",
                    f"{tile:g}",
                    "--report",
                    report,
                ],
                directory / f"gdscheck-{tile:g}.log",
                allowed=(0, 2),
            )
            gc_runs[f"{tile:g}"] = read_report(report, "gdscheck", top)
        gc = next(iter(gc_runs.values()))
        stable = all(
            signature(gc) == signature(markers) for markers in gc_runs.values()
        )
        if not stable:
            errors.append(f"{name}: tile-dependent marker geometry/counts")
        rows = compare(gc, kl)
        checks = []
        for expected in case["expectations"]:
            rule = expected["rule"]
            outcomes = [any(m["rule"] == rule for m in markers) for markers in (gc, kl)]
            checks.append(
                dict(expected, gdscheck_bad=outcomes[0], klayout_bad=outcomes[1])
            )
            if outcomes[0] != expected["bad"]:
                errors.append(
                    f"{name}: gdscheck disagrees with manual expectation for {rule}"
                )
            if outcomes[1] != expected["bad"]:
                differences.append(
                    f"{name}: KLayout disagrees with manual expectation for {rule}"
                )
        if name == "routed_clean" and gc:
            errors.append("routed_clean: unexpected gdscheck violations")
        if any(r["unmatched_gdscheck"] or r["unmatched_klayout"] for r in rows):
            differences.append(f"{name}: unmatched rule/locations")
        result["cases"][name] = {
            "input": str(case["input"]),
            "input_sha256": hashlib.sha256(case["input"].read_bytes()).hexdigest(),
            "tile_stable": stable,
            "comparison": rows,
            "expectations": checks,
            "gdscheck": gc_runs,
            "klayout": kl,
        }
    result.update(errors=errors, differences=differences)
    (work / "comparison.json").write_text(json.dumps(result, indent=2) + "\n")
    lines = [
        "# ASAP7 checker comparison",
        "",
        f"KLayout: {result['klayout_version']}; deck: `{revision}`.",
        "",
        "Counts are markers. Location matching is many-to-many bounding-box overlap within 0.25 nm,",
        "a triage aid, not proof of equivalent violations. Only explicitly targeted cases count as exercised rules.",
        "",
    ]
    for name, data in result["cases"].items():
        lines += [
            f"## {name}",
            "",
            f"Tile stable: {data['tile_stable']}",
            "",
            "| Rule | gdscheck | KLayout | Unmatched GC / KL | Note |",
            "|---|---:|---:|---:|---|",
        ]
        for row in data["comparison"]:
            lines.append(
                f"| {row['rule']} | {row['gdscheck']} | {row['klayout']} | "
                f"{row['unmatched_gdscheck']} / {row['unmatched_klayout']} | {row['note']} |"
            )
        for check in data["expectations"]:
            lines.append(
                f"\nTarget `{check['rule']}`: manual {'fail' if check['bad'] else 'pass'}; "
                f"gdscheck {'fail' if check['gdscheck_bad'] else 'pass'}; "
                f"KLayout {'fail' if check['klayout_bad'] else 'pass'}."
            )
        lines.append("")
    checks = [e for c in result["cases"].values() for e in c["expectations"]]
    agree = sum(e["gdscheck_bad"] == e["klayout_bad"] for e in checks)
    rules = {e["rule"] for e in checks}
    summary = f"{agree}/{len(checks)} targeted verdicts agree across {len(rules)} exercised rules; not a whole-PDK parity percentage."
    lines += [
        summary,
        "",
        f"Regression errors: {len(errors)}; discrepancy notices: {len(differences)}.",
    ]
    lines += [f"- {error}" for error in errors + differences]
    (work / "comparison.md").write_text("\n".join(lines) + "\n")
    print(summary)
    print(f"Reports and logs: {work}")
    for error in errors:
        print(error, file=sys.stderr)
    return 1 if errors or (args.strict and differences) else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (
        OSError,
        ValueError,
        RuntimeError,
        ET.ParseError,
        subprocess.CalledProcessError,
    ) as error:
        print(f"ASAP7 comparison failed: {error}", file=sys.stderr)
        sys.exit(2)
