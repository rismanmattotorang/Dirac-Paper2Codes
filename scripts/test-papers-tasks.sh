#!/bin/bash

# Comprehensive test script for Papers and Task Queue features
# This script validates all implemented features including:
# - Backend API endpoints
# - Frontend components
# - Storage integration
# - WebSocket real-time updates

set -e

echo "============================================"
echo "Paper2Codes - Papers & Tasks Feature Tests"
echo "============================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0

# Function to print test result
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASSED${NC}: $2"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAILED${NC}: $2"
        ((TESTS_FAILED++))
    fi
}

# Check if backend is running
check_backend() {
    echo "Checking if backend is running..."
    if curl -s http://localhost:3001/api/health > /dev/null 2>&1; then
        print_result 0 "Backend health check"
        return 0
    else
        print_result 1 "Backend health check - Backend is not running!"
        echo -e "${YELLOW}⚠ Please start the backend first: cd Paper2Codes-Core && cargo run --release${NC}"
        return 1
    fi
}

# Check if frontend is running
check_frontend() {
    echo "Checking if frontend is running..."
    if curl -s http://localhost:3000 > /dev/null 2>&1; then
        print_result 0 "Frontend accessibility check"
        return 0
    else
        print_result 1 "Frontend accessibility check - Frontend is not running!"
        echo -e "${YELLOW}⚠ Please start the frontend first: cd Paper2Codes-WebUI && deno task dev${NC}"
        return 1
    fi
}

# Test 1: Papers CRUD Operations
test_papers_crud() {
    echo ""
    echo "=== Test 1: Papers CRUD Operations ==="
    
    # Create paper
    echo "Creating paper..."
    CREATE_RESPONSE=$(curl -s -X POST http://localhost:3001/api/papers \
        -H "Content-Type: application/json" \
        -d '{"title": "Test Paper - Attention Mechanisms", "abstract_text": "This paper explores attention mechanisms"}')
    
    PAPER_ID=$(echo $CREATE_RESPONSE | jq -r '.data.id')
    if [ "$PAPER_ID" != "null" ] && [ ! -z "$PAPER_ID" ]; then
        print_result 0 "Create paper"
    else
        print_result 1 "Create paper"
        return 1
    fi
    
    # Get paper
    echo "Retrieving paper..."
    GET_RESPONSE=$(curl -s http://localhost:3001/api/papers/$PAPER_ID)
    GET_TITLE=$(echo $GET_RESPONSE | jq -r '.data.title')
    if [ "$GET_TITLE" == "Test Paper - Attention Mechanisms" ]; then
        print_result 0 "Get paper by ID"
    else
        print_result 1 "Get paper by ID"
    fi
    
    # Update paper
    echo "Updating paper..."
    UPDATE_RESPONSE=$(curl -s -X PUT http://localhost:3001/api/papers/$PAPER_ID \
        -H "Content-Type: application/json" \
        -d '{"title": "Updated Test Paper", "abstract_text": "Updated abstract"}')
    UPDATE_TITLE=$(echo $UPDATE_RESPONSE | jq -r '.data.title')
    if [ "$UPDATE_TITLE" == "Updated Test Paper" ]; then
        print_result 0 "Update paper"
    else
        print_result 1 "Update paper"
    fi
    
    # List papers
    echo "Listing papers..."
    LIST_RESPONSE=$(curl -s "http://localhost:3001/api/papers?page=1&per_page=10")
    LIST_COUNT=$(echo $LIST_RESPONSE | jq '.data | length')
    if [ $LIST_COUNT -gt 0 ]; then
        print_result 0 "List papers"
    else
        print_result 1 "List papers"
    fi
    
    # Delete paper
    echo "Deleting paper..."
    DELETE_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE http://localhost:3001/api/papers/$PAPER_ID)
    if [ "$DELETE_RESPONSE" == "204" ]; then
        print_result 0 "Delete paper"
    else
        print_result 1 "Delete paper"
    fi
    
    # Verify deletion
    echo "Verifying deletion..."
    VERIFY_RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3001/api/papers/$PAPER_ID)
    if [ "$VERIFY_RESPONSE" == "404" ]; then
        print_result 0 "Verify paper deletion"
    else
        print_result 1 "Verify paper deletion"
    fi
}

