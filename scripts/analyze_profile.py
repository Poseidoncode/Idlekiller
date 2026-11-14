#!/usr/bin/env python3
"""Analyze profiler CSV without claiming an RSS trend proves or excludes a leak.

Usage: python3 scripts/analyze_profile.py profile.csv
"""
from __future__ import annotations
import csv
import statistics
import sys
from pathlib import Path


def summarize(rows: list[dict[str, str]]) -> dict[str, float]:
    if not rows:
        raise ValueError("profile has no samples")
    scans = [int(row["scan_ms"]) for row in rows]
    rss = [int(row["own_rss_bytes"]) for row in rows]
    times = [int(row["seconds"]) for row in rows]
    if any(x < 0 for x in scans + rss) or any(x < 0 for x in times):
        raise ValueError("profile contains negative values")
    ordered = sorted(scans)
    p95_index = max(0, (95 * len(ordered) + 99) // 100 - 1)
    valid_rss = [(t, r) for t, r in zip(times, rss) if r > 0]
    if len(valid_rss) >= 2:
        duration = max(1, valid_rss[-1][0] - valid_rss[0][0])
        rss_delta_mb = (valid_rss[-1][1] - valid_rss[0][1]) / (1024 * 1024)
        slope_mb_h = rss_delta_mb * 3600 / duration
    else:
        rss_delta_mb = float("nan")
        slope_mb_h = float("nan")
    return {
        "samples": float(len(rows)),
        "scan_median_ms": float(statistics.median(scans)),
        "scan_p95_ms": float(ordered[p95_index]),
        "scan_max_ms": float(max(scans)),
        "rss_start_mb": valid_rss[0][1] / (1024 * 1024) if valid_rss else float("nan"),
        "rss_end_mb": valid_rss[-1][1] / (1024 * 1024) if valid_rss else float("nan"),
        "rss_delta_mb": rss_delta_mb,
        "rss_linear_endpoint_mb_per_hour": slope_mb_h,
    }


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print("Usage: analyze_profile.py <profile.csv>", file=sys.stderr)
        return 2
    try:
        with Path(argv[1]).open(newline="", encoding="utf-8-sig") as handle:
            reader = csv.DictReader(handle)
            required = {"seconds", "process_count", "scan_ms", "own_rss_bytes"}
            if not required.issubset(reader.fieldnames or []):
                raise ValueError("CSV columns missing: " + ", ".join(sorted(required)))
            result = summarize(list(reader))
    except (OSError, ValueError, KeyError) as error:
        print(f"Invalid profile: {error}", file=sys.stderr)
        return 1
    for key, value in result.items():
        print(f"{key}: {value:.2f}")
    print("NOTE: RSS variation alone does not demonstrate or rule out a memory leak.")
    print("Repeat profiles at stable load and correlate with heap/handle profiling.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
