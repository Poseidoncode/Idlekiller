[English](./README.md) | [繁體中文](./README.zh-TW.md)

# Idlekiller

A Rust-based TUI process manager for identifying and cleaning up resource-heavy processes.

## Features

- Live process table with CPU, memory and status
- Sort by any column (`Tab` / `Shift + Tab`, `r` to reverse)
- Filter by name in-app (`f` or `/`); `Esc` restores the previous filter
- Confirmed cleanup of repeatedly idle and memory-heavy process candidates (`Shift + K` twice)
- Search the selected process in your browser (`s`)
- Mouse support: scroll, click headers to sort
- Background monitoring with bounded latest-only snapshots and responsive UI
- One copy of the process data with lightweight filter/sort indices
- Identity/owner revalidation before a termination signal
- Per-PID result dialog and `SCAN` collector timing diagnostics
- Immediate stale/fresh status updates and protected-process hints
- Modal dialogs block background mouse input; repeated rendering failures restore the terminal

## Requirements

- Rust 1.88 or newer
- A terminal at least 50x16

## Installation

### Build this reviewed ZIP locally (recommended)

Extract the archive, enter the `Idlekiller` folder and run:

```bash
cargo install --path . --locked
```

**Note:** The one-line installers below download the GitHub `main` branch, which may not contain the changes in this reviewed ZIP. Read scripts before running them.

### Upstream one-line install (macOS / Linux)

```bash
curl -sSL https://raw.githubusercontent.com/Poseidoncode/Idlekiller/main/install.sh | bash
```

### Windows

One-line install with PowerShell (requires Rust):

```powershell
irm https://raw.githubusercontent.com/Poseidoncode/Idlekiller/main/install.ps1 | iex
```

## Usage

```bash
idlekiller
```

## Controls

| Key | Action |
| --- | --- |
| `↑` / `↓` / `k` / `j` | Move selection up / down |
| `Enter` / `x` | Preview and confirm termination of the selected process (press twice within 8s) |
| `f` / `/` | Edit name filter; Enter commits, Esc restores the previous filter |
| `Shift + K` | Review cleanup candidates (press twice within 8s to confirm) |
| `s` | Search the selected process on Google |
| `Tab` / `Shift + Tab` | Cycle sort column forward / backward |
| `r` | Reverse the current sort direction |
| `q` / `Esc` | Quit; Esc closes an open confirmation/result dialog first |

Mouse:

- Scroll to move through the list
- Click a column header to sort by that column

## How cleanup works

A process becomes a **cleanup candidate**, not a proven leak, when:

- Status is `Sleeping`, `Idle` or `Parked`
- CPU usage is below `0.1%`
- Memory usage is above `500 MB`
- These conditions hold for **3 consecutive CPU samples** (following CPU warm-up)
- The owner can be verified and the process is not protected

Press `Shift + K` twice to request termination of visible-filter candidates, up to 4 at a time. The worker re-verifies identities and eligibility before signalling. The UI remains responsive during scanning and termination. PIDs `0`-`2`, kernel threads, processes owned by other users, well-known system/desktop/shell apps, and the current Idlekiller process are always protected and will not be killed. Bulk cleanup is refused while running as root (single kills still work).


### Safety and limitations

**Sleeping and high memory usage do not prove a process is expendable.** The 500 MB / 0.1% thresholds identify *candidates* only, not memory leaks. Always inspect the preview and avoid bulk cleanup when important work is running. On Unix, the program requests graceful termination with SIGTERM; on Windows, termination remains forceful. Unsaved work may still be lost. Individual and bulk termination require a second confirmation within eight seconds and recheck process identity and ownership before acting. The program does not guarantee protection against every possible PID race. If ownership cannot be established, termination is refused.

The table adapts to 50-column terminals (status is hidden at narrow widths), and mouse clicks can select rows.

## Tests and limitations

CI configuration is in `.github/workflows/ci.yml`. See [V4 release-readiness review](./V4_RELEASE_READINESS.zh-TW.md). Formatting checks are advisory until the existing source is fully rustfmt-formatted. On systems where user identity cannot be retrieved, termination is intentionally disabled.

## Non-destructive memory and performance profiling

Measure collector scan latency and the application's RSS, without sending any termination signals:

```bash
cargo run --locked --release --example profile_collector -- 3600 > profile.csv
python3 scripts/analyze_profile.py profile.csv
python3 -m unittest discover -s scripts -p 'test_*.py'
```

The duration is clamped to 1–21,600 seconds (up to 6 hours). RSS changes alone neither prove nor exclude leaks; repeat at comparable workloads and corroborate with heap/handle tooling. See [V4 verification notes](./V4_RELEASE_READINESS.zh-TW.md).
