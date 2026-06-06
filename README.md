# tick-to-ohlcv

Fast, exchange-agnostic trade tick aggregation for market data pipelines.

`tick-to-ohlcv` converts raw trade CSVs into clean OHLCV candles with volume,
turnover, trade counts, taker buy/sell volume, optional flat gap candles, and
CSV or Parquet output. It is built for crypto, FX, equities, and any market
where raw ticks need to become analysis-ready candles.

## Why Use It

- **Generic CSV mapping**: point the tool at your timestamp, price, size,
  symbol, side, turnover, and trade-id columns.
- **Fast Rust CLI**: aggregate large folders of trade files without writing
  one-off scripts.
- **Python compatibility**: use the same core ideas from Python data pipelines.
- **Flexible intervals**: create `1m`, `5m`, `1h`, daily, or custom-second
  candles.
- **Gap filling**: emit zero-volume flat candles between observed trades when
  you need continuous time series.
- **CSV and Parquet output**: write quick CSVs or columnar datasets for
  analytics.
- **Folder discovery**: include and exclude file patterns under a root instead
  of listing every input file.

## Install

Build the Rust CLI:

```bash
cargo build --release
```

Install the Python package:

```bash
pip install tick-to-ohlcv
```

Install Python with optional Parquet support:

```bash
pip install "tick-to-ohlcv[parquet]"
```

## Quick Start

Convert one CSV file to one-minute candles:

```bash
tick-to-ohlcv csv trades.csv \
  --symbol BTCUSDT \
  --timestamp-column timestamp_ms \
  --price-column price \
  --size-column size \
  --turnover-column quote_volume \
  --side-column side \
  --trade-id-column trade_id \
  --interval 1m \
  --output candles.csv
```

Discover many files, fill gaps, and write Parquet:

```bash
cargo run --release -- csv \
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

Use second timestamps instead of milliseconds:

```bash
tick-to-ohlcv csv fills.csv \
  --symbol ETHUSD \
  --timestamp-column time \
  --timestamp-unit s \
  --price-column px \
  --size-column qty \
  --interval 1h \
  --output eth_1h.csv
```

## Output Columns

Candles use Unix-second interval starts and ISO UTC datetimes:

```text
symbol, ts, datetime, open, high, low, close, volume, turnover,
trade_count, buy_volume, sell_volume, first_trade_ts_ms, last_trade_ts_ms
```

Notes:

- `volume` is base-asset volume.
- `turnover` is quote/notional volume when provided, otherwise `price * size`.
- `buy_volume` and `sell_volume` are taker-side base volumes from the mapped
  side column.
- Gap candles have `trade_count = 0`, zero volume/turnover, and flat OHLC.

## Python Library Usage

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
    side_column="side",
    trade_id_column="trade_id",
)

candles = aggregate_csv_files(
    files,
    mapping,
    interval_seconds=parse_interval_seconds("1m"),
    fill_gaps=True,
)

write_candles(candles, output=Path("/data/candles.csv"))
```

## Good Fits

- Rebuilding OHLCV candles from public trade archives.
- Normalizing multiple venues into one candle schema.
- Creating research datasets from raw fills.
- Backtesting and market microstructure workflows.
- Converting tick data to Parquet for warehouse or lakehouse ingestion.

## Suggested GitHub Description

```text
Fast Rust/Python CLI for converting raw trade ticks into OHLCV candles, CSV, and Parquet.
```

## Suggested GitHub Topics

```text
ohlcv, tick-data, market-data, crypto, trading, parquet, rust-cli, python, csv, candles
```
