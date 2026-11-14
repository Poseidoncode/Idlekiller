# Idlekiller 靜態審查、修改摘要與後續驗證
> **歷史紀錄：** 本文件是第一階段審查。後續已實作背景收集器、索引視圖、模擬終止測試與跨平台 CI，詳見 [COMPLETION.zh-TW.md](./COMPLETION.zh-TW.md)。以下「尚未實作」係指第一階段狀態，並非此 ZIP 的現況。


審查基準：使用者提供的 `Idlekiller.zip`。本報告所稱的「原始位置」均指壓縮檔內的原始程式碼行號；修改後行號可能有所變動。

## 範圍與可信度

- 已檢閱：`src/main.rs`、`src/app.rs`、`src/ui.rs`、`Cargo.toml`、`Cargo.lock`、測試/benchmark、`install.sh`、`install.ps1`、README、Makefile。
- 已套用：程式邏輯修正、TUI 操作修正、部分配置/渲染優化、測試與安裝腳本更新。
- 已驗證：Bash 腳本語法（`bash -n`）、Python 驗證表格欄寬與可視區索引不變條件、Rust lexer 層級的括號/字面量檢查；**這些不等於 Rust 編譯**。
- **未驗證**：此環境沒有 `rustc`、`cargo`、`rustfmt`、`pwsh`，也無法透過網路安裝；未執行 `cargo test/check/clippy`，未在 Windows/macOS/Linux 編譯或測量 RSS/CPU、未進行真實程序終止測試。
- 結論是「找到並修復已識別的一部分問題」，不是「保證找出所有 bug、記憶體洩漏或競態」。

## 重要問題與處置

| 優先 | 原始位置 | 問題／影響 | 修正狀態 |
|---|---|---|---|
| P0 | `src/app.rs:544–624` | 第一次 `K` 預覽 PID，第二次重新計算候選清單，**新出現的程序也可能被終止**，而使用者未確認那些目標。 | **已修改**：鎖定首次預覽的 PID、名稱、起始時間，8 秒內再次確認；重新取得 OS 資料並逐項檢查，不增加新目標。仍有 PID 重用競態限制。 |
| P0 | `src/app.rs:327–439` | `Enter/x` 單次按鍵就發送終止要求，容易誤操作。 | **已修改**：二次確認、保護身分立即拒絕、再次檢查程序資訊，另顯示確認對話框。 |
| P0 | `src/app.rs:685–691` | `current UID=None` 且 `process UID=None` 時回傳 `true`，屬於失敗開放式授權。 | **已修改**：未知擁有者一律拒絕；特定平台若無法提供 UID，可能導致終止功能被停用，需跨平台測試。 |
| P0/P1 | `src/app.rs:365–387, 599–620` | 原有重用 PID 檢查比對的是 `sysinfo::System` 內可能過期的快照，未在發訊號前強制重讀 OS。 | **部分緩解**：發送前以 `ProcessesToUpdate::Some` 定向重新整理，再核對名稱、啟動時間、UID、保護規則。對極短時間內、同名且啟動時間解析度不足的重用 PID，**仍無絕對保證**。 |
| P1 | `src/app.rs:242–246` | 僅以 refresh 次數當作有效 CPU 採樣；若兩次連續執行而無時間間隔，CPU 可能被錯判為 0%。 | **已修改**：至少 250ms 的時間間隔才增加有效樣本計數。仍建議採持續多個樣本的判斷策略。 |
| P1 | `src/ui.rs:333–413` | 每次重畫都格式化並配置**全部**程序列，而非只處理可見列；N 遠大於畫面高度時浪費大量配置。 | **已修改**：建立可視範圍內的 `Row`，維持全域選取索引與捲動偏移。 |
| P1 | `src/app.rs:78–80, 128–130` | `System::new_all()` 後立刻 `refresh()`，啟動期間重複昂貴的 OS 資料收集。 | **已修改**：初始化改為 `System::new()`，首次掃描後才取得自身 UID。 |
| P1 | `src/app.rs:153–161, 188–196` | 程序名稱重複轉換與配置；`all_processes` 每次被整個替換。 | **部分改善**：每個程序的名稱只解碼一次、保留 `all_processes` 的 Vec 容量、使用不穩定排序；仍存在 `all_processes -> processes` 的深度字串複製，後續應改為索引視圖。 |
| P1 | `src/ui.rs:82–111` | 點擊座標與真實 `.column_spacing(1)` 不一致，名稱欄寬計算也與 `Constraint::Min` 不一致。 | **已修改**：以共用欄寬函式同時產生表格與 hit-test；另支援點選資料列。 |
| P1 | `src/main.rs:50–58` 與 `src/ui.rs:117–125` | 宣稱支援 50x12，但固定行高與表格最小寬度不相容，畫面在窄視窗易被裁切。 | **已修改**：支援 50x16、窄版隱藏 STATUS 欄、縮小表格最小高度，尺寸不足時顯示調整提示。 |
| P1 | `src/ui.rs:337–340` | 黃色警示未考慮被保護程序或程序擁有者，造成畫面上「可清理」與實際清理規則不一致。 | **已修改**：僅標示符合權限、保護及門檻的候選程序。**候選≠記憶體洩漏**。 |
| P1 | `src/app.rs:491–542` | `xdg-open/open/rundll32` 子程序若遲不退出，其 `Child` handles 可能持續累積。 | **部分改善**：每次開啟前嘗試回收，最多保留 8 個等待回收的子程序；持續 `try_wait()` 失敗時依然可能保留 handle。 |
| P2 | `src/ui.rs:489–504` | 失敗、拒絕執行的提示都以綠色成功圖示顯示。 | **已修改**：中性資訊提示；建議未來明確定義 Success/Warning/Error 等級。 |
| P2 | `src/main.rs:108–110` | 所有 `KeyEventKind::Repeat` 都被跳過，長按上下鍵無法順暢連續移動。 | **已修改**：導覽與 Backspace 允許 Repeat，終止動作仍只接受 Press。 |
| P2 | `src/main.rs:127–130` | 篩選欄位的字元數無上限，持續輸入會造成配置、排序成本增長。 | **已修改**：輸入上限 128 個 Unicode 字元；仍可再做搜尋 debounce。 |
| P2 | `install.ps1:42–46` | PowerShell 呼叫外部 `cargo` 失敗，不一定會因 `$ErrorActionPreference='Stop'` 而中斷，可能繼續安裝舊檔案。 | **已修改**：明確檢查 `$LASTEXITCODE`，兩平台都使用 `cargo build --locked`。 |
| P2 | `README.md:19`、`README.zh-TW.md:19` | README 寫 Rust 1.85，原 `Cargo.toml` 設 1.86，但鎖定的 `sysinfo 0.38.3` 最低 Rust 為 1.88。 | **已修改**：Cargo 與 README 均更新為 1.88；仍需在 Rust 1.88 工具鏈編譯驗證所有相依套件。 |

