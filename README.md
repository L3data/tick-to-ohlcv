# tick-to-ohlcv

Generic utility for converting raw market trade ticks into structured OHLCV candles.

Current adapters:

- Explicitly mapped CSV trade files

Example:

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

Output candles use second-based interval start timestamps and include:

```text
symbol, ts, datetime, open, high, low, close, volume, turnover,
trade_count, buy_volume, sell_volume, first_trade_ts_ms, last_trade_ts_ms
```
