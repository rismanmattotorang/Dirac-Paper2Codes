#!/bin/bash

# Paper2Codes Full Application Stop Script
# This script stops both the backend and frontend

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

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

# Parse arguments
STOP_BACKEND=true
STOP_FRONTEND=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --backend-only)
            STOP_FRONTEND=false
            shift
            ;;
        --frontend-only)
            STOP_BACKEND=false
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Usage: $0 [--backend-only] [--frontend-only]"
            exit 1
            ;;
    esac
done

print_header "Stopping Paper2Codes Application"

BACKEND_STOPPED=false
FRONTEND_STOPPED=false

# Stop frontend
if [ "$STOP_FRONTEND" = true ]; then
    print_info "Stopping frontend..."
    if "$SCRIPT_DIR/stop-frontend.sh"; then
        FRONTEND_STOPPED=true
    else
        print_warn "Frontend stop script returned error (may not have been running)"
        FRONTEND_STOPPED=true  # Consider it stopped anyway
    fi
fi

# Stop backend
if [ "$STOP_BACKEND" = true ]; then
    print_info "Stopping backend..."
    if "$SCRIPT_DIR/stop-backend.sh"; then
        BACKEND_STOPPED=true
    else
        print_warn "Backend stop script returned error (may not have been running)"
        BACKEND_STOPPED=true  # Consider it stopped anyway
    fi
fi

# Print summary
print_header "Stop Summary"

if [ "$STOP_BACKEND" = true ]; then
    if [ "$BACKEND_STOPPED" = true ]; then
        print_info "✓ Backend: STOPPED"
    else
        print_error "✗ Backend: FAILED TO STOP"
    fi
fi

if [ "$STOP_FRONTEND" = true ]; then
    if [ "$FRONTEND_STOPPED" = true ]; then
        print_info "✓ Frontend: STOPPED"
    else
        print_error "✗ Frontend: FAILED TO STOP"
    fi
fi

echo ""
print_info "Application stopped successfully"

