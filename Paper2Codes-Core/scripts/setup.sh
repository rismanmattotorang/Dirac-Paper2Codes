#!/bin/bash
# Paper2Codes Setup Script

set -e

echo "========================================="
echo "Paper2Codes Setup"
echo "========================================="
echo

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi
echo "✓ Rust is installed"

# Check Rust version
RUST_VERSION=$(rustc --version | awk '{print $2}')
echo "  Version: $RUST_VERSION"

# Create necessary directories
echo
echo "Creating directories..."
mkdir -p ~/.config/paper2codes
mkdir -p ~/.cache/paper2codes
mkdir -p ~/.local/share/paper2codes/logs
mkdir -p ./output
echo "✓ Directories created"

# Copy example config if doesn't exist
if [ ! -f ~/.config/paper2codes/config.toml ]; then
    echo
    echo "Copying example configuration..."
    cp config.example.toml ~/.config/paper2codes/config.toml
    echo "✓ Configuration file created at ~/.config/paper2codes/config.toml"
    echo "  Please edit this file to add your API keys"
else
    echo
    echo "⚠  Configuration file already exists at ~/.config/paper2codes/config.toml"
fi

# Build project
echo
echo "Building Paper2Codes..."
cargo build --release
echo "✓ Build complete"

# Check for SurrealDB
echo
echo "Checking for SurrealDB..."
if command -v surreal &> /dev/null; then
    echo "✓ SurrealDB is installed"
    SURREAL_VERSION=$(surreal version | head -n 1)
    echo "  Version: $SURREAL_VERSION"
else
    echo "⚠  SurrealDB is not installed (optional)"
    echo "  Install from: https://surrealdb.com/docs/installation"
    echo "  Or disable storage in config: storage.enabled = false"
fi

echo
echo "========================================="
echo "Setup Complete!"
echo "========================================="
echo
echo "Next steps:"
echo "1. Edit ~/.config/paper2codes/config.toml to add your API keys"
echo "2. (Optional) Start SurrealDB: surreal start --user root --pass root"
echo "3. Run Paper2Codes: ./target/release/paper2codes --help"
echo
echo "For more information, see README.md"

