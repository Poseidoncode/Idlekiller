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

> Review the [install.sh](./install.sh) script first if you prefer not to pipe directly to `bash`.

### Build from source

```bash
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller
cargo build --release
```

The binary is placed at `target/release/idlekiller`.

### Windows

One-line install with PowerShell (requires Rust):

```powershell
irm https://raw.githubusercontent.com/Poseidoncode/Idlekiller/main/install.ps1 | iex
```

> Review [install.ps1](./install.ps1) before running if you prefer not to pipe directly to `iex`.

### macOS

Install to your path:

```bash
sudo cp target/release/idlekiller /usr/local/bin/
```

Or package as a clickable `.app`:

```bash
make app
# Then drag Idlekiller.app to /Applications
```

### Linux

```bash
sudo cp target/release/idlekiller /usr/local/bin/
```

## Usage

```bash
idlekiller
```

On macOS you can also open `Idlekiller.app` from Launchpad or Finder.

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

## Development

```bash
cargo run           # Run in development mode
cargo test          # Run logic and benchmark tests
cargo build --release
make app            # Package macOS .app
```
