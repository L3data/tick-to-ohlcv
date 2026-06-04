from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from tick_to_ohlcv.intervals import parse_interval_seconds


class IntervalParsingTests(unittest.TestCase):
    def test_parses_seconds_minutes_hours_and_days(self):
        self.assertEqual(parse_interval_seconds("60"), 60)
        self.assertEqual(parse_interval_seconds("15s"), 15)
        self.assertEqual(parse_interval_seconds("5m"), 300)
        self.assertEqual(parse_interval_seconds("2h"), 7_200)
        self.assertEqual(parse_interval_seconds("1d"), 86_400)

    def test_rejects_invalid_interval(self):
        with self.assertRaises(ValueError):
            parse_interval_seconds("m1")
        with self.assertRaises(ValueError):
            parse_interval_seconds("0m")
        with self.assertRaises(ValueError):
            parse_interval_seconds("1w")


if __name__ == "__main__":
    unittest.main()
