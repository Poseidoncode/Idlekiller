"""Independent Python tests; run without Cargo: python3 -m unittest discover scripts."""
import unittest
from analyze_profile import summarize


class ProfileSummaryTests(unittest.TestCase):
    def test_median_p95_and_growth(self):
        rows = [
            {"seconds": str(i * 60), "scan_ms": str(i),
             "own_rss_bytes": str((100 + i) * 1024 * 1024)}
            for i in range(1, 21)
        ]
        out = summarize(rows)
        self.assertEqual(out["samples"], 20.0)
        self.assertEqual(out["scan_median_ms"], 10.5)
        self.assertEqual(out["scan_p95_ms"], 19.0)
        self.assertEqual(out["rss_delta_mb"], 19.0)

    def test_rejects_empty_profile(self):
        with self.assertRaises(ValueError):
            summarize([])

    def test_missing_rss_is_not_false_zero_growth(self):
        rows = [
            {"seconds": "0", "scan_ms": "12", "own_rss_bytes": "0"},
            {"seconds": "1", "scan_ms": "7", "own_rss_bytes": "0"},
        ]
        self.assertNotEqual(summarize(rows)["rss_delta_mb"], 0.0)


if __name__ == "__main__":
    unittest.main()
