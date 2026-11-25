import re
import unittest
from pathlib import Path

from tools import parity_gaps as pg

README = Path(__file__).resolve().parents[1] / "docs" / "status.md"


def lists(*opens):
    return [{"number": 3512 + i, "area": f"Area {i}", "open": n, "closed": 1} for i, n in enumerate(opens)]


class CountTests(unittest.TestCase):
    def test_rows(self):
        body = "Intro\n\n- [ ] `A.b` (missing): x\n- [x] `C.d` (gap): y\n- [X] `E.f`\n* [ ] `G.h`\n  - [ ] nested\n"
        self.assertEqual(pg.rows(body), (3, 2))
        self.assertEqual(pg.rows(None), (0, 0))

    def test_area_from_the_title(self):
        self.assertEqual(pg.area("CRUCIAL: Player-visible port gaps: Blocks, redstone and fluids (86)"),
                         "Blocks, redstone and fluids")


class ReadmeTests(unittest.TestCase):
    def test_block_round_trip(self):
        block = pg.block(lists(5, 20, 0))
        self.assertIn("**Player-visible gaps: 25 open** of the 28 ", block)
        self.assertLess(block.index("Area 1 [20]"), block.index("Area 0 [5]"))  # the biggest first
        self.assertEqual(pg.written(block), (25, [20, 5, 0]))

    def test_rewrite_replaces_the_block_and_the_badge(self):
        text = ("<p align=\"center\">\n  <img src=\"https://img.shields.io/badge/other-1-F0795A\" alt=\"x\">\n</p>\n\n"
                "# Horizon\n\n## Project status\n\nThe table.\n")
        once = pg.rewrite(text, pg.block(lists(7)), 7)
        self.assertIn("player--visible%20gaps-7%20open-", once)
        self.assertLess(once.index("badge/other-1"), once.index("player--visible%20gaps-7"))
        self.assertLess(once.index("player--visible%20gaps-7"), once.index("</p>\n\n# Horizon"))
        self.assertIn("## Project status\n\n**Player-visible gaps: 7 open**", once)
        twice = pg.rewrite(once, pg.block(lists(3)), 3)
        self.assertEqual(twice.count("**Player-visible gaps:"), 1)
        self.assertIn("player--visible%20gaps-3%20open-", twice)
        self.assertEqual(pg.written(twice)[0], 3)

    def test_rewrite_adds_a_badge_without_the_canonical_title(self):
        text = ("# Status page\n\n## Project status\n\n"
                "**Player-visible gaps: 42 open** of the gaps.\n\nSome prose.\n")
        out = pg.rewrite(text, "**Player-visible gaps: 5 open** of the gaps.", 5)
        self.assertIn("**Player-visible gaps: 5 open**", out)
        self.assertIn("player--visible%20gaps-5%20open-", out)
        self.assertIn("img.shields.io/badge", out)
        self.assertLess(out.index("img.shields.io/badge"), out.index("## Project status"))
        self.assertEqual(pg.written(out)[0], 5)

    def test_rewrite_fails_loudly_when_no_badge_placement_exists(self):
        text = "Just some prose with no status section.\n"
        with self.assertRaises(ValueError) as ctx:
            pg.rewrite(text, "**Player-visible gaps: 5 open** of the gaps.", 5)
        self.assertIn("could not place the player-visible gaps badge", str(ctx.exception))



if __name__ == "__main__":
    unittest.main()
