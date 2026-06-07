#!/bin/bash

# Paper2Codes Frontend Startup Script
# This script starts the Paper2Codes WebUI frontend

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WEBUI_DIR="$PROJECT_ROOT/Paper2Codes-WebUI"
FRONTEND_PID_FILE="$PROJECT_ROOT/.frontend.pid"
FRONTEND_LOG_FILE="$PROJECT_ROOT/logs/frontend.log"

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

# Check if frontend is already running
if [ -f "$FRONTEND_PID_FILE" ]; then
    PID=$(cat "$FRONTEND_PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        print_warn "Frontend is already running (PID: $PID)"
        exit 1
    else
        print_info "Removing stale PID file"
        rm -f "$FRONTEND_PID_FILE"
    fi
fi

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    print_error "Node.js is not installed. Please install Node.js 18+ first."
    exit 1
fi

# Check Node.js version
NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    print_error "Node.js version 18+ is required. Current version: $(node -v)"
    exit 1
fi

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    print_error "npm is not installed. Please install npm first."
    exit 1
fi

# Check if Deno is available (preferred)
USE_DENO=false
if command -v deno &> /dev/null; then
    if [ -f "$WEBUI_DIR/deno.json" ]; then
        USE_DENO=true
        print_info "Deno detected. Will use Deno tasks."
    fi
fi

# Check if node_modules exists, if not install dependencies
if [ ! -d "$WEBUI_DIR/node_modules" ]; then
    print_info "Dependencies not found. Installing..."
    cd "$WEBUI_DIR"
    
    # Try to use available package manager
    if command -v pnpm &> /dev/null; then
        pnpm install
    elif command -v yarn &> /dev/null; then
        yarn install
    elif command -v npm &> /dev/null; then
        npm install
    else
        print_error "No package manager found (npm/pnpm/yarn). Please install npm."
        exit 1
    fi
    
    if [ $? -ne 0 ]; then
        print_error "Failed to install dependencies"
        exit 1
    fi
    print_info "Dependencies installed successfully"
fi

# Set default port
PORT="${FRONTEND_PORT:-3000}"

# Check if production build exists
PRODUCTION_MODE=false
if [ -d "$WEBUI_DIR/.next" ]; then
    PRODUCTION_MODE=true
    print_info "Production build found. Starting in production mode..."
else
    print_warn "Production build not found. Starting in development mode..."
    print_warn "To build for production, run: ./scripts/build.sh"
fi

print_info "Starting Paper2Codes frontend on port $PORT"
print_info "Logs will be written to: $FRONTEND_LOG_FILE"

# Start the frontend server
cd "$WEBUI_DIR"
if [ "$PRODUCTION_MODE" = true ]; then
    if [ "$USE_DENO" = true ]; then
        deno task start > "$FRONTEND_LOG_FILE" 2>&1 &
    elif command -v pnpm &> /dev/null; then
        pnpm start > "$FRONTEND_LOG_FILE" 2>&1 &
    elif command -v yarn &> /dev/null; then
        yarn start > "$FRONTEND_LOG_FILE" 2>&1 &
    else
        npm start > "$FRONTEND_LOG_FILE" 2>&1 &
    fi
else
    if [ "$USE_DENO" = true ]; then
        deno task dev > "$FRONTEND_LOG_FILE" 2>&1 &
    elif command -v pnpm &> /dev/null; then
        pnpm dev > "$FRONTEND_LOG_FILE" 2>&1 &
    elif command -v yarn &> /dev/null; then
        yarn dev > "$FRONTEND_LOG_FILE" 2>&1 &
    else
        npm run dev > "$FRONTEND_LOG_FILE" 2>&1 &
    fi
fi
FRONTEND_PID=$!

# Save PID
echo "$FRONTEND_PID" > "$FRONTEND_PID_FILE"

# Wait a moment to check if it started successfully
sleep 3

if ps -p "$FRONTEND_PID" > /dev/null 2>&1; then
    print_info "Frontend started successfully (PID: $FRONTEND_PID)"
    print_info "WebUI available at: http://localhost:$PORT"
    print_info "To stop the frontend, run: ./scripts/stop-frontend.sh"
else
    print_error "Frontend failed to start. Check logs: $FRONTEND_LOG_FILE"
    rm -f "$FRONTEND_PID_FILE"
    exit 1
fi

