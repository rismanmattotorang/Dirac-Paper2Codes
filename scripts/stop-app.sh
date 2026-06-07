#!/bin/bash

# Paper2Codes Application Stop Script
# This script stops both the backend and frontend

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_header "Stopping Paper2Codes Application"

# Stop frontend
print_info "Stopping frontend..."
"$SCRIPT_DIR/stop-frontend.sh" || print_warn "Failed to stop frontend (may not be running)"

# Stop backend
print_info "Stopping backend..."
"$SCRIPT_DIR/stop-backend.sh" || print_warn "Failed to stop backend (may not be running)"

print_header "Paper2Codes Application Stopped"
