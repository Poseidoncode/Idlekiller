[English](./README.md) | 繁體中文

# Idlekiller

一個以 Rust 打造的 TUI 進程管理工具，用來識別並清理占用過多系統資源的進程。

## 功能

- 即時顯示進程列表，包含 CPU、記憶體與狀態
- 可依任何欄位排序（`Tab` / `Shift + Tab`，`r` 反轉排序）
- 在程式內依名稱過濾進程（`f` 或 `/`）；按 `Esc` 還原前次篩選
- 對連續多次閒置且佔用大量記憶體的候選程序進行二次確認清理（`Shift + K`）
- 在瀏覽器搜尋選中的進程資訊（`s`）
- 支援滑鼠：捲動與點擊欄位標題排序
- 背景掃描 + 最新快照槽，降低 UI 阻塞與舊資料堆積
- 唯一程序資料表 + 輕量索引視圖，降低篩選排序記憶體配置
- 終止前重新檢查程序身分與擁有者
- 逐 PID 結果視窗，以及背景掃描時間 `SCAN` 指標（用於效能診斷）
- 資料過期／恢復狀態及受保護程序提示
- 操作對話框阻隔背景滑鼠操作；多次繪圖失敗時安全還原終端

## 需求

- Rust 1.88 或更新版本
- 至少 50x16 的終端機

## 安裝

### 安裝本次審查 ZIP 版本（建議）

解壓縮後進入 `Idlekiller` 資料夾，執行：

```bash
cargo install --path . --locked
```

**注意：** 下列一鍵安裝腳本會下載 GitHub `main` 分支，不一定是本次 ZIP 的優化版。執行前請先檢查腳本內容。

### 上游一鍵安裝（macOS / Linux）

```bash
curl -sSL https://raw.githubusercontent.com/Poseidoncode/Idlekiller/main/install.sh | bash
```

### Windows

使用 PowerShell 一鍵安裝（需先安裝 Rust）：

```powershell
irm https://raw.githubusercontent.com/Poseidoncode/Idlekiller/main/install.ps1 | iex
```

## 使用方式

```bash
idlekiller
```

## 操作說明

| 按鍵 | 功能 |
| --- | --- |
| `↑` / `↓` / `k` / `j` | 上下移動選擇 |
| `Enter` / `x` | 預覽並確認終止選中進程（8 秒內連按兩次） |
| `f` / `/` | 編輯名稱篩選；Enter 套用，Esc 還原原有篩選 |
| `Shift + K` | 預覽清理候選程序（8 秒內連按兩次確認） |
| `s` | 用 Google 搜尋該進程 |
| `Tab` / `Shift + Tab` | 切換排序欄位（順向 / 反向） |
| `r` | 反轉目前排序方向 |
| `q` / `Esc` | 退出；確認或結果視窗開啟時，`Esc` 先關閉視窗 |

滑鼠操作：

- 捲動：在列表中上下移動
- 點擊欄位標題：依該欄位排序

## 清理規則

當進程符合以下條件時，會被標示為「清理候選」（以黃色顯示，並不代表記憶體洩漏）：

- 狀態為 `Sleeping`、`Idle` 或 `Parked`
- CPU 使用率低於 `0.1%`
- 記憶體佔用超過 `500 MB`
- 通過 CPU 暖機後，**連續三個有效樣本**皆符合條件
- 身分與擁有者可驗證，且不在保護程序內

按兩次 `Shift + K` 可請求終止目前篩選結果中的候選程序，單次上限 4 筆；背景工作執行緒在發訊號前仍會逐項複核。`0`–`2` 號 PID、核心執行緒、其他使用者擁有的進程、常見的系統／桌面／Shell 應用程式，以及 Idlekiller 自身會受到保護，不會被終止。以 root 身分執行時會拒絕一鍵清理（仍可單獨終止進程）。


### 安全與限制

**睡眠狀態加上高記憶體占用，不代表程序沒有用途或發生記憶體洩漏。** 500 MB / 0.1% 的門檻僅用於標記「候選程序」，請先確認實際用途。Unix 系統預設傳送 SIGTERM 請求正常結束；Windows 仍採立即終止，均可能造成未儲存資料遺失。單一與批次終止皆須在八秒內二次確認，執行前會重新檢查程序身分與擁有者，但仍無法保證完全排除 PID 重用的競態風險；無法確認擁有者時將拒絕終止。

窄終端機（50 欄起）會隱藏狀態欄，滑鼠點擊也可選擇程序。

## 測試與限制

已加入 `.github/workflows/ci.yml` 跨平台 CI 設定；請參閱[V4 發布前驗證紀錄](./V4_RELEASE_READINESS.zh-TW.md)。格式檢查目前為建議性質，舊版程式碼尚待完整 rustfmt 格式化。如果所在平台無法讀取程序擁有者，將採保守策略拒絕終止。

## 記憶體與效能分析（非破壞性）

此版本提供 1–21,600 秒的採樣範例，**不會終止任何程序**。執行後產生 CSV，包含掃描耗時及 Idlekiller 自身 RSS：

```bash
cargo run --locked --release --example profile_collector -- 3600 > profile.csv
python3 scripts/analyze_profile.py profile.csv
python3 -m unittest discover -s scripts -p 'test_*.py'
```

程式畫面右側的 `SCAN` / `S` 顯示背景工作完成一次掃描及轉換所花的時間（毫秒）。**RSS 成長不一定是洩漏，RSS 穩定也無法證明完全沒有洩漏**；應在相似的工作負載下多次測量並與 heap/handle profiler 交叉驗證。

V4 修正及驗證狀態請見 [V4 發布前驗證紀錄](./V4_RELEASE_READINESS.zh-TW.md)。
