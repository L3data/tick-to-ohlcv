from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Iterable, Iterator


@dataclass(frozen=True)
class Trade:
    """A normalized market trade tick.

    `ts_ms` is Unix time in milliseconds. `size` is base-asset volume and
    `turnover` is quote-asset volume.
    """

    symbol: str
    ts_ms: int
    price: float
    size: float
    turnover: float | None = None
    side: str | None = None
    trade_id: str | None = None


@dataclass
class Candle:
    symbol: str
    ts: int
    open: float
    high: float
    low: float
    close: float
    volume: float = 0.0
    turnover: float = 0.0
    trade_count: int = 0
    buy_volume: float = 0.0
    sell_volume: float = 0.0
    first_trade_ts_ms: int | None = None
    last_trade_ts_ms: int | None = None

    @property
    def datetime(self) -> str:
        return datetime.fromtimestamp(self.ts, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    def add(self, trade: Trade) -> None:
        self.high = max(self.high, trade.price)
        self.low = min(self.low, trade.price)
        self.close = trade.price
        self.volume += trade.size
        self.turnover += trade.turnover if trade.turnover is not None else trade.price * trade.size
        self.trade_count += 1
        self.last_trade_ts_ms = trade.ts_ms
        side = (trade.side or "").lower()
        if side == "buy":
            self.buy_volume += trade.size
        elif side == "sell":
            self.sell_volume += trade.size

    def to_dict(self) -> dict[str, object]:
        return {
            "symbol": self.symbol,
            "ts": self.ts,
            "datetime": self.datetime,
            "open": self.open,
            "high": self.high,
            "low": self.low,
            "close": self.close,
            "volume": self.volume,
            "turnover": self.turnover,
            "trade_count": self.trade_count,
            "buy_volume": self.buy_volume,
            "sell_volume": self.sell_volume,
            "first_trade_ts_ms": self.first_trade_ts_ms,
            "last_trade_ts_ms": self.last_trade_ts_ms,
        }


def interval_start_s(ts_ms: int, interval_seconds: int) -> int:
    if interval_seconds <= 0:
        raise ValueError("interval_seconds must be positive")
    ts_s = ts_ms // 1000
    return (ts_s // interval_seconds) * interval_seconds


def _trade_sort_key(trade: Trade) -> tuple[int, int, int | str]:
    raw_id = trade.trade_id or ""
    if raw_id.isdigit():
        return trade.ts_ms, 0, int(raw_id)
    return trade.ts_ms, 1, raw_id


def aggregate_trades(
    trades: Iterable[Trade],
    *,
    interval_seconds: int = 60,
    sort: bool = True,
    fill_gaps: bool = False,
) -> Iterator[Candle]:
    """Aggregate trades into candles grouped by symbol and interval.

    Candles are emitted in `(symbol, ts)` order. Empty intervals are not filled;
    when `fill_gaps` is enabled, missing intervals between observed candles are
    emitted as zero-volume flat candles using the previous close.
    """

    ordered = sorted(trades, key=_trade_sort_key) if sort else list(trades)
    candles: dict[tuple[str, int], Candle] = {}

    for trade in ordered:
        if trade.price <= 0 or trade.size < 0:
            continue
        symbol = trade.symbol.upper()
        bucket_ts = interval_start_s(trade.ts_ms, interval_seconds)
        key = (symbol, bucket_ts)
        candle = candles.get(key)
        if candle is None:
            turnover = trade.turnover if trade.turnover is not None else trade.price * trade.size
            candle = Candle(
                symbol=symbol,
                ts=bucket_ts,
                open=trade.price,
                high=trade.price,
                low=trade.price,
                close=trade.price,
                volume=trade.size,
                turnover=turnover,
                trade_count=1,
                first_trade_ts_ms=trade.ts_ms,
                last_trade_ts_ms=trade.ts_ms,
            )
            side = (trade.side or "").lower()
            if side == "buy":
                candle.buy_volume = trade.size
            elif side == "sell":
                candle.sell_volume = trade.size
            candles[key] = candle
        else:
            candle.add(trade)

    if not fill_gaps:
        for key in sorted(candles):
            yield candles[key]
        return

    previous_by_symbol: dict[str, Candle] = {}
    for symbol, ts in sorted(candles):
        current = candles[(symbol, ts)]
        previous = previous_by_symbol.get(symbol)
        if previous is not None:
            gap_ts = previous.ts + interval_seconds
            while gap_ts < current.ts:
                yield Candle(
                    symbol=symbol,
                    ts=gap_ts,
                    open=previous.close,
                    high=previous.close,
                    low=previous.close,
                    close=previous.close,
                )
                gap_ts += interval_seconds
        yield current
        previous_by_symbol[symbol] = current
