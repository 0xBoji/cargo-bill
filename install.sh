#!/bin/bash
set -e

echo "Downloading cargo-bill binary..."

# Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" = "Linux" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        ASSET="cargo-bill-linux-amd64"
    elif [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
        ASSET="cargo-bill-linux-arm64"
    else
        echo "Unsupported Linux architecture: $ARCH"
        exit 1
    fi
elif [ "$OS" = "Darwin" ]; then
    if [ "$ARCH" = "x86_64" ]; then
        ASSET="cargo-bill-macos-amd64"
    elif [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
        ASSET="cargo-bill-macos-arm64"
    else
        echo "Unsupported macOS architecture: $ARCH"
        exit 1
    fi
else
    echo "Unsupported OS: $OS"
    exit 1
fi

DOWNLOAD_URL="https://github.com/0xBoji/cargo-bill/releases/latest/download/$ASSET"

echo "Fetching from $DOWNLOAD_URL"
curl -L -f "$DOWNLOAD_URL" -o cargo-bill
chmod +x cargo-bill

echo "Installing cargo-bill to /usr/local/bin... (this might require sudo context)"
sudo mv cargo-bill /usr/local/bin/

echo "Success! Run 'cargo-bill --help' to get started."
