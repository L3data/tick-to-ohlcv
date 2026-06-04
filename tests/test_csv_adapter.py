from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from tick_to_ohlcv.csv_adapter import CsvTradeMapping, aggregate_csv_files


class CsvAdapterTests(unittest.TestCase):
    def test_aggregates_mapped_csv_trades(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "trades.csv"
            path.write_text(
                "\n".join([
                    "id,ts,px,qty,quote,side,market",
                    "1,1700000000000,10.0,2.0,20.0,buy,ABCUSD",
                    "2,1700000001000,11.0,3.0,33.0,sell,ABCUSD",
                    "3,1700000060000,12.0,1.5,18.0,buy,ABCUSD",
                ]) + "\n",
                encoding="utf-8",
            )

            candles = aggregate_csv_files(
                [path],
                CsvTradeMapping(
                    timestamp_column="ts",
                    price_column="px",
                    size_column="qty",
                    turnover_column="quote",
                    side_column="side",
                    trade_id_column="id",
                    symbol_column="market",
                ),
            )

        self.assertEqual(len(candles), 2)
        self.assertEqual(candles[0].symbol, "ABCUSD")
        self.assertEqual(candles[0].open, 10.0)
        self.assertEqual(candles[0].high, 11.0)
        self.assertEqual(candles[0].low, 10.0)
        self.assertEqual(candles[0].close, 11.0)
        self.assertEqual(candles[0].volume, 5.0)
        self.assertEqual(candles[0].turnover, 53.0)
        self.assertEqual(candles[0].trade_count, 2)
        self.assertEqual(candles[0].buy_volume, 2.0)
        self.assertEqual(candles[0].sell_volume, 3.0)


if __name__ == "__main__":
    unittest.main()
