from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from tick_to_ohlcv.core import Candle
from tick_to_ohlcv.writers import infer_output_format, write_candles


class CandleWriterTests(unittest.TestCase):
    def test_infers_output_format_from_extension_or_explicit_value(self):
        self.assertEqual(infer_output_format(Path("candles.csv")), "csv")
        self.assertEqual(infer_output_format(Path("candles.parquet")), "parquet")
        self.assertEqual(infer_output_format(Path("candles.csv"), "parquet"), "parquet")

    def test_writes_csv_candles(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "candles.csv"
            write_candles(
                [
                    Candle(
                        symbol="ABC",
                        ts=60,
                        open=1.0,
                        high=2.0,
                        low=1.0,
                        close=2.0,
                        volume=3.0,
                    )
                ],
                output=output,
                output_format="csv",
            )

            text = output.read_text(encoding="utf-8")

        self.assertIn("symbol,ts,datetime,open,high,low,close", text)
        self.assertIn("ABC,60,1970-01-01T00:01:00Z,1.0,2.0,1.0,2.0", text)

    def test_parquet_output_requires_output_path(self):
        with self.assertRaises(ValueError):
            write_candles([], output=None, output_format="parquet")


if __name__ == "__main__":
    unittest.main()
