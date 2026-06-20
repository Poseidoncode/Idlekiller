.PHONY: all help app

all: help

help:
	@echo "Available commands:"
	@echo "  make        - Show this help"
	@echo "  make app    - Build release + package as macOS .app + copy to /Applications"
	@echo ""
	@echo "Or use cargo directly: cargo run / cargo build --release / cargo clean"

app:
	cargo build --release
	@echo "Packaging macOS app..."
	mkdir -p Idlekiller.app/Contents/MacOS
	mkdir -p Idlekiller.app/Contents/Resources
	cp target/release/idlekiller Idlekiller.app/Contents/MacOS/
	# Create a launcher script so the TUI opens in Terminal.app when double-clicked
	printf '#!/bin/bash\ncd "$$(dirname "$$0")"\nopen -a Terminal "$$(dirname "$$0")/idlekiller"\n' > Idlekiller.app/Contents/MacOS/idlekiller-launcher
	chmod +x Idlekiller.app/Contents/MacOS/idlekiller-launcher
	printf '<?xml version="1.0" encoding="UTF-8"?>\n\
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n\
<plist version="1.0">\n\
<dict>\n\
	<key>CFBundleExecutable</key>\n\
	<string>idlekiller-launcher</string>\n\
	<key>CFBundleIdentifier</key>\n\
	<string>com.poseidoncode.idlekiller</string>\n\
	<key>CFBundleName</key>\n\
	<string>Idlekiller</string>\n\
	<key>CFBundleVersion</key>\n\
	<string>1.0</string>\n\
	<key>CFBundlePackageType</key>\n\
	<string>APPL</string>\n\
</dict>\n\
</plist>\n' > Idlekiller.app/Contents/Info.plist
	# Remove quarantine so macOS doesn't block it
	xattr -dr com.apple.quarantine Idlekiller.app 2>/dev/null || true
	@echo "Copying to /Applications... (you may be prompted for your password)"
	sudo rm -rf /Applications/Idlekiller.app 2>/dev/null; sudo cp -R Idlekiller.app /Applications/ && \
	  sudo xattr -dr com.apple.quarantine /Applications/Idlekiller.app 2>/dev/null || true
	@echo "Done! Find Idlekiller in Launchpad or /Applications/"
