from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from tick_to_ohlcv import cli


class CliTests(unittest.TestCase):
    def test_csv_command_discovers_files_and_fills_gaps(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            input_dir = root / "raw"
            input_dir.mkdir()
            (input_dir / "trades.csv").write_text(
                "\n".join([
                    "ts,price,size,quote,side",
                    "0,10,1,10,buy",
                    "120000,12,2,24,sell",
                ]) + "\n",
                encoding="utf-8",
            )
            output = root / "candles.csv"

            rc = cli.main([
                "csv",
                "--input-root",
                str(input_dir),
                "--include",
                "**/*.csv",
                "--symbol",
                "ABCUSDT",
                "--timestamp-column",
                "ts",
                "--price-column",
                "price",
                "--size-column",
                "size",
                "--turnover-column",
                "quote",
                "--side-column",
                "side",
                "--interval",
                "1m",
                "--fill-gaps",
                "--output-format",
                "csv",
                "--output",
                str(output),
            ])

            rows = output.read_text(encoding="utf-8").splitlines()

        self.assertEqual(rc, 0)
        self.assertEqual(len(rows), 4)
        self.assertIn("ABCUSDT,60,1970-01-01T00:01:00Z,10.0,10.0,10.0,10.0,0.0,0.0,0", rows[2])

    def test_csv_command_requires_input_files(self):
        with self.assertRaisesRegex(ValueError, "at least one input file"):
            cli.main([
                "csv",
                "--symbol",
                "ABCUSDT",
                "--timestamp-column",
                "ts",
                "--price-column",
                "price",
                "--size-column",
                "size",
            ])


if __name__ == "__main__":
    unittest.main()