### 尚未完全排除的可靠性／安全性風險

1. **「Sleeping + RAM > 500MB + CPU < 0.1%」無法推斷記憶體洩漏**：瀏覽器背景分頁、等待 I/O 的資料庫、快取服務、IDE 等都可能符合。批次清理應改為「候選檢視 → 顯示風險／擁有者／命令列 → 選取要操作的項目」，並預設只執行模擬 (dry-run)。
2. **缺乏 OS 層級穩定程序 handle**：PID + 名稱 + 啟動時間只是緩解競態；Linux 可調查 `pidfd`，Windows 使用帶存取權限檢查的 Process HANDLE；其他平台也需要相應的生命週期驗證。
3. **監控與 UI 在相同執行緒**：`refresh_processes_specifics`、排序、查詢與系統資訊刷新在事件迴圈中同步執行。程序數量龐大或 OS 呼叫停滯時，鍵盤與畫面仍可能卡住。
4. **無持續記憶體成長證據**：Rust 的 ownership 並不排除 cache、child handle 或向量容量持續成長。需要 1–6 小時測試與 RSS/heap profiler 才能區分洩漏、合理快取及配置抖動。
5. **批次終止缺乏使用者逐項確認與詳細結果**：已改成鎖定候選快照，但仍是全部執行。下一版應提供勾選、排除、顯示失敗原因及操作日誌。
6. **Unix 改為優先發送 SIGTERM**，可讓一般程序處理正常結束；Windows 仍為強制終止。發出終止訊號並不等於程序已退出，應於 UI 顯示「已送出要求」並再次觀察狀態；不要虛報「成功清理」。
7. **名稱保護清單是 heuristic，不是權限模型**：不能保證所有系統服務、容器、桌面服務都已列入。應引入白名單/拒絕清單可配置規則與操作前權限檢查，對不明程序採保守策略。
8. **目前 README 的遠端腳本安裝方式有供應鏈信任風險**：`curl | bash` / `irm | iex` 便利但無預設固定版本或簽章校驗。建議提供版本固定、簽章或 checksum 明確驗證的 release 安裝流程。

