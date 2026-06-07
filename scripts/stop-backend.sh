#!/bin/bash

# Paper2Codes Backend Stop Script
# This script stops the Paper2Codes backend API server

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BACKEND_PID_FILE="$PROJECT_ROOT/.backend.pid"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

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

# Check if PID file exists
if [ ! -f "$BACKEND_PID_FILE" ]; then
    print_warn "Backend PID file not found. Backend may not be running."
    exit 0
fi

PID=$(cat "$BACKEND_PID_FILE")

# Check if process is running
if ps -p "$PID" > /dev/null 2>&1; then
    print_info "Stopping backend (PID: $PID)..."
    kill "$PID"
    
    # Wait for process to stop
    for i in {1..10}; do
        if ! ps -p "$PID" > /dev/null 2>&1; then
            print_info "Backend stopped successfully"
            rm -f "$BACKEND_PID_FILE"
            exit 0
        fi
        sleep 1
    done
    
    # Force kill if still running
    if ps -p "$PID" > /dev/null 2>&1; then
        print_warn "Backend did not stop gracefully. Force killing..."
        kill -9 "$PID"
        sleep 1
        if ! ps -p "$PID" > /dev/null 2>&1; then
            print_info "Backend force stopped"
            rm -f "$BACKEND_PID_FILE"
        else
            print_error "Failed to stop backend"
            exit 1
        fi
    fi
else
    print_warn "Backend process (PID: $PID) is not running"
    rm -f "$BACKEND_PID_FILE"
fi

