from __future__ import annotations

from pathlib import Path


def _matches_any(path: Path, patterns: list[str]) -> bool:
    text = path.as_posix()
    return any(path.match(pattern) or Path(text).match(pattern) for pattern in patterns)


def discover_files(
    *,
    paths: list[Path] | None = None,
    input_root: Path | None = None,
    include_patterns: list[str] | None = None,
    exclude_patterns: list[str] | None = None,
) -> list[Path]:
    """Return sorted unique files from explicit paths and optional root globs."""

    selected: list[Path] = []
    seen: set[Path] = set()

    def add(path: Path) -> None:
        resolved = path.resolve()
        if resolved in seen or not path.is_file():
            return
        seen.add(resolved)
        selected.append(path)

    for path in paths or []:
        add(path)

    if input_root is not None:
        includes = include_patterns or ["**/*.csv"]
        excludes = exclude_patterns or []
        for pattern in includes:
            for path in sorted(input_root.glob(pattern)):
                if not path.is_file():
                    continue
                relative = path.relative_to(input_root)
                if _matches_any(relative, excludes):
                    continue
                add(path)

    return sorted(selected)
