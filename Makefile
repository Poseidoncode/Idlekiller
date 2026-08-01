.PHONY: all help run test check build app clean

all: help

help:
	@echo "Available commands:"
	@echo "  make run    - Run the TUI application locally (cargo run)"
	@echo "  make test   - Run unit tests (cargo test)"
	@echo "  make check  - Check code compilation (cargo check)"
	@echo "  make build  - Build debug binary (cargo build)"
	@echo "  make app    - Build release + package Idlekiller.app (drag to /Applications)"
	@echo "  make clean  - Run cargo clean and remove local Idlekiller.app"

run:
	cargo run

test:
	cargo test

check:
	cargo check

build:
	cargo build

app:
	cargo build --release
	@echo "Packaging macOS app..."
	rm -rf Idlekiller.app
	mkdir -p Idlekiller.app/Contents/MacOS
	mkdir -p Idlekiller.app/Contents/Resources
	cp target/release/idlekiller Idlekiller.app/Contents/MacOS/
	# Launcher that opens the bundled .command in Terminal.app
	@printf '#!/bin/bash\nDIR="$$(cd "$$(dirname "$$0")" && pwd)"\nopen -a Terminal "$$DIR/Idlekiller.command"\n' > Idlekiller.app/Contents/MacOS/idlekiller-launcher
	chmod +x Idlekiller.app/Contents/MacOS/idlekiller-launcher
	# The .command file actually runs the idlekiller binary
	@printf '#!/bin/bash\nDIR="$$(cd "$$(dirname "$$0")" && pwd)"\ncd "$$DIR"\nexec ./idlekiller\n' > Idlekiller.app/Contents/MacOS/Idlekiller.command
	chmod +x Idlekiller.app/Contents/MacOS/Idlekiller.command
	@printf '<?xml version="1.0" encoding="UTF-8"?>\n\
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n\
<plist version="1.0">\n\
<dict>\n\
\t<key>CFBundleExecutable</key>\n\
\t<string>idlekiller-launcher</string>\n\
\t<key>CFBundleIdentifier</key>\n\
\t<string>com.poseidoncode.idlekiller</string>\n\
\t<key>CFBundleName</key>\n\
\t<string>Idlekiller</string>\n\
\t<key>CFBundleVersion</key>\n\
\t<string>1.0</string>\n\
\t<key>CFBundlePackageType</key>\n\
\t<string>APPL</string>\n\
</dict>\n\
</plist>\n' > Idlekiller.app/Contents/Info.plist
	@echo "Created Idlekiller.app. Drag it to /Applications to install."

clean:
	cargo clean
	rm -rf Idlekiller.app
