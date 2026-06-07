#!/bin/bash

# Script to verify all connections are working properly
# - SurrealDB connection
# - Backend API health
# - WebSocket connection
# - Environment variables

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_header "Paper2Codes Connection Verification"

# 1. Check SurrealDB
echo ""
echo "1. Checking SurrealDB..."
if ps aux | grep -i surreal | grep -v grep > /dev/null; then
    print_success "SurrealDB process is running"
    
    # Check if port 8000 is accessible
    if curl -s http://localhost:8000/health > /dev/null 2>&1; then
        print_success "SurrealDB is accessible on port 8000"
    else
        print_warn "SurrealDB is running but /health endpoint not accessible"
    fi
else
    print_error "SurrealDB is not running"
    echo "   Start it with: cd $PROJECT_ROOT/Paper2Codes-Core && ./scripts/start_surrealdb.sh"
fi

# 2. Check Backend API
echo ""
echo "2. Checking Backend API..."
if curl -s http://127.0.0.1:8080/api/health > /dev/null 2>&1; then
    print_success "Backend API is accessible"
    
    # Check health response
    HEALTH_RESPONSE=$(curl -s http://127.0.0.1:8080/api/health)
    if echo "$HEALTH_RESPONSE" | grep -q "healthy"; then
        print_success "Backend API health check passed"
    else
        print_warn "Backend API health check returned unexpected response"
    fi
else
    print_error "Backend API is not accessible at http://127.0.0.1:8080"
    echo "   Start it with: $SCRIPT_DIR/start-backend.sh"
fi

# 3. Check WebSocket endpoint
echo ""
echo "3. Checking WebSocket endpoint..."
if curl -s -I http://127.0.0.1:8080/ws 2>&1 | grep -q "101\|Upgrade"; then
    print_success "WebSocket endpoint is accessible"
else
    print_warn "WebSocket endpoint might not be working properly"
fi

# 4. Check Configuration
echo ""
echo "4. Checking Configuration..."
CONFIG_FILE="$HOME/.config/paper2codes/config.toml"
if [ -f "$CONFIG_FILE" ]; then
    print_success "Config file exists at $CONFIG_FILE"
    
    # Check storage enabled
    if grep -q "enabled = true" "$CONFIG_FILE"; then
        print_success "Storage is enabled in config"
    else
        print_warn "Storage might be disabled in config"
    fi
    
    # Check connection string
    if grep -q "connection_string = \"ws://" "$CONFIG_FILE"; then
        CONNECTION_STRING=$(grep "connection_string" "$CONFIG_FILE" | head -1 | cut -d'"' -f2)
        print_success "Connection string configured: $CONNECTION_STRING"
    else
        print_warn "Connection string not found in config"
    fi
else
    print_warn "Config file not found at $CONFIG_FILE"
    echo "   Creating from example..."
    mkdir -p "$HOME/.config/paper2codes"
    cp "$PROJECT_ROOT/Paper2Codes-Core/config.example.toml" "$CONFIG_FILE"
    print_success "Config file created. Please edit it with your settings."
fi

# 5. Check Environment Variables
echo ""
echo "5. Checking Environment Variables..."

if [ -n "$OPENROUTER_API_KEY" ]; then
    print_success "OPENROUTER_API_KEY is set"
elif [ -n "$OPENAI_API_KEY" ]; then
    print_success "OPENAI_API_KEY is set"
elif [ -n "$ANTHROPIC_API_KEY" ]; then
    print_success "ANTHROPIC_API_KEY is set"
else
    print_warn "No LLM API keys found in environment variables"
    echo "   Set one of: OPENROUTER_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY"
    echo "   Or configure in $CONFIG_FILE"
fi

if [ -n "$DATABASE_URL" ]; then
    print_success "DATABASE_URL is set: $DATABASE_URL"
fi

# 6. Test Database Connection (if backend is running)
echo ""
echo "6. Testing Database Connection..."
if curl -s http://127.0.0.1:8080/api/health > /dev/null 2>&1; then
    # Check backend logs for storage connection
    if [ -f "$PROJECT_ROOT/logs/backend.log" ]; then
        if grep -q "Successfully connected to SurrealDB" "$PROJECT_ROOT/logs/backend.log"; then
            print_success "Backend logs show successful SurrealDB connection"
        elif grep -q "Failed to connect to SurrealDB" "$PROJECT_ROOT/logs/backend.log"; then
            print_error "Backend logs show SurrealDB connection failure"
            echo "   Check logs: tail -20 $PROJECT_ROOT/logs/backend.log"
        else
            print_warn "Cannot determine database connection status from logs"
        fi
    fi
else
    print_warn "Cannot test database connection - backend not running"
fi

echo ""
print_header "Verification Complete"
echo ""
echo "Summary:"
echo "- SurrealDB: Check status above"
echo "- Backend API: Check status above"
echo "- WebSocket: Check status above"
echo "- Configuration: Check status above"
echo ""
echo "If there are issues:"
echo "1. Ensure SurrealDB is running: cd $PROJECT_ROOT/Paper2Codes-Core && ./scripts/start_surrealdb.sh"
echo "2. Restart backend: $SCRIPT_DIR/stop-backend.sh && $SCRIPT_DIR/start-backend.sh"
echo "3. Check logs: tail -f $PROJECT_ROOT/logs/backend.log"

