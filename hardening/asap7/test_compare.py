# SPDX-FileCopyrightText: 2026 aesc silicon
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Regression checks for false agreement in the comparison machinery."""

import tempfile
from pathlib import Path
import sys
import unittest

from compare import canonical, compare, read_report, run, signature


def marker(box, rule="FIN.W.2"):
    return {"rule": rule, "bbox_um": box}


class ComparisonTest(unittest.TestCase):
    def test_two_walls_match_one_edge_pair(self):
        gc = [marker([0, 0, 0, 0.01]), marker([0.054, 0, 0.054, 0.01])]
        kl = [marker([0, 0, 0.054, 0.01])]
        (row,) = compare(gc, kl)
        self.assertEqual(row["note"], "count-difference")
        self.assertEqual(row["unmatched_gdscheck"], 0)
        self.assertEqual(row["unmatched_klayout"], 0)

    def test_equal_counts_at_different_locations_do_not_agree(self):
        (row,) = compare([marker([0, 0, 1, 1])], [marker([2, 0, 3, 1])])
        self.assertEqual(row["note"], "location-difference")
        self.assertEqual(row["unmatched_gdscheck"], 1)
        self.assertEqual(row["unmatched_klayout"], 1)

    def test_tile_comparison_checks_geometry_and_multiplicity(self):
        a = marker([0, 0, 1, 1])
        b = marker([1, 0, 2, 1])
        self.assertNotEqual(signature([a]), signature([b]))
        self.assertNotEqual(signature([a]), signature([a, a]))
        self.assertEqual(signature([a, b]), signature([b, a]))

    def test_explicit_aliases_preserve_unrelated_rules(self):
        self.assertEqual(
            canonical("'LIG.SDT.S.8LIG.SDT.S.8 : spacing'", "", "klayout"),
            "LIG.SDT.S.8",
        )
        self.assertEqual(
            canonical(
                "'GATE.S.2'", "Every gate needs at least one other GATE", "klayout"
            ),
            "GATE.S.3",
        )
        self.assertEqual(canonical("GATE.S.2", "", "gdscheck"), "GATE.S.2")
        self.assertEqual(
            canonical("'SRAM.ACTIVE.A.1A'", "", "klayout"), "SRAM.ACTIVE.A.2A"
        )
        self.assertEqual(canonical("'V4.S.1-2-3'", "", "klayout"), "V4.S.2")
        self.assertEqual(canonical("'UNKNOWN.RULE'", "", "klayout"), "UNKNOWN.RULE")

    def report(self, body):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "test.lyrdb"
            path.write_text(body)
            return read_report(path, "klayout", "TOP")

    def test_hierarchical_or_unmeasured_items_are_not_silently_clean(self):
        for item in [
            "<cell>CHILD</cell><values><value>edge:(0,0;1,1)</value></values>",
            "<cell>TOP</cell><tags>skipped</tags>",
            "<cell>TOP</cell><multiplicity>2</multiplicity>",
            "<cell>TOP</cell><values><value>text: not checked</value></values>",
        ]:
            with self.subTest(item=item), self.assertRaises(ValueError):
                self.report(
                    f"<report-database><items><item><category>'FIN.W.2'</category>{item}</item></items></report-database>"
                )
        with self.assertRaises(ValueError):
            self.report("<report-database/>")

    def test_polygon_holes_and_negative_coordinates(self):
        result = self.report("""<report-database><items><item>
          <cell>TOP</cell><category>'M8.A.1'</category><values>
          <value>polygon: (-1e-3,0;0.2,0;0.2,0.5/-0.0005,0.1;0.1,0.1)</value>
          <value>text: 'distance 100,200 is not geometry'</value>
          </values></item></items></report-database>""")
        self.assertEqual(result[0]["bbox_um"], [-0.001, 0, 0.2, 0.5])

    def test_failed_checker_is_not_treated_as_clean(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "run.log"
            with self.assertRaises(RuntimeError):
                run([sys.executable, "-c", "raise SystemExit(1)"], log)
            # gdscheck uses status 2 for violations; the caller must explicitly
            # allow it, then still require a valid report.
            with self.assertRaises(RuntimeError):
                run([sys.executable, "-c", "raise SystemExit(2)"], log)
            run([sys.executable, "-c", "raise SystemExit(2)"], log, allowed=(0, 2))


if __name__ == "__main__":
    unittest.main()
