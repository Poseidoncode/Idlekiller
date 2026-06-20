[English](./README.md) | [繁體中文](./README.zh-TW.md)

# Idlekiller

A Rust-based TUI process management tool designed to identify and clean up unnecessary processes consuming system resources.

---

## 1️⃣ Installation

### Windows

```powershell
# 1. Download and extract release files
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller

# 2. Build (requires Rust installed)
cargo build --release

# 3. Move executable to your preferred folder (e.g., C:\Software\Idlekiller\)
copy target\release\idlekiller.exe C:\Path\To\Your\Folder\
```

### macOS

```bash
# Method A: Build from source
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller
cargo build --release
sudo cp target/release/idlekiller /usr/local/bin/

# Method B: Package as .app (icon launcher)
make app
# Generates Idlekiller.app, drag to Applications folder
```

### Linux

```bash
# 1. Build from source
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller
cargo build --release

# 2. Install to system path
sudo cp target/release/idlekiller /usr/local/bin/
```

---

## 2️⃣ Usage

### Launch the Program

```bash
idlekiller
```

Or directly click `Idlekiller.app` on macOS.

### Key Bindings

| ↑ / ↓ / **k** / **j** | Move up/down to select process      |
| **Enter** / **x**     | Terminate selected process (Kill)   |
| **f** / **/**         | Filter processes by name (In-app)   |
| **Shift + K**         | One-click Clean Wasteful Processes  |
| **s**                 | Search process info in browser      |
| **q** / **Esc**       | Exit the tool                       |

---

