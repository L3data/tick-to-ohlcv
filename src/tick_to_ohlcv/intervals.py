from __future__ import annotations


UNITS = {
    "s": 1,
    "sec": 1,
    "second": 1,
    "seconds": 1,
    "m": 60,
    "min": 60,
    "minute": 60,
    "minutes": 60,
    "h": 60 * 60,
    "hr": 60 * 60,
    "hour": 60 * 60,
    "hours": 60 * 60,
    "d": 24 * 60 * 60,
    "day": 24 * 60 * 60,
    "days": 24 * 60 * 60,
}


def parse_interval_seconds(value: str | int) -> int:
    """Parse interval strings like `60`, `1m`, `5m`, `1h`, or `1d`."""

    if isinstance(value, int):
        if value <= 0:
            raise ValueError("interval must be positive")
        return value

    text = str(value).strip().lower()
    if not text:
        raise ValueError("interval must not be empty")
    if text.isdigit():
        seconds = int(text)
        if seconds <= 0:
            raise ValueError("interval must be positive")
        return seconds

    index = 0
    while index < len(text) and text[index].isdigit():
        index += 1
    if index == 0:
        raise ValueError(f"interval must start with a number: {value}")

    amount = int(text[:index])
    unit = text[index:].strip()
    if amount <= 0:
        raise ValueError("interval must be positive")
    if unit not in UNITS:
        raise ValueError(f"unsupported interval unit: {unit}")
    return amount * UNITS[unit]
