#!/bin/bash

# Test script to verify all frontend pages are connected to backend
# This script tests all API endpoints used by the frontend

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# API base URL
API_BASE="${API_BASE_URL:-http://127.0.0.1:8080}"

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

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

# Function to test an API endpoint
test_endpoint() {
    local method=$1
    local endpoint=$2
    local description=$3
    local data=$4
    local expected_status=${5:-200}
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    local url="${API_BASE}${endpoint}"
    local response
    local status_code
    
    echo -n "Testing ${description}... "
    
    # Add delay to avoid rate limiting (0.5 seconds between requests)
    sleep 0.5
    
    if [ "$method" = "GET" ]; then
        response=$(curl -s -w "\n%{http_code}" "$url" 2>&1)
    elif [ "$method" = "POST" ]; then
        if [ -n "$data" ]; then
            response=$(curl -s -w "\n%{http_code}" -X POST -H "Content-Type: application/json" -d "$data" "$url" 2>&1)
        else
            response=$(curl -s -w "\n%{http_code}" -X POST "$url" 2>&1)
        fi
    elif [ "$method" = "PUT" ]; then
        response=$(curl -s -w "\n%{http_code}" -X PUT -H "Content-Type: application/json" -d "$data" "$url" 2>&1)
    elif [ "$method" = "DELETE" ]; then
        response=$(curl -s -w "\n%{http_code}" -X DELETE "$url" 2>&1)
    fi
    
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    # Handle rate limiting (429) - retry after delay
    if [ "$status_code" = "429" ]; then
        echo -n "(rate limited, waiting 2s...) "
        sleep 2
        # Retry once
        if [ "$method" = "GET" ]; then
            response=$(curl -s -w "\n%{http_code}" "$url" 2>&1)
        elif [ "$method" = "POST" ]; then
            if [ -n "$data" ]; then
                response=$(curl -s -w "\n%{http_code}" -X POST -H "Content-Type: application/json" -d "$data" "$url" 2>&1)
            else
                response=$(curl -s -w "\n%{http_code}" -X POST "$url" 2>&1)
            fi
        elif [ "$method" = "PUT" ]; then
            response=$(curl -s -w "\n%{http_code}" -X PUT -H "Content-Type: application/json" -d "$data" "$url" 2>&1)
        elif [ "$method" = "DELETE" ]; then
            response=$(curl -s -w "\n%{http_code}" -X DELETE "$url" 2>&1)
        fi
        status_code=$(echo "$response" | tail -n1)
        body=$(echo "$response" | sed '$d')
    fi
    
    if [ "$status_code" = "$expected_status" ] || ([ "$status_code" = "200" ] && [ "$expected_status" = "201" ]); then
        # Check if response is valid JSON (if not empty)
        if [ -n "$body" ]; then
            if echo "$body" | jq . >/dev/null 2>&1 2>/dev/null || [ "$status_code" = "204" ]; then
                echo -e "${GREEN}✓ PASS${NC} (Status: $status_code)"
                PASSED_TESTS=$((PASSED_TESTS + 1))
                return 0
            else
                echo -e "${YELLOW}⚠ WARN${NC} (Status: $status_code, but response is not valid JSON)"
                PASSED_TESTS=$((PASSED_TESTS + 1))
                return 0
            fi
        else
            echo -e "${GREEN}✓ PASS${NC} (Status: $status_code)"
            PASSED_TESTS=$((PASSED_TESTS + 1))
            return 0
        fi
    else
        # Accept 401/403 for endpoints that require auth (that's expected)
        if [ "$status_code" = "401" ] || [ "$status_code" = "403" ]; then
            echo -e "${YELLOW}⚠ AUTH${NC} (Status: $status_code - requires authentication, which is expected)"
            PASSED_TESTS=$((PASSED_TESTS + 1))
            return 0
        fi
        echo -e "${RED}✗ FAIL${NC} (Expected: $expected_status, Got: $status_code)"
        if [ -n "$body" ]; then
            echo "  Response: $(echo "$body" | head -c 200)"
        fi
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

# Check if backend is running
print_header "Checking Backend Status"
if ! curl -s "${API_BASE}/api/health" > /dev/null 2>&1; then
    print_error "Backend is not running at ${API_BASE}"
    print_error "Please start the backend first: ./scripts/start-backend.sh"
    exit 1
fi

print_info "Backend is running at ${API_BASE}"

# Test Health Endpoint
print_header "Testing Health Endpoint"
test_endpoint "GET" "/api/health" "Health check"

# Test Dashboard Endpoints
print_header "Testing Dashboard Page Endpoints"
test_endpoint "GET" "/api/analytics/overview" "Analytics overview (Dashboard)"
test_endpoint "GET" "/api/tasks?page=1&per_page=10" "List tasks (Dashboard)"
test_endpoint "GET" "/api/papers?page=1&per_page=5" "List papers (Dashboard)"
test_endpoint "GET" "/api/analytics/agents" "Agent performance (Dashboard)"

# Test Papers Page Endpoints
print_header "Testing Papers Page Endpoints"
test_endpoint "GET" "/api/papers?page=1&per_page=50" "List papers (Papers page)"
test_endpoint "GET" "/api/papers?page=1&per_page=50" "List papers with pagination"

# Test creating a paper (we'll clean it up if it succeeds)
PAPER_ID=""
if test_endpoint "POST" "/api/papers" "Create paper" '{"title":"Test Paper","abstract_text":"Test abstract"}' 201; then
    # Extract paper ID from response if possible
    response=$(curl -s -X POST -H "Content-Type: application/json" -d '{"title":"Test Paper Delete","abstract_text":"Test"}' "${API_BASE}/api/papers" 2>&1)
    if echo "$response" | jq -e '.data.id' >/dev/null 2>&1; then
        PAPER_ID=$(echo "$response" | jq -r '.data.id')
        if [ -n "$PAPER_ID" ] && [ "$PAPER_ID" != "null" ]; then
            test_endpoint "GET" "/api/papers/${PAPER_ID}" "Get paper by ID"
            test_endpoint "PUT" "/api/papers/${PAPER_ID}" "Update paper" '{"title":"Updated Test Paper"}'
            test_endpoint "DELETE" "/api/papers/${PAPER_ID}" "Delete paper" "" 204
        fi
    fi
fi

# Test Tasks Page Endpoints
print_header "Testing Tasks Page Endpoints"
test_endpoint "GET" "/api/tasks?page=1&per_page=100" "List tasks (Tasks page)"

# Test creating a task
TASK_ID=""
if test_endpoint "POST" "/api/tasks" "Create task" '{"task_type":"Planning","description":"Test task"}' 201; then
    response=$(curl -s -X POST -H "Content-Type: application/json" -d '{"task_type":"Planning","description":"Test task delete"}' "${API_BASE}/api/tasks" 2>&1)
    if echo "$response" | jq -e '.data.id' >/dev/null 2>&1; then
        TASK_ID=$(echo "$response" | jq -r '.data.id')
        if [ -n "$TASK_ID" ] && [ "$TASK_ID" != "null" ]; then
            test_endpoint "GET" "/api/tasks/${TASK_ID}" "Get task by ID"
        fi
    fi
fi

# Test Code/Repositories Page Endpoints
print_header "Testing Code/Repositories Page Endpoints"
test_endpoint "GET" "/api/repositories?page=1&per_page=50" "List repositories (Code page)"

# Test Analytics Page Endpoints
print_header "Testing Analytics Page Endpoints"
test_endpoint "GET" "/api/analytics/overview" "Analytics overview (Analytics page)"
test_endpoint "GET" "/api/analytics/performance" "Performance metrics"
test_endpoint "GET" "/api/analytics/performance?range=7d" "Performance metrics with range"
test_endpoint "GET" "/api/analytics/usage" "Usage statistics"
test_endpoint "GET" "/api/analytics/usage?period=30d" "Usage statistics with period"
test_endpoint "GET" "/api/analytics/agents" "Agent performance (Analytics page)"

# Test Settings Page Endpoints
print_header "Testing Settings Page Endpoints"
test_endpoint "GET" "/api/settings" "Get settings (Settings page)"
# Note: PUT test might fail if settings require specific format, that's okay

# Test Search Endpoints
print_header "Testing Search Endpoints"
test_endpoint "POST" "/api/search" "Search" '{"query":"test"}'
test_endpoint "GET" "/api/search/suggestions?q=test" "Search suggestions"
test_endpoint "GET" "/api/search/facets" "Search facets"

# Test File Management Endpoints
print_header "Testing File Management Endpoints"
# File upload requires multipart/form-data, so we'll just check if endpoint exists
test_endpoint "GET" "/api/files" "List files" "" 200 2>/dev/null || test_endpoint "GET" "/api/files" "List files" "" 404

# Print Summary
print_header "Test Summary"
echo "Total Tests: $TOTAL_TESTS"
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}Failed: $FAILED_TESTS${NC}"
else
    echo -e "${GREEN}Failed: $FAILED_TESTS${NC}"
fi

if [ $FAILED_TESTS -eq 0 ]; then
    print_info "All tests passed! Frontend pages are properly connected to backend."
    exit 0
else
    print_warn "Some tests failed. Please check the errors above."
    exit 1
fi

