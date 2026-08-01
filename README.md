[English](./README.md) | [繁體中文](./README.zh-TW.md)

# Idlekiller

A Rust-based TUI process manager for identifying and cleaning up resource-heavy processes.

## Features

- Live process table with CPU, memory and status
- Sort by any column (`Tab` / `Shift + Tab`, `r` to reverse)
- Filter by name in-app (`f` or `/`)
- One-click cleanup of idle but memory-heavy processes (`Shift + K`)
- Search the selected process in your browser (`s`)
- Mouse support: scroll, click headers to sort
- Built-in protection for system PIDs and self

## Requirements

- Rust 1.85 or newer
- A terminal at least 80x24

## Installation

### One-line install (macOS / Linux)

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
| `Enter` / `x` | Kill the selected process |
| `f` / `/` | Filter processes by name |
| `Shift + K` | Clean up wasteful idle processes (press twice to confirm) |
| `s` | Search the selected process on Google |
| `Tab` / `Shift + Tab` | Cycle sort column forward / backward |
| `r` | Reverse the current sort direction |
| `q` / `Esc` | Quit |

Mouse:

- Scroll to move through the list
- Click a column header to sort by that column

## How cleanup works

A process is flagged as wasteful and shown in yellow when:

- Status is `Sleeping`, `Idle` or `Parked`
- CPU usage is below `0.1%`
- Memory usage is above `500 MB`

Press `Shift + K` twice to terminate all processes matching these rules. PIDs `0`, `1` and the current Idlekiller process are always protected and will not be killed.

