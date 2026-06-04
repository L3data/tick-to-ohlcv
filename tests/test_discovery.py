from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from tick_to_ohlcv.discovery import discover_files


class FileDiscoveryTests(unittest.TestCase):
    def test_discovers_sorted_files_with_includes_and_excludes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            keep_a = root / "a" / "trades.csv"
            keep_b = root / "b" / "trades.csv"
            skip = root / "skip" / "trades.csv"
            ignored = root / "notes.txt"
            for path in (keep_a, keep_b, skip, ignored):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("", encoding="utf-8")

            files = discover_files(
                input_root=root,
                include_patterns=["**/*.csv"],
                exclude_patterns=["skip/**"],
            )

        self.assertEqual([path.name for path in files], ["trades.csv", "trades.csv"])
        self.assertEqual([path.parent.name for path in files], ["a", "b"])

    def test_merges_explicit_paths_and_discovered_files_without_duplicates(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            explicit = root / "explicit.csv"
            discovered = root / "nested" / "trades.csv"
            for path in (explicit, discovered):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("", encoding="utf-8")

            files = discover_files(paths=[explicit], input_root=root, include_patterns=["**/*.csv"])

        self.assertEqual(files, sorted([explicit, discovered]))


if __name__ == "__main__":
    unittest.main()
