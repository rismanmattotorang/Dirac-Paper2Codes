#!/bin/bash

# Paper2Codes Full Application Startup Script
# This script starts both the backend and frontend

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
START_BACKEND=true
START_FRONTEND=true

while [[ $# -gt 0 ]]; do
    case $1 in
        --backend-only)
            START_FRONTEND=false
            shift
            ;;
        --frontend-only)
            START_BACKEND=false
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Usage: $0 [--backend-only] [--frontend-only]"
            exit 1
            ;;
    esac
done

print_header "Starting Paper2Codes Application"

BACKEND_STARTED=false
FRONTEND_STARTED=false

# Start backend
if [ "$START_BACKEND" = true ]; then
    print_info "Starting backend..."
    if "$SCRIPT_DIR/start-backend.sh"; then
        BACKEND_STARTED=true
        print_info "Waiting for backend to initialize..."
        sleep 3
    else
        print_error "Failed to start backend"
        if [ "$START_FRONTEND" = false ]; then
            exit 1
        fi
        print_warn "Continuing with frontend only..."
    fi
fi

# Start frontend
if [ "$START_FRONTEND" = true ]; then
    print_info "Starting frontend..."
    if "$SCRIPT_DIR/start-frontend.sh"; then
        FRONTEND_STARTED=true
    else
        print_error "Failed to start frontend"
        if [ "$BACKEND_STARTED" = true ]; then
            print_warn "Backend is still running. You can stop it with: ./scripts/stop-backend.sh"
        fi
        if [ "$START_BACKEND" = false ]; then
            exit 1
        fi
    fi
fi

# Print summary
print_header "Startup Summary"

if [ "$START_BACKEND" = true ]; then
    if [ "$BACKEND_STARTED" = true ]; then
        print_info "✓ Backend: RUNNING"
        print_info "  API: http://127.0.0.1:8080"
        print_info "  Health: http://127.0.0.1:8080/api/health"
    else
        print_error "✗ Backend: FAILED"
    fi
fi

if [ "$START_FRONTEND" = true ]; then
    if [ "$FRONTEND_STARTED" = true ]; then
        FRONTEND_PORT="${FRONTEND_PORT:-3000}"
        print_info "✓ Frontend: RUNNING"
        print_info "  WebUI: http://localhost:$FRONTEND_PORT"
    else
        print_error "✗ Frontend: FAILED"
    fi
fi

echo ""
if [ "$BACKEND_STARTED" = true ] || [ "$FRONTEND_STARTED" = true ]; then
    print_info "Application started. To stop, run: ./scripts/stop.sh"
    if [ "$BACKEND_STARTED" = false ] || [ "$FRONTEND_STARTED" = false ]; then
        print_warn "Some components failed to start. Check logs in $PROJECT_ROOT/logs/"
        exit 1
    else
        exit 0
    fi
else
    print_error "Failed to start any components"
    exit 1
fi

