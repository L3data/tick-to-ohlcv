from __future__ import annotations

import csv
from dataclasses import dataclass
from pathlib import Path
from typing import Iterator

from .core import Candle, Trade, aggregate_trades


@dataclass(frozen=True)
class CsvTradeMapping:
    timestamp_column: str
    price_column: str
    size_column: str
    symbol: str | None = None
    symbol_column: str | None = None
    turnover_column: str | None = None
    side_column: str | None = None
    trade_id_column: str | None = None
    timestamp_unit: str = "ms"

    def resolve_symbol(self, row: dict[str, str]) -> str:
        if self.symbol:
            return self.symbol.upper()
        if self.symbol_column and row.get(self.symbol_column):
            return str(row[self.symbol_column]).upper()
        raise ValueError("CSV mapping requires either symbol or symbol_column")


def _optional_float(row: dict[str, str], column: str | None) -> float | None:
    if not column:
        return None
    value = row.get(column)
    if value in (None, ""):
        return None
    return float(value)


def _optional_string(row: dict[str, str], column: str | None) -> str | None:
    if not column:
        return None
    value = row.get(column)
    if value in (None, ""):
        return None
    return str(value)


def _timestamp_ms(value: str, unit: str) -> int:
    ts = float(value)
    normalized = unit.lower()
    if normalized in {"ms", "millisecond", "milliseconds"}:
        return int(ts)
    if normalized in {"s", "sec", "second", "seconds"}:
        return int(ts * 1000)
    raise ValueError(f"Unsupported timestamp unit: {unit}")


def trade_from_csv_row(row: dict[str, str], mapping: CsvTradeMapping) -> Trade:
    price = float(row[mapping.price_column])
    size = float(row[mapping.size_column])
    return Trade(
        symbol=mapping.resolve_symbol(row),
        ts_ms=_timestamp_ms(row[mapping.timestamp_column], mapping.timestamp_unit),
        price=price,
        size=size,
        turnover=_optional_float(row, mapping.turnover_column),
        side=_optional_string(row, mapping.side_column),
        trade_id=_optional_string(row, mapping.trade_id_column),
    )


def iter_csv_trades(path: Path, mapping: CsvTradeMapping, *, delimiter: str = ",") -> Iterator[Trade]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter=delimiter)
        for row in reader:
            yield trade_from_csv_row(row, mapping)


def aggregate_csv_files(
    paths: list[Path],
    mapping: CsvTradeMapping,
    *,
    interval_seconds: int = 60,
    delimiter: str = ",",
) -> list[Candle]:
    trades = (trade for path in paths for trade in iter_csv_trades(path, mapping, delimiter=delimiter))
    return list(aggregate_trades(trades, interval_seconds=interval_seconds))
