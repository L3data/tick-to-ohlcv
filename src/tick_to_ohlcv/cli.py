from __future__ import annotations

import argparse
from pathlib import Path

from .csv_adapter import CsvTradeMapping, aggregate_csv_files
from .discovery import discover_files
from .intervals import parse_interval_seconds
from .writers import write_candles


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Convert raw trade ticks into OHLCV candles.")
    sub = parser.add_subparsers(dest="source", required=True)

    csv_parser = sub.add_parser("csv", help="Convert CSV trade files using explicit column mappings")
    csv_parser.add_argument("paths", nargs="*", type=Path)
    csv_parser.add_argument("--input-root", type=Path, help="Discover input files under this directory")
    csv_parser.add_argument(
        "--include",
        action="append",
        help="Glob pattern relative to --input-root; can be passed multiple times",
    )
    csv_parser.add_argument(
        "--exclude",
        action="append",
        help="Glob pattern relative to --input-root to skip; can be passed multiple times",
    )
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
    csv_parser.add_argument("--interval", default="1m", help="Candle interval such as 60, 1m, 5m, 1h, or 1d")
    csv_parser.add_argument("--interval-seconds", type=int, help="Deprecated alias for second-based intervals")
    csv_parser.add_argument("--fill-gaps", action="store_true", help="Emit zero-volume flat candles for empty intervals")
    csv_parser.add_argument("--output-format", choices=["csv", "parquet"], help="Defaults to output extension or csv")
    csv_parser.add_argument("--output", type=Path, help="Output path; CSV defaults to stdout")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if args.source == "csv":
        if not args.symbol and not args.symbol_column:
            raise ValueError("CSV input requires --symbol or --symbol-column")
        paths = discover_files(
            paths=args.paths,
            input_root=args.input_root,
            include_patterns=args.include,
            exclude_patterns=args.exclude,
        )
        if not paths:
            raise ValueError("CSV input requires at least one input file")
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
        interval_seconds = args.interval_seconds or parse_interval_seconds(args.interval)
        candles = aggregate_csv_files(
            paths,
            mapping,
            interval_seconds=interval_seconds,
            delimiter=args.delimiter,
            fill_gaps=args.fill_gaps,
        )
    else:
        raise ValueError(f"Unsupported source: {args.source}")
    write_candles(candles, output=args.output, output_format=args.output_format)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
