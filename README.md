# Idlekiller

一個基於 Rust 的 TUI (Terminal User Interface) 進程管理工具，專為識別與清理不必要消耗系統資源的進程而設計。

## 🚀 快速開始

### 前置要求
- 已安裝 [Rust](https://www.rust-lang.org/tools/install) 編譯環境 (Cargo)。

### 複製專案
```bash
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller
```

### 直接執行 (開發模式)
```bash
cargo run
# 或者使用 Makefile
make run
```

---

## 📦 打包與建置 (多平台支援)

本專案支援 **Windows**, **macOS**, 及 **Linux**。以下是各平台的建置方法：

### 通用建置 (推薦)
在專案根目錄執行以下指令，將會生成優化後的正式版執行檔：
```bash
cargo build --release
```
建置完成後的檔案路徑：
- **Windows**: `target/release/idlekiller.exe`
- **macOS / Linux**: `target/release/idlekiller`

### 使用 Makefile 簡化 (Mac / Linux)
如果你已安裝 `make` 工具，可以使用以下快捷指令：
```bash
make release
```

### macOS 專屬：打包成應用程式 (.app)
如果你想在 macOS 上將其作為一般的應用程式點擊執行：
```bash
make app
```
這會在根目錄生成 `Idlekiller.app`。

---

## 🛠️ 安裝與使用

### 安裝方式
這是一個攜帶式 (Portable) 的工具。你只需將編譯好的執行檔移動到你的 `PATH` 路徑下，或直接點擊執行：

- **Windows**: 將 `idlekiller.exe` 放到自訂資料夾並加入系統環境變數，或直接雙擊執行。
- **macOS**:
  - 終端機執行：`./target/release/idlekiller`
  - 應用程式執行：直接點擊 `Idlekiller.app`
- **Linux**: 將 `idlekiller` 移動至 `/usr/local/bin/`：
  ```bash
  sudo cp target/release/idlekiller /usr/local/bin/
  ```

### 使用說明
- **方向鍵 / J, K**: 上下移動選擇進程。
- **Enter / K**: 終止所選進程 (Kill)。
- **S**: 在網頁瀏覽器中搜尋該進程的詳細資訊（幫助判斷是否為系統關鍵進程）。
- **Q / Esc**: 退出工具。

---

## 📄 授權
MIT License
