#!/bin/bash
# Comprehensive Dashboard API Testing Script
# Tests all backend endpoints used by the Dashboard

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# API Configuration
API_BASE="${API_BASE_URL:-http://localhost:8080}"
AUTH_TOKEN="${AUTH_TOKEN:-}"

# Test results tracking
PASSED=0
FAILED=0
TOTAL=0

# Helper functions
print_header() {
    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

print_test() {
    echo -e "${YELLOW}Testing:${NC} $1"
}

print_pass() {
    echo -e "${GREEN}✓ PASS:${NC} $1"
    ((PASSED++))
    ((TOTAL++))
}

print_fail() {
    echo -e "${RED}✗ FAIL:${NC} $1"
    echo -e "${RED}  Error:${NC} $2"
    ((FAILED++))
    ((TOTAL++))
}

# Make API request
api_request() {
    local method="$1"
    local endpoint="$2"
    local data="${3:-}"
    
    local headers=(-H "Content-Type: application/json")
    if [ -n "$AUTH_TOKEN" ]; then
        headers+=(-H "Authorization: Bearer $AUTH_TOKEN")
    fi
    
    if [ -n "$data" ]; then
        curl -s -X "$method" "${API_BASE}${endpoint}" \
            "${headers[@]}" \
            -d "$data" \
            -w "\n%{http_code}"
    else
        curl -s -X "$method" "${API_BASE}${endpoint}" \
            "${headers[@]}" \
            -w "\n%{http_code}"
    fi
}

# Test health endpoint
test_health() {
    print_test "Health endpoint"
    
    response=$(api_request GET "/api/health")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ]; then
        if echo "$body" | grep -q "healthy"; then
            print_pass "Health check returned healthy status"
        else
            print_fail "Health check" "Response doesn't contain 'healthy'"
        fi
    else
        print_fail "Health check" "HTTP status $status_code"
    fi
}

# Test analytics overview
test_analytics_overview() {
    print_test "Analytics overview endpoint"
    
    response=$(api_request GET "/api/analytics/overview")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ] || [ "$status_code" = "401" ]; then
        if echo "$body" | grep -q "total_papers\|Unauthorized"; then
            print_pass "Analytics overview endpoint is accessible"
            
            # Validate response structure
            if echo "$body" | grep -q "total_papers" && \
               echo "$body" | grep -q "total_tasks" && \
               echo "$body" | grep -q "success_rate"; then
                print_pass "Analytics overview has correct structure"
            else
                print_fail "Analytics overview" "Missing required fields"
            fi
        else
            print_fail "Analytics overview" "Invalid response structure"
        fi
    else
        print_fail "Analytics overview" "HTTP status $status_code"
    fi
}

# Test performance metrics
test_performance_metrics() {
    print_test "Performance metrics endpoint"
    
    response=$(api_request GET "/api/analytics/performance?range=24h")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ] || [ "$status_code" = "401" ]; then
        if echo "$body" | grep -q "api_response_times\|Unauthorized"; then
            print_pass "Performance metrics endpoint is accessible"
            
            # Validate response structure
            if echo "$body" | grep -q "api_response_times" && \
               echo "$body" | grep -q "database_query_times" && \
               echo "$body" | grep -q "llm_request_times"; then
                print_pass "Performance metrics has correct structure"
            else
                print_fail "Performance metrics" "Missing required fields"
            fi
        else
            print_fail "Performance metrics" "Invalid response structure"
        fi
    else
        print_fail "Performance metrics" "HTTP status $status_code"
    fi
}

# Test usage statistics
test_usage_statistics() {
    print_test "Usage statistics endpoint"
    
    response=$(api_request GET "/api/analytics/usage?period=week")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ] || [ "$status_code" = "401" ]; then
        if echo "$body" | grep -q "total_requests\|Unauthorized"; then
            print_pass "Usage statistics endpoint is accessible"
            
            # Validate response structure
            if echo "$body" | grep -q "total_requests" && \
               echo "$body" | grep -q "llm_tokens_used"; then
                print_pass "Usage statistics has correct structure"
            else
                print_fail "Usage statistics" "Missing required fields"
            fi
        else
            print_fail "Usage statistics" "Invalid response structure"
        fi
    else
        print_fail "Usage statistics" "HTTP status $status_code"
    fi
}

