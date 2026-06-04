from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path
from typing import Iterable

from .core import Candle
from .csv_adapter import CsvTradeMapping, aggregate_csv_files


FIELDS = [
    "symbol",
    "ts",
    "datetime",
    "open",
    "high",
    "low",
    "close",
    "volume",
    "turnover",
    "trade_count",
    "buy_volume",
    "sell_volume",
    "first_trade_ts_ms",
    "last_trade_ts_ms",
]


def _write_csv(candles: Iterable[Candle], output: Path | None) -> None:
    handle = output.open("w", encoding="utf-8", newline="") if output else sys.stdout
    try:
        writer = csv.DictWriter(handle, fieldnames=FIELDS)
        writer.writeheader()
        for candle in candles:
            writer.writerow(candle.to_dict())
    finally:
        if output:
            handle.close()


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Convert raw trade ticks into OHLCV candles.")
    sub = parser.add_subparsers(dest="source", required=True)

    csv_parser = sub.add_parser("csv", help="Convert CSV trade files using explicit column mappings")
    csv_parser.add_argument("paths", nargs="+", type=Path)
    csv_parser.add_argument("--timestamp-column", required=True)
    csv_parser.add_argument("--timestamp-unit", default="ms", choices=["ms", "s"])
    csv_parser.add_argument("--price-column", required=True)
    csv_parser.add_argument("--size-column", required=True)
    csv_parser.add_argument("--symbol")
    csv_parser.add_argument("--symbol-column")
    csv_parser.add_argument("--turnover-column")
    csv_parser.add_argument("--side-column")
    csv_parser.add_argument("--trade-id-column")
    csv_parser.add_argument("--delimiter", default=",")
    csv_parser.add_argument("--interval-seconds", type=int, default=60)
    csv_parser.add_argument("--output", type=Path, help="CSV output path; defaults to stdout")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if args.source == "csv":
        if not args.symbol and not args.symbol_column:
            raise ValueError("CSV input requires --symbol or --symbol-column")
        mapping = CsvTradeMapping(
            timestamp_column=args.timestamp_column,
            timestamp_unit=args.timestamp_unit,
            price_column=args.price_column,
            size_column=args.size_column,
            symbol=args.symbol,
            symbol_column=args.symbol_column,
            turnover_column=args.turnover_column,
            side_column=args.side_column,
            trade_id_column=args.trade_id_column,
        )
        candles = aggregate_csv_files(
            args.paths,
            mapping,
            interval_seconds=args.interval_seconds,
            delimiter=args.delimiter,
        )
    else:
        raise ValueError(f"Unsupported source: {args.source}")
    _write_csv(candles, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
