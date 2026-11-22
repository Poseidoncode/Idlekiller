#!/usr/bin/env bash
set -euo pipefail

REPO="Poseidoncode/Idlekiller"
REF="${IDLEKILLER_REF:-main}"
# Allow only safe ref characters so REF cannot escape into the URL or the
# extraction directory below.
if [[ ! "$REF" =~ ^[A-Za-z0-9._/-]{1,128}$ ]] || [[ "$REF" == *".."* ]]; then
  echo "Invalid IDLEKILLER_REF: '$REF' (allowed: letters, digits, . _ - /)" >&2
  exit 1
fi
# /archive/<ref> resolves branches, tags and commit SHAs alike.
# For a reproducible install, pin a tag or commit and set IDLEKILLER_SHA256
# (a checksum of a moving branch such as "main" goes stale on every push).
URL="https://github.com/$REPO/archive/$REF.tar.gz"
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

  local extract="$TMP_DIR/src"
  mkdir -p "$extract"
  tar -xzf "$archive" -C "$extract"
  # The top-level directory name depends on the ref (slashes become dashes,
  # a leading "v" is dropped for tags), so locate it instead of guessing.
  local dirs=("$extract"/*/)
  if [ "${#dirs[@]}" -ne 1 ] || [ ! -f "${dirs[0]}Cargo.toml" ]; then
    echo "Unexpected archive layout for ref '$REF'" >&2
    exit 1
  fi
  cd "${dirs[0]}"

  echo "Building..."
  cargo build --release --locked

  if [ -w /usr/local/bin ] 2>/dev/null; then
    INSTALL_DIR="/usr/local/bin"
  else
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "$INSTALL_DIR"
  fi

  install -m 755 target/release/idlekiller "$INSTALL_DIR/idlekiller"

  echo "Installed to $INSTALL_DIR/idlekiller"

  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo "$INSTALL_DIR is not on your PATH. Add this line to your shell profile:"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    if [ "${IDLEKILLER_MODIFY_PATH:-0}" = "1" ]; then
      case "${SHELL:-}" in
        */zsh) rc="${HOME}/.zshrc" ;;
        */bash) rc="${HOME}/.bashrc" ;;
        *) rc="" ;;
      esac
      if [ -n "$rc" ] && ! grep -qxF "export PATH=\"$INSTALL_DIR:\$PATH\"" "$rc" 2>/dev/null; then
        echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$rc"
        echo "Added $INSTALL_DIR to PATH in $rc"
      fi
    else
      echo "(set IDLEKILLER_MODIFY_PATH=1 to let this installer edit your shell profile)"
    fi
  fi
}

main
