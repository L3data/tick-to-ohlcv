# tick-to-ohlcv

Generic utility for converting raw market trade ticks into structured OHLCV candles.

Current adapters:

- Explicitly mapped CSV trade files

Install with optional Parquet support:

```bash
pip install "tick-to-ohlcv[parquet]"
```

Convert explicitly selected CSV files:

```bash
python -m tick_to_ohlcv.cli csv path/to/trades.csv \
  --symbol BTCUSD \
  --timestamp-column timestamp_ms \
  --price-column price \
  --size-column size \
  --turnover-column notional \
  --side-column side \
  --trade-id-column trade_id \
  --output candles.csv
```

Discover many files under a root, emit flat zero-volume gap candles, and write
Parquet:

```bash
tick-to-ohlcv csv \
  --input-root /data/raw-trades \
  --include "**/*.csv" \
  --exclude "**/bad/*.csv" \
  --symbol-column symbol \
  --timestamp-column timestamp_ms \
  --price-column price \
  --size-column size \
  --turnover-column notional \
  --side-column side \
  --interval 5m \
  --fill-gaps \
  --output-format parquet \
  --output /data/candles.parquet
```

Output candles use second-based interval start timestamps and include:

```text
symbol, ts, datetime, open, high, low, close, volume, turnover,
trade_count, buy_volume, sell_volume, first_trade_ts_ms, last_trade_ts_ms
```

## Library usage

```python
from pathlib import Path

from tick_to_ohlcv import CsvTradeMapping, aggregate_csv_files, discover_files, write_candles
from tick_to_ohlcv.intervals import parse_interval_seconds

files = discover_files(
    input_root=Path("/data/raw-trades"),
    include_patterns=["**/*.csv"],
    exclude_patterns=["**/bad/*.csv"],
)
mapping = CsvTradeMapping(
    symbol_column="symbol",
    timestamp_column="timestamp_ms",
    price_column="price",
    size_column="size",
    turnover_column="notional",
)
candles = aggregate_csv_files(
    files,
    mapping,
    interval_seconds=parse_interval_seconds("1m"),
    fill_gaps=True,
)
write_candles(candles, output=Path("/data/candles.csv"))
```
