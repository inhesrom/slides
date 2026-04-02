#!/bin/bash
set -e

INSTALL_DIR="${SLIDES_INSTALL_DIR:-$HOME/.local/bin}"

echo "Building slides (release)..."
cargo build --release

VERSION=$(cargo metadata --no-deps --format-version 1 | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4)

mkdir -p "$INSTALL_DIR"
rm -f "$INSTALL_DIR/slides"
cp target/release/slides "$INSTALL_DIR/slides"
chmod +x "$INSTALL_DIR/slides"

echo "Installed slides ${VERSION} to ${INSTALL_DIR}/slides"

case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *) echo "Note: ${INSTALL_DIR} is not in your PATH. Add it with:"
     echo "  export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac
