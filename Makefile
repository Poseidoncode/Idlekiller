.PHONY: all help app

all: help

help:
	@echo "Available commands:"
	@echo "  make        - Show this help"
	@echo "  make app    - Build release + package as macOS Idlekiller.app (macOS only)"
	@echo ""
	@echo "Or use cargo directly: cargo run / cargo build --release / cargo clean"

app:
	cargo build --release
	@echo "Packaging macOS app..."
	mkdir -p Idlekiller.app/Contents/MacOS
	mkdir -p Idlekiller.app/Contents/Resources
	cp target/release/idlekiller Idlekiller.app/Contents/MacOS/
	printf '<?xml version="1.0" encoding="UTF-8"?>\n\
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n\
<plist version="1.0">\n\
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
</plist>\n' > Idlekiller.app/Contents/Info.plist
	@echo "Done! Open Idlekiller.app or run: open Idlekiller.app"
