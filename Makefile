.PHONY: help run build release clean app

# 預設執行 help
all: help

# 顯示幫助資訊
help:
	@echo "可用指令："
	@echo "  make         - 顯示此幫助資訊"
	@echo "  make help    - 顯示此幫助資訊"
	@echo "  make run     - 執行開發版直接運行 (Windows/macOS/Linux)"
	@echo "  make build   - 編譯開發版"
	@echo "  make release - 編譯正式版（最佳化）"
	@echo "  make clean   - 清除所有的編譯暫存檔"
	@echo "  make app     - 編譯正式版並幫你打包成 macOS 的 Idlekiller.app (僅限 macOS)"

# 執行開發版
run:
	cargo run

# 編譯開發版
build:
	cargo build

# 編譯正式版（最佳化）
release:
	cargo build --release

# 清除編譯暫存
clean:
	cargo clean

# 打包成 macOS 應用程式
app: release
	@echo "打包 macOS 應用程式..."
	mkdir -p Idlekiller.app/Contents/MacOS
	mkdir -p Idlekiller.app/Contents/Resources
	cp target/release/idlekiller Idlekiller.app/Contents/MacOS/
	@echo "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
	<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
	<plist version=\"1.0\">\n\
	<dict>\n\
		<key>CFBundleExecutable</key>\n\
		<string>idlekiller</string>\n\
		<key>CFBundleIdentifier</key>\n\
		<string>com.yourdomain.idlekiller</string>\n\
		<key>CFBundleName</key>\n\
		<string>Idlekiller</string>\n\
		<key>CFBundleVersion</key>\n\
		<string>1.0</string>\n\
		<key>CFBundlePackageType</key>\n\
		<string>APPL</string>\n\
	</dict>\n\
	</plist>" > Idlekiller.app/Contents/Info.plist
	@echo "打包完成！可以直接點擊 Idlekiller.app 或執行 open Idlekiller.app"
