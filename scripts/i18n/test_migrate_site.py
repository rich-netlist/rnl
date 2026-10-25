"""Tests of scripts/i18n/migrate_site.py: python -m unittest discover -s scripts/i18n -p 'test_*.py'"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import migrate_site  # noqa: E402


class Plan(unittest.TestCase):
    def test_blocks_move_to_the_catalog_and_the_english_stays(self):
        text = '<p><zh>你好</zh>\n  <en>Hello  there</en></p><p><en>Only English</en></p><b><zh></zh><en>Gloss</en></b>'
        new, added = migrate_site.plan("page.html", text, {"zh": {}})
        self.assertEqual(new, '<p><en>Hello  there</en></p><p><en>Only English</en></p><b><en>Gloss</en></b>')
        self.assertEqual(added, {"zh": {"Hello there": "你好", "Gloss": ""}})
        self.assertEqual(migrate_site.plan("page.html", new, {"zh": added["zh"]}), (new, {}))  # again: nothing

    def test_two_ways_of_one_english_name_a_context(self):
        text = '<zh>是</zh><en>Yes</en>|<zh>看得懂</zh><en>Yes</en>|<zh>是</zh><en>Yes</en>|<zh>Host adapter</zh><en></en>'
        new, added = migrate_site.plan("index.html", text, {"zh": {}})
        self.assertEqual(new, '<en>Yes</en>|<en ctx=readable>Yes</en>|<en>Yes</en>|<en ctx=host-adapter></en>')
        self.assertEqual(added["zh"], {"Yes": "是", "readable|Yes": "看得懂", "host-adapter|": "Host adapter"})

    def test_an_unnamed_way_or_a_catalog_that_differs_stops(self):
        with self.assertRaises(migrate_site.Stop):
            migrate_site.plan("other.html", '<zh>甲</zh><en>A</en><zh>乙</zh><en>A</en>', {"zh": {}})
        with self.assertRaises(migrate_site.Stop):
            migrate_site.plan("other.html", '<zh>注</zh><en></en>', {"zh": {}})  # not English words: no context
        with self.assertRaises(migrate_site.Stop):
            migrate_site.plan("other.html", '<zh>甲</zh><en>A</en>', {"zh": {"A": "乙"}})


if __name__ == "__main__":
    unittest.main()
