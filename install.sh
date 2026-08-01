#!/usr/bin/env bash
set -euo pipefail

REPO="Poseidoncode/Idlekiller"
REF="${IDLEKILLER_REF:-main}"
URL="https://github.com/$REPO/archive/refs/heads/$REF.tar.gz"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

main() {
  if ! command -v cargo >/dev/null 2>&1; then
    cat <<'EOF' >&2
Rust is required but not found. Install it from:
  https://rustup.rs
Or run:
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
EOF
    exit 1
  fi

  echo "Downloading Idlekiller source (ref: $REF)..."
  local archive="$TMP_DIR/idlekiller.tar.gz"
  curl -fsSL "$URL" -o "$archive"

  if [ -n "${IDLEKILLER_SHA256:-}" ]; then
    echo "Verifying checksum..."
    if command -v sha256sum >/dev/null 2>&1; then
      echo "$IDLEKILLER_SHA256  $archive" | sha256sum -c -
    elif command -v shasum >/dev/null 2>&1; then
      echo "$IDLEKILLER_SHA256  $archive" | shasum -a 256 -c -
    else
      echo "IDLEKILLER_SHA256 is set but no sha256 tool is available" >&2
      exit 1
    fi
  fi

  tar -xzf "$archive" -C "$TMP_DIR"
  cd "$TMP_DIR/Idlekiller-$REF"

  echo "Building..."
  cargo build --release

  if [ -w /usr/local/bin ] 2>/dev/null; then
    INSTALL_DIR="/usr/local/bin"
  else
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "$INSTALL_DIR"
  fi

  install -m 755 target/release/idlekiller "$INSTALL_DIR/idlekiller"

  echo "Installed to $INSTALL_DIR/idlekiller"

  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    case "${SHELL:-}" in
      */zsh) rc="${HOME}/.zshrc" ;;
      */bash) rc="${HOME}/.bashrc" ;;
      *) rc="" ;;
    esac
    if [ -n "$rc" ] && ! grep -qxF "export PATH=\"$INSTALL_DIR:\$PATH\"" "$rc" 2>/dev/null; then
      echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$rc"
      echo "Added $INSTALL_DIR to PATH in $rc"
    else
      echo "Add $INSTALL_DIR to your PATH if 'idlekiller' is not found:"
      echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    fi
  fi
}

main
