[English](./README.md) | 繁體中文

# Idlekiller

一個以 Rust 打造的 TUI 進程管理工具，用來識別並清理占用過多系統資源的進程。

## 功能

- 即時顯示進程列表，包含 CPU、記憶體與狀態
- 可依任何欄位排序（`Tab` / `Shift + Tab`，`r` 反轉排序）
- 在程式內依名稱過濾進程（`f` 或 `/`）
- 一鍵清理閒置但佔用大量記憶體的進程（`Shift + K`）
- 在瀏覽器搜尋選中的進程資訊（`s`）
- 支援滑鼠：捲動與點擊欄位標題排序
- 內建保護系統進程與自身進程

## 需求

- Rust 1.85 或更新版本
- 至少 80x24 的終端機

## 安裝

### 一鍵安裝（macOS / Linux）

```bash
curl -sSL https://raw.githubusercontent.com/Poseidoncode/Idlekiller/main/install.sh | bash
```

> 如果你不想直接 pipe 到 `bash`，可以先查看 [install.sh](./install.sh) 腳本內容。

### 從原始碼編譯

```bash
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller
cargo build --release
```

編譯完成後，執行檔位於 `target/release/idlekiller`。

### Windows

使用 PowerShell 一鍵安裝（需先安裝 Rust）：

```powershell
irm https://raw.githubusercontent.com/Poseidoncode/Idlekiller/main/install.ps1 | iex
```

> 如果你不想直接 pipe 到 `iex`，可以先查看 [install.ps1](./install.ps1)  腳本內容。

### macOS

安裝到系統路徑：

```bash
sudo cp target/release/idlekiller /usr/local/bin/
```

或打包成可雙擊開啟的 `.app`：

```bash
make app
# 然後將 Idlekiller.app 拖入 /Applications
```

### Linux

```bash
sudo cp target/release/idlekiller /usr/local/bin/
```

## 使用方式

```bash
idlekiller
```

在 macOS 上也可以從 Launchpad 或 Finder 開啟 `Idlekiller.app`。

## 操作說明

| 按鍵 | 功能 |
| --- | --- |
| `↑` / `↓` / `k` / `j` | 上下移動選擇 |
| `Enter` / `x` | 終止選中的進程 |
| `f` / `/` | 依名稱過濾進程 |
| `Shift + K` | 一鍵清理資源浪費者（需按兩次確認） |
| `s` | 用 Google 搜尋該進程 |
| `Tab` / `Shift + Tab` | 切換排序欄位（順向 / 反向） |
| `r` | 反轉目前排序方向 |
| `q` / `Esc` | 退出 |

滑鼠操作：

- 捲動：在列表中上下移動
- 點擊欄位標題：依該欄位排序

## 清理規則

當進程符合以下條件時，會被標示為資源浪費者（以黃色顯示）：

- 狀態為 `Sleeping`、`Idle` 或 `Parked`
- CPU 使用率低於 `0.1%`
- 記憶體佔用超過 `500 MB`

按兩次 `Shift + K` 即可終止所有符合條件的進程。`0`、`1` 號 PID 與 Idlekiller 自身會受到保護，不會被終止。

## 開發

```bash
cargo run           # 開發模式執行
cargo test          # 執行邏輯與效能測試
cargo build --release
make app            # 打包 macOS .app
```
