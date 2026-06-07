#!/bin/bash

# Paper2Codes Backend Startup Script
# This script starts the Paper2Codes backend API server

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CORE_DIR="$PROJECT_ROOT/Paper2Codes-Core"
BACKEND_PID_FILE="$PROJECT_ROOT/.backend.pid"
BACKEND_LOG_FILE="$PROJECT_ROOT/logs/backend.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create logs directory if it doesn't exist
mkdir -p "$PROJECT_ROOT/logs"

# Function to print colored messages
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if backend is already running
if [ -f "$BACKEND_PID_FILE" ]; then
    PID=$(cat "$BACKEND_PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        print_warn "Backend is already running (PID: $PID)"
        exit 1
    else
        print_info "Removing stale PID file"
        rm -f "$BACKEND_PID_FILE"
    fi
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo is not installed. Please install Rust first."
    exit 1
fi

# Check if backend binary exists, if not build it
BACKEND_BINARY="$CORE_DIR/target/release/paper2codes"
if [ ! -f "$BACKEND_BINARY" ]; then
    print_info "Backend binary not found. Building release version..."
    cd "$CORE_DIR"
    cargo build --release --features api
    if [ $? -ne 0 ]; then
        print_error "Failed to build backend"
        exit 1
    fi
    print_info "Backend built successfully"
fi

# Check if config file exists
CONFIG_DIR="$HOME/.config/paper2codes"
CONFIG_FILE="$CONFIG_DIR/config.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    print_warn "Config file not found at $CONFIG_FILE"
    print_info "Creating config directory and copying example config..."
    mkdir -p "$CONFIG_DIR"
    if [ -f "$CORE_DIR/config.example.toml" ]; then
        cp "$CORE_DIR/config.example.toml" "$CONFIG_FILE"
        print_info "Example config copied. Please edit $CONFIG_FILE with your settings."
    else
        print_error "Example config file not found"
        exit 1
    fi
fi

# Set default host and port
HOST="${BACKEND_HOST:-127.0.0.1}"
PORT="${BACKEND_PORT:-8080}"

print_info "Starting Paper2Codes backend on $HOST:$PORT"
print_info "Logs will be written to: $BACKEND_LOG_FILE"

# Set environment variable to disable authentication for development
export PAPER2CODES_DISABLE_AUTH="${PAPER2CODES_DISABLE_AUTH:-true}"
if [ "$PAPER2CODES_DISABLE_AUTH" = "true" ]; then
    print_info "Authentication is DISABLED (development mode)"
    print_warn "⚠ WARNING: Running without authentication. Do not use in production!"
else
    print_info "Authentication is ENABLED"
fi

# Start the backend server
cd "$CORE_DIR"
PAPER2CODES_DISABLE_AUTH="$PAPER2CODES_DISABLE_AUTH" "$BACKEND_BINARY" api --host "$HOST" --port "$PORT" > "$BACKEND_LOG_FILE" 2>&1 &
BACKEND_PID=$!

# Save PID
echo "$BACKEND_PID" > "$BACKEND_PID_FILE"

# Wait a moment to check if it started successfully
sleep 2

if ps -p "$BACKEND_PID" > /dev/null 2>&1; then
    print_info "Backend started successfully (PID: $BACKEND_PID)"
    print_info "API available at: http://$HOST:$PORT"
    print_info "Health check: http://$HOST:$PORT/api/health"
    print_info "To stop the backend, run: ./scripts/stop-backend.sh"
else
    print_error "Backend failed to start. Check logs: $BACKEND_LOG_FILE"
    rm -f "$BACKEND_PID_FILE"
    exit 1
fi

