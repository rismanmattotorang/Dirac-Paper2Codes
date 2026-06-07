#!/bin/bash

# Paper2Codes Frontend Stop Script
# This script stops the Paper2Codes WebUI frontend

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FRONTEND_PID_FILE="$PROJECT_ROOT/.frontend.pid"

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
if [ ! -f "$FRONTEND_PID_FILE" ]; then
    print_warn "Frontend PID file not found. Frontend may not be running."
    exit 0
fi

PID=$(cat "$FRONTEND_PID_FILE")

# Check if process is running
if ps -p "$PID" > /dev/null 2>&1; then
    print_info "Stopping frontend (PID: $PID)..."
    kill "$PID"
    
    # Wait for process to stop
    for i in {1..10}; do
        if ! ps -p "$PID" > /dev/null 2>&1; then
            print_info "Frontend stopped successfully"
            rm -f "$FRONTEND_PID_FILE"
            exit 0
        fi
        sleep 1
    done
    
    # Force kill if still running
    if ps -p "$PID" > /dev/null 2>&1; then
        print_warn "Frontend did not stop gracefully. Force killing..."
        kill -9 "$PID"
        sleep 1
        if ! ps -p "$PID" > /dev/null 2>&1; then
            print_info "Frontend force stopped"
            rm -f "$FRONTEND_PID_FILE"
        else
            print_error "Failed to stop frontend"
            exit 1
        fi
    fi
else
    print_warn "Frontend process (PID: $PID) is not running"
    rm -f "$FRONTEND_PID_FILE"
fi

