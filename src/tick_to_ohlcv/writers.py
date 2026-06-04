from __future__ import annotations

import csv
import sys
from pathlib import Path
from typing import Iterable, Literal

from .core import Candle


OutputFormat = Literal["csv", "parquet"]

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


def infer_output_format(output: Path | None, explicit: str | None = None) -> OutputFormat:
    if explicit:
        normalized = explicit.lower()
        if normalized not in {"csv", "parquet"}:
            raise ValueError(f"unsupported output format: {explicit}")
        return normalized  # type: ignore[return-value]
    if output and output.suffix.lower() in {".parquet", ".pq"}:
        return "parquet"
    return "csv"


def write_candles_csv(candles: Iterable[Candle], output: Path | None = None) -> None:
    handle = output.open("w", encoding="utf-8", newline="") if output else sys.stdout
    try:
        writer = csv.DictWriter(handle, fieldnames=FIELDS)
        writer.writeheader()
        for candle in candles:
            writer.writerow(candle.to_dict())
    finally:
        if output:
            handle.close()


def write_candles_parquet(candles: Iterable[Candle], output: Path) -> None:
    try:
        import pyarrow as pa
        import pyarrow.parquet as pq
    except ModuleNotFoundError as exc:
        raise RuntimeError("Parquet output requires pyarrow") from exc

    rows = [candle.to_dict() for candle in candles]
    table = pa.Table.from_pylist(rows, schema=None)
    output.parent.mkdir(parents=True, exist_ok=True)
    pq.write_table(table, output)


def write_candles(
    candles: Iterable[Candle],
    *,
    output: Path | None = None,
    output_format: str | None = None,
) -> None:
    resolved_format = infer_output_format(output, output_format)
    if resolved_format == "csv":
        write_candles_csv(candles, output)
        return
    if output is None:
        raise ValueError("Parquet output requires --output")
    write_candles_parquet(candles, output)
