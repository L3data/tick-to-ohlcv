"""Generic trade tick to OHLCV conversion helpers."""

from .core import Candle, Trade, aggregate_trades
from .csv_adapter import CsvTradeMapping, aggregate_csv_files, iter_csv_trades
from .discovery import discover_files
from .intervals import parse_interval_seconds
from .writers import write_candles

__all__ = [
    "Candle",
    "CsvTradeMapping",
    "Trade",
    "aggregate_csv_files",
    "aggregate_trades",
    "discover_files",
    "iter_csv_trades",
    "parse_interval_seconds",
    "write_candles",
]