## 建議的下一階段架構（尚未實作）

```text
┌───────────────────────────┐
│ OS process adapters       │ ← sysinfo + platform-specific handles
│ Linux / macOS / Windows   │
└─────────────┬─────────────┘
              │ bounded, latest-only snapshot queue
┌─────────────▼─────────────┐
│ Collector worker          │ ← adaptive polling, errors, profiling metrics
└─────────────┬─────────────┘
              │ immutable snapshots + timestamps
┌─────────────▼─────────────┐
│ Controller / AppState      │ ← selection, filter, sort index, UX state
│ TerminationPlan FSM        │ ← preview / confirm / expired / executing / result
└─────────────┬─────────────┘
              │ lightweight view model
┌─────────────▼─────────────┐
│ Ratatui renderer           │ ← visible rows only, responsive columns, modal
└───────────────────────────┘
```

**建議拆檔：** `collector.rs`（OS 讀取）、`process.rs`（資料型別/身分）、`policy.rs`（保護與清理候選規則）、`actions.rs`（確認狀態機與執行結果）、`state.rs`（選擇與過濾排序）、`ui/`（版面與輸入座標），`main.rs` 只保留終端機生命週期和事件路由。設計 `ProcessSource` trait 與 `FakeProcessSource` 以避免測試真的終止程序。

**資料模型建議：** 不再保存兩份 `Vec<ProcessInfo>`（每秒複製名稱和 status），而是保存唯一 `Vec<ProcessInfo>` 快照，篩選排序只維護 `Vec<usize>` 或快照 ID 的索引。若採背景執行緒，以容量 1 的 latest-only channel 丟棄過期快照，避免消費端卡住時無限排隊。

## 效能假說與量測方法

- 原版 UI 在每次重畫時格式化 **N** 列；本修改版格式化約 **min(N, 畫面可見列數)** 列。例：1000 個程序、可見 30 列時，**理論格式化列數從 1000 降至 30**（約 97% 減少），但這不是測得的 CPU、記憶體或 FPS 改善。
- 仍待優化：`apply_filter()` 每次都複製/排序全部匹配的 `ProcessInfo`；`is_protected_name()` 每次執行 Unicode 轉小寫與分詞；高頻鍵盤輸入可能反覆排序。
- 加上 `Instant` 量測 `refresh_processes_specifics`、`apply_filter`、`terminal.draw` 和事件延遲，觀察 p50/p95/p99；分別在 100、1000、5000 個程序與 50/80/120 欄終端機尺寸下量測。
- 連續執行 1–6 小時，追蹤程序 RSS、heap allocation rate、CPU、檔案描述符/handles、child process 數量。檢查空閒時 RSS 是否持續單調成長，以及頻繁開啟搜尋／改變篩選時是否產生異常堆積。

## 建議在本機執行的驗證命令（尚未執行）

```bash
cargo fmt --all
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --locked --release
cargo run --locked
```

> 危險操作的整合測試必須使用隔離、可重建的測試程序和專用帳號，不可用日常工作程序或系統服務作測試目標。

### 必須補齊的測試矩陣

- `TerminationPlan`：一次按鍵不終止、二次按鍵成功、8 秒超時、切換選取/篩選/排序取消、目標已退出、PID 重用、權限不明、root 拒絕批次、預覽後新出現候選不被納入。
- `Collector`：空清單、1 萬程序、高頻程序創建/結束、讀取失敗、CPU 前兩次取樣不可靠、記憶體值極端與 NaN。
- `UI`：50x16、71x16、72x16、120x40、縮小後恢復、Unicode 名稱、控制字元、滑鼠欄位點擊/列點擊、視窗底部捲動、確認文字換行。
- `OS`：Linux/macOS/Windows 編譯與執行、不同使用者 UID、無法存取程序、同名程序、子程序退出/launcher 停滯。

## 合併準則

目前的修改版應視為 **待 CI/人工驗證的修正候選**，不是已可直接上線的正式版。通過 Rust 1.88 或更新版本工具鏈測試、至少一輪非破壞性 UI 整合測試、跨平台冒煙測試與記憶體壓力測試後，才適合合併發布。
