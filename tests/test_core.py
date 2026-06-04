from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from tick_to_ohlcv.core import Trade, aggregate_trades


class CoreAggregationTests(unittest.TestCase):
    def test_fill_gaps_emits_flat_zero_volume_candles(self):
        candles = list(
            aggregate_trades(
                [
                    Trade(symbol="abc", ts_ms=0, price=10.0, size=2.0),
                    Trade(symbol="abc", ts_ms=120_000, price=12.0, size=1.0),
                ],
                fill_gaps=True,
            )
        )

        self.assertEqual([candle.ts for candle in candles], [0, 60, 120])
        gap = candles[1]
        self.assertEqual(gap.symbol, "ABC")
        self.assertEqual(gap.open, 10.0)
        self.assertEqual(gap.high, 10.0)
        self.assertEqual(gap.low, 10.0)
        self.assertEqual(gap.close, 10.0)
        self.assertEqual(gap.volume, 0.0)
        self.assertEqual(gap.turnover, 0.0)
        self.assertEqual(gap.trade_count, 0)
        self.assertIsNone(gap.first_trade_ts_ms)
        self.assertIsNone(gap.last_trade_ts_ms)

    def test_fill_gaps_does_not_cross_symbols(self):
        candles = list(
            aggregate_trades(
                [
                    Trade(symbol="abc", ts_ms=0, price=10.0, size=1.0),
                    Trade(symbol="xyz", ts_ms=120_000, price=20.0, size=1.0),
                ],
                fill_gaps=True,
            )
        )

        self.assertEqual([(candle.symbol, candle.ts) for candle in candles], [("ABC", 0), ("XYZ", 120)])


if __name__ == "__main__":
    unittest.main()
