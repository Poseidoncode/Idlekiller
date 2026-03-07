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

# 3. 將執行檔移至 PATH 或任意資料夾
copy target\release\idlekiller.exe C:\Program Files\Idlekiller\
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

| 按鍵                  | 功能                   |
| --------------------- | ---------------------- |
| ↑ / ↓ / **K** / **J** | 上下移動選擇進程       |
| **Enter**             | 終止所選進程 (Kill)    |
| **S**                 | 在瀏覽器搜尋該進程資訊 |
| **Q** / **Esc**       | 退出工具               |

---

## 📄 授權

MIT License
