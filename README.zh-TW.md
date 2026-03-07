[English](./README.md) | 繁體中文

# Idlekiller

一個基於 Rust 的 TUI 進程管理工具，專為識別與清理不必要消耗系統資源的進程而設計。

---

## 1️⃣ 如何安裝

### Windows

```powershell
# 1. 下載並解壓縮釋出檔
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller

# 2. 編譯（需先安裝 Rust）
cargo build --release

# 3. 將執行檔移至你偏好的資料夾 (例如 C:\Software\Idlekiller\)
copy target\release\idlekiller.exe C:\妳想放的路徑\Idlekiller\
```

### macOS

```bash
# 方式 A：從原始碼編譯
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller
cargo build --release
sudo cp target/release/idlekiller /usr/local/bin/

# 方式 B：打包成 .app（圖示啟動）
make app
# 生成 Idlekiller.app，拖到 Applications 即可
```

### Linux

```bash
# 1. 從原始碼編譯
git clone https://github.com/Poseidoncode/Idlekiller.git
cd Idlekiller
cargo build --release

# 2. 安裝到系統路徑
sudo cp target/release/idlekiller /usr/local/bin/
```

---

## 2️⃣ 如何使用

### 啟動程式

```bash
idlekiller
```

或在 macOS 上直接點擊 `Idlekiller.app`。

### 操作說明

| 按鍵                  | 功能                       |
| --------------------- | -------------------------- |
| ↑ / ↓ / **K** / **J** | 上下移動選擇進程           |
| **Enter** / **X**     | 終止所選進程 (Kill)        |
| **f** / **/**         | 根據名稱搜尋/過濾進程      |
| **Shift + K**         | **一鍵清理潛在資源浪費者** |
| **S**                 | 在瀏覽器搜尋該進程資訊     |
| **Q** / **Esc**       | 退出工具                   |

---

## 3️⃣ 智能識別系統

Idlekiller 會自動標示潛在的資源浪費者（顯示為**黃色**）：
- **觸發條件**：程序處於 `Sleeping` 或 `Idle` 狀態，且 CPU < 0.1%，但記憶體佔用超過 **50MB**。
- **處理建議**：如果您確定該程序目前不需要使用，可以使用 `Shift + K` 一鍵清理，以釋放系統資源。

---