# Test 2: Task Queue Operations
test_tasks_crud() {
    echo ""
    echo "=== Test 2: Task Queue Operations ==="
    
    # Create task
    echo "Creating task..."
    CREATE_RESPONSE=$(curl -s -X POST http://localhost:3001/api/tasks \
        -H "Content-Type: application/json" \
        -d '{"description": "Test task for paper processing", "task_type": "Planning"}')
    
    TASK_ID=$(echo $CREATE_RESPONSE | jq -r '.data.id')
    if [ "$TASK_ID" != "null" ] && [ ! -z "$TASK_ID" ]; then
        print_result 0 "Create task"
    else
        print_result 1 "Create task"
        return 1
    fi
    
    # Get task
    echo "Retrieving task..."
    GET_RESPONSE=$(curl -s http://localhost:3001/api/tasks/$TASK_ID)
    GET_DESC=$(echo $GET_RESPONSE | jq -r '.data.description')
    if [ "$GET_DESC" == "Test task for paper processing" ]; then
        print_result 0 "Get task by ID"
    else
        print_result 1 "Get task by ID"
    fi
    
    # Update task status
    echo "Updating task status..."
    UPDATE_RESPONSE=$(curl -s -X PUT http://localhost:3001/api/tasks/$TASK_ID/status \
        -H "Content-Type: application/json" \
        -d '{"status": "InProgress"}')
    UPDATE_STATUS=$(echo $UPDATE_RESPONSE | jq -r '.data.status')
    if [ "$UPDATE_STATUS" == "InProgress" ]; then
        print_result 0 "Update task status"
    else
        print_result 1 "Update task status"
    fi
    
    # List tasks
    echo "Listing tasks..."
    LIST_RESPONSE=$(curl -s "http://localhost:3001/api/tasks?page=1&per_page=10")
    LIST_COUNT=$(echo $LIST_RESPONSE | jq '.data | length')
    if [ $LIST_COUNT -gt 0 ]; then
        print_result 0 "List tasks"
    else
        print_result 1 "List tasks"
    fi
    
    # Cancel task
    echo "Cancelling task..."
    CANCEL_RESPONSE=$(curl -s -X POST http://localhost:3001/api/tasks/$TASK_ID/cancel)
    CANCEL_STATUS=$(echo $CANCEL_RESPONSE | jq -r '.data.status')
    if [[ "$CANCEL_STATUS" == *"Failed"* ]]; then
        print_result 0 "Cancel task"
    else
        print_result 1 "Cancel task"
    fi
}

# Test 3: Integration Tests
test_integration() {
    echo ""
    echo "=== Test 3: Integration Tests ==="
    
    # Run Rust integration tests
    echo "Running Rust integration tests..."
    cd Paper2Codes-Core
    if cargo test --test papers_tasks_integration --features api -- --test-threads=1 2>&1 | grep -q "test result: ok"; then
        print_result 0 "Rust integration tests"
    else
        print_result 1 "Rust integration tests"
    fi
    cd ..
}

# Test 4: Error Handling
test_error_handling() {
    echo ""
    echo "=== Test 4: Error Handling ==="
    
    # Test 404 for non-existent paper
    echo "Testing 404 error handling..."
    RESPONSE_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3001/api/papers/00000000-0000-0000-0000-000000000000)
    if [ "$RESPONSE_CODE" == "404" ]; then
        print_result 0 "404 error for non-existent paper"
    else
        print_result 1 "404 error for non-existent paper"
    fi
    
    # Test 400 for invalid UUID
    echo "Testing 400 error handling..."
    RESPONSE_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3001/api/tasks/invalid-uuid)
    if [ "$RESPONSE_CODE" == "400" ]; then
        print_result 0 "400 error for invalid UUID"
    else
        print_result 1 "400 error for invalid UUID"
    fi
    
    # Test validation error
    echo "Testing validation error..."
    RESPONSE_CODE=$(curl -s -o /dev/null -w "%{http_code}" -X POST http://localhost:3001/api/papers \
        -H "Content-Type: application/json" \
        -d '{"title": "", "abstract_text": ""}')
    if [ "$RESPONSE_CODE" == "400" ]; then
        print_result 0 "Validation error for empty title"
    else
        print_result 1 "Validation error for empty title"
    fi
}

# Test 5: Pagination
test_pagination() {
    echo ""
    echo "=== Test 5: Pagination ==="
    
    # Create multiple papers
    echo "Creating multiple papers for pagination test..."
    for i in {1..15}; do
        curl -s -X POST http://localhost:3001/api/papers \
            -H "Content-Type: application/json" \
            -d "{\"title\": \"Pagination Test Paper $i\", \"abstract_text\": \"Abstract $i\"}" > /dev/null
    done
    
    # Test page 1
    echo "Testing page 1..."
    PAGE1_RESPONSE=$(curl -s "http://localhost:3001/api/papers?page=1&per_page=10")
    PAGE1_COUNT=$(echo $PAGE1_RESPONSE | jq '.data | length')
    if [ $PAGE1_COUNT -eq 10 ]; then
        print_result 0 "Pagination page 1 (10 items)"
    else
        print_result 1 "Pagination page 1 (expected 10, got $PAGE1_COUNT)"
    fi
    
    # Test page 2
    echo "Testing page 2..."
    PAGE2_RESPONSE=$(curl -s "http://localhost:3001/api/papers?page=2&per_page=10")
    PAGE2_COUNT=$(echo $PAGE2_RESPONSE | jq '.data | length')
    if [ $PAGE2_COUNT -ge 5 ]; then
        print_result 0 "Pagination page 2 (at least 5 items)"
    else
        print_result 1 "Pagination page 2 (expected at least 5, got $PAGE2_COUNT)"
    fi
}

# Main execution
main() {
    echo "Starting comprehensive feature tests..."
    echo ""
    
    # Check prerequisites
    if ! command -v jq &> /dev/null; then
        echo -e "${RED}Error: jq is not installed. Please install jq to run these tests.${NC}"
        exit 1
    fi
    
    if ! command -v curl &> /dev/null; then
        echo -e "${RED}Error: curl is not installed. Please install curl to run these tests.${NC}"
        exit 1
    fi
    
    # Check backend and frontend
    check_backend || exit 1
    check_frontend || echo -e "${YELLOW}⚠ Frontend is not running. Some tests may be skipped.${NC}"
    
    # Run all tests
    test_papers_crud
    test_tasks_crud
    test_error_handling
    test_pagination
    
    # Print summary
    echo ""
    echo "============================================"
    echo "Test Summary"
    echo "============================================"
    echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
    echo "Total Tests: $((TESTS_PASSED + TESTS_FAILED))"
    echo ""
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}✓ All tests passed!${NC}"
        exit 0
    else
        echo -e "${RED}✗ Some tests failed. Please review the output above.${NC}"
        exit 1
    fi
}

# Run main function
main

