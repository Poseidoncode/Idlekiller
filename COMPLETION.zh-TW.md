# Idlekiller — 第二階段改造與驗證紀錄

基礎版本：前次交付的 `Idlekiller_optimized_review.zip`。

## 已實作

1. **程序收集在背景執行緒**：`src/collector.rs` 擁有唯一 `sysinfo::System`，每秒左右掃描並覆寫最新快照槽（容量 1），避免 UI 卡住或積壓舊快照。UI 以 100ms 間隔非阻塞取得最新資料。
2. **單一程序資料 + 索引視圖**：`src/app.rs` 僅保存一份 `Vec<ProcessInfo>`，排序篩選是 `Vec<usize>`，不再每次深度複製 `ProcessInfo` 的名稱。`src/ui.rs` 仍只格式化可視列。
3. **終止操作也由背景執行緒處理**：UI 二次確認只提交佇列請求，工作執行緒定向刷新 PID、再次核對名稱、啟動時間、程序擁有者及保護規則，傳回各結果統計。此操作採有界佇列，單次只能有一個待執行請求。
4. **更保守的批次規則**：CPU 至少經過兩次有效取樣；程序須連續三個有效樣本滿足低 CPU、睡眠及記憶體門檻，才會顯示候選警示。執行前會再次檢查是否仍為同一程序、擁有者與睡眠／記憶體狀態，且樣本須新鮮。一次批次至多 4 筆；現在依目前的篩選結果操作，不會偷偷納入畫面外的其他查詢結果。
5. **跨平台擁有者來源**：Unix 取有效 UID；Windows 使用 `Process::user_id()`，因 `effective_user_id()` 在 Windows 必然為 `None`。資料仍不可取得時拒絕發送終止訊號。
6. **回歸測試與 CI**：`src/collector.rs` 的 FakeSource 測試不發送真實終止訊號，涵蓋 PID 重用、權限不明／不同、保護程序、候選狀態更新及命令逾時。`src/app.rs` 測試篩選／排序索引、Unicode。新增 GitHub Actions Linux/macOS/Windows 三系統檢查與測試流程。

## 重要限制

- **此環境沒有 rustc、cargo、rustfmt，且網路 DNS 無法連往 Rust 下載服務**。無法在本機執行 `cargo check/test/clippy/fmt`、release 編譯、跨平台整合或動態記憶體剖析；本版本必須標示為 **尚未編譯驗證**。
- `PID + 名稱 + 啟動時間` 的重新核對只能縮小競態窗口，不能完全替代 Linux pidfd / Windows PROCESS HANDLE 等穩定核心程序 handle；在實際發出訊號前 PID 仍可能被非常快速地重用。
- Windows 的 CPU 狀態與擁有者資訊需實機驗證；若 `user_id()` 無法取得，程式將拒絕終止。
- 大量程序每次 OS 重新掃描仍必須配置一份新快照；新增索引可消除 UI 深度複製，但不能保證完全沒有配置壓力或記憶體持續增加。
- **低 CPU 加高 RAM 不等於記憶體洩漏或可安全終止**。SIGTERM 是否成功送出不代表程序已正常退出；任何終止都可能造成未存檔資料損失。

## 已完成的非編譯驗證

- Bash：`bash -n install.sh` 通過。
- 靜態詞法：Pygments Rust lexer 未發現 Error token；所檢查的 Rust 原始碼括號、方括號、大括號均配對。**此項不等於編譯通過**。
- GitHub Actions：`.github/workflows/ci.yml` 由 YAML parser 成功解析。
- `sysinfo 0.38.3` 的 `Process`、CPU 更新及擁有者 API 已對照對應版本的官方文件檢查；特別確認 Windows `effective_user_id()` 不可用的限制。

## 專案下載後的驗證命令

```bash
cargo fmt --all
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
cargo run --locked
```

在個別 OS 請先手動確認 UI 可顯示與排序，然後才測試「無實際發出終止訊號」的預覽。實際終止整合測試必須在獨立、可重建的虛擬機及專門建立的測試程序中進行；切勿選擇日常工作程序。