# Test agent performance
test_agent_performance() {
    print_test "Agent performance endpoint"
    
    response=$(api_request GET "/api/analytics/agents?range=24h")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ] || [ "$status_code" = "401" ]; then
        if echo "$body" | grep -q "planning_agent\|Unauthorized"; then
            print_pass "Agent performance endpoint is accessible"
            
            # Validate response structure
            if echo "$body" | grep -q "planning_agent" && \
               echo "$body" | grep -q "analysis_agent" && \
               echo "$body" | grep -q "coding_agent" && \
               echo "$body" | grep -q "verification_agent"; then
                print_pass "Agent performance has correct structure"
            else
                print_fail "Agent performance" "Missing required agent fields"
            fi
        else
            print_fail "Agent performance" "Invalid response structure"
        fi
    else
        print_fail "Agent performance" "HTTP status $status_code"
    fi
}

# Test tasks listing
test_tasks_list() {
    print_test "Tasks listing endpoint"
    
    response=$(api_request GET "/api/tasks?page=1&per_page=10")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ] || [ "$status_code" = "401" ]; then
        if echo "$body" | grep -q "data\|Unauthorized"; then
            print_pass "Tasks listing endpoint is accessible"
            
            # Validate pagination structure
            if echo "$body" | grep -q "page" && \
               echo "$body" | grep -q "total"; then
                print_pass "Tasks listing has correct pagination structure"
            else
                print_fail "Tasks listing" "Missing pagination fields"
            fi
        else
            print_fail "Tasks listing" "Invalid response structure"
        fi
    else
        print_fail "Tasks listing" "HTTP status $status_code"
    fi
}

# Test papers listing
test_papers_list() {
    print_test "Papers listing endpoint"
    
    response=$(api_request GET "/api/papers?page=1&per_page=10")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ] || [ "$status_code" = "401" ]; then
        if echo "$body" | grep -q "data\|Unauthorized"; then
            print_pass "Papers listing endpoint is accessible"
            
            # Validate pagination structure
            if echo "$body" | grep -q "page" && \
               echo "$body" | grep -q "total"; then
                print_pass "Papers listing has correct pagination structure"
            else
                print_fail "Papers listing" "Missing pagination fields"
            fi
        else
            print_fail "Papers listing" "Invalid response structure"
        fi
    else
        print_fail "Papers listing" "HTTP status $status_code"
    fi
}

# Test repositories listing
test_repositories_list() {
    print_test "Repositories listing endpoint"
    
    response=$(api_request GET "/api/repositories?page=1&per_page=10")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ] || [ "$status_code" = "401" ]; then
        if echo "$body" | grep -q "data\|Unauthorized"; then
            print_pass "Repositories listing endpoint is accessible"
            
            # Validate pagination structure
            if echo "$body" | grep -q "page" && \
               echo "$body" | grep -q "total"; then
                print_pass "Repositories listing has correct pagination structure"
            else
                print_fail "Repositories listing" "Missing pagination fields"
            fi
        else
            print_fail "Repositories listing" "Invalid response structure"
        fi
    else
        print_fail "Repositories listing" "HTTP status $status_code"
    fi
}

# Test metrics endpoint (Prometheus format)
test_prometheus_metrics() {
    print_test "Prometheus metrics endpoint"
    
    response=$(api_request GET "/metrics")
    status_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "200" ]; then
        if echo "$body" | grep -q "# HELP\|# TYPE"; then
            print_pass "Prometheus metrics endpoint is accessible"
        else
            print_fail "Prometheus metrics" "Response not in Prometheus format"
        fi
    else
        print_fail "Prometheus metrics" "HTTP status $status_code"
    fi
}

# Main execution
main() {
    print_header "Paper2Codes Dashboard API Testing Suite"
    
    echo "Configuration:"
    echo "  API Base URL: $API_BASE"
    echo "  Auth Token: ${AUTH_TOKEN:+[SET]}${AUTH_TOKEN:-[NOT SET]}"
    echo ""
    
    # Run all tests
    print_header "Health & Core Endpoints"
    test_health
    
    print_header "Analytics Endpoints"
    test_analytics_overview
    test_performance_metrics
    test_usage_statistics
    test_agent_performance
    
    print_header "Data Endpoints"
    test_tasks_list
    test_papers_list
    test_repositories_list
    
    print_header "Monitoring Endpoints"
    test_prometheus_metrics
    
    # Print summary
    print_header "Test Summary"
    echo -e "Total Tests: ${TOTAL}"
    echo -e "${GREEN}Passed: ${PASSED}${NC}"
    echo -e "${RED}Failed: ${FAILED}${NC}"
    echo ""
    
    if [ $FAILED -eq 0 ]; then
        echo -e "${GREEN}✓ All tests passed!${NC}\n"
        exit 0
    else
        echo -e "${RED}✗ Some tests failed${NC}\n"
        exit 1
    fi
}

# Run main function
main

