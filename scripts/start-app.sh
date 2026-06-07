#!/bin/bash

# Paper2Codes Application Startup Script
# This script builds (if needed) and starts both the backend and frontend

set -e

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
BUILD_FIRST=false
SKIP_BUILD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --build)
            BUILD_FIRST=true
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Usage: $0 [--build] [--skip-build]"
            exit 1
            ;;
    esac
done

print_header "Starting Paper2Codes Application"

CORE_DIR="$PROJECT_ROOT/Paper2Codes-Core"
WEBUI_DIR="$PROJECT_ROOT/Paper2Codes-WebUI"
BACKEND_BINARY="$CORE_DIR/target/release/paper2codes"

# Check if we need to build
if [ "$SKIP_BUILD" = false ]; then
    NEED_BUILD=false
    
    # Check if backend binary exists
    if [ ! -f "$BACKEND_BINARY" ]; then
        print_warn "Backend binary not found"
        NEED_BUILD=true
    fi
    
    # Check if frontend is built (check for .next directory)
    if [ ! -d "$WEBUI_DIR/.next" ]; then
        print_warn "Frontend build not found"
        NEED_BUILD=true
    fi
    
    # Build if needed or if --build flag is set
    if [ "$BUILD_FIRST" = true ] || [ "$NEED_BUILD" = true ]; then
        print_info "Building application..."
        if [ "$NEED_BUILD" = true ] && [ "$BUILD_FIRST" = false ]; then
            "$SCRIPT_DIR/build.sh" --skip-deps
        else
            "$SCRIPT_DIR/build.sh"
        fi
        if [ $? -ne 0 ]; then
            print_error "Build failed"
            exit 1
        fi
    fi
fi

# Start backend
print_info "Starting backend..."
"$SCRIPT_DIR/start-backend.sh"
if [ $? -ne 0 ]; then
    print_error "Failed to start backend"
    exit 1
fi

# Wait a bit for backend to fully start
sleep 2

# Start frontend
print_info "Starting frontend..."
"$SCRIPT_DIR/start-frontend.sh"
if [ $? -ne 0 ]; then
    print_error "Failed to start frontend"
    print_warn "Stopping backend..."
    "$SCRIPT_DIR/stop-backend.sh"
    exit 1
fi

print_header "Paper2Codes Application Started Successfully"
print_info "Backend API: http://127.0.0.1:8080"
print_info "Frontend WebUI: http://localhost:3000"
print_info ""
print_info "To stop the application, run: ./scripts/stop-app.sh"
