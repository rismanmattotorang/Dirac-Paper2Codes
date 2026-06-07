#!/bin/bash
# Test script for Settings feature

set -e

echo "=========================================="
echo "Settings Feature Test Script"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Base URL for API
API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"

# Test token (replace with actual token in production)
AUTH_TOKEN="${AUTH_TOKEN:-test-token}"

echo "Testing against: $API_BASE_URL"
echo ""

# Function to make API calls
test_endpoint() {
    local method=$1
    local endpoint=$2
    local data=$3
    local description=$4
    
    echo "Testing: $description"
    echo "  → $method $endpoint"
    
    if [ -z "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" \
            -X "$method" \
            -H "Authorization: Bearer $AUTH_TOKEN" \
            -H "Content-Type: application/json" \
            "$API_BASE_URL$endpoint")
    else
        response=$(curl -s -w "\n%{http_code}" \
            -X "$method" \
            -H "Authorization: Bearer $AUTH_TOKEN" \
            -H "Content-Type: application/json" \
            -d "$data" \
            "$API_BASE_URL$endpoint")
    fi
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" -ge 200 ] && [ "$http_code" -lt 300 ]; then
        echo -e "  ${GREEN}✓ Success (HTTP $http_code)${NC}"
        echo "  Response: $body" | head -c 200
        echo ""
    else
        echo -e "  ${RED}✗ Failed (HTTP $http_code)${NC}"
        echo "  Response: $body"
        echo ""
    fi
    
    echo ""
}

# Check if server is running
echo "Checking if API server is running..."
if ! curl -s "$API_BASE_URL/api/health" > /dev/null 2>&1; then
    echo -e "${RED}✗ API server is not running at $API_BASE_URL${NC}"
    echo "Please start the server first:"
    echo "  cd Paper2Codes-Core && cargo run --features api"
    exit 1
fi
echo -e "${GREEN}✓ API server is running${NC}"
echo ""

# Test 1: GET Settings
echo "=========================================="
echo "Test 1: GET Settings"
echo "=========================================="
test_endpoint "GET" "/api/settings" "" "Retrieve all settings"

# Test 2: Update General Settings
echo "=========================================="
echo "Test 2: Update General Settings"
echo "=========================================="
test_endpoint "PUT" "/api/settings" \
    '{
        "general": {
            "organization_name": "Test Organization",
            "default_domain": "Deep Learning",
            "dark_mode": true,
            "theme": "dark"
        }
    }' \
    "Update general settings"

# Test 3: Update Notification Settings
echo "=========================================="
echo "Test 3: Update Notification Settings"
echo "=========================================="
test_endpoint "PUT" "/api/settings" \
    '{
        "notifications": {
            "paper_processing_complete": true,
            "code_generation_errors": false,
            "task_queue_updates": true,
            "weekly_report": false
        }
    }' \
    "Update notification settings"

# Test 4: Update LLM Settings (Valid)
echo "=========================================="
echo "Test 4: Update LLM Settings (Valid)"
echo "=========================================="
test_endpoint "PUT" "/api/settings" \
    '{
        "llm": {
            "primary_provider": "anthropic",
            "temperature": 0.8,
            "model_preferences": {
                "planning": "anthropic/claude-3-opus",
                "analysis": "anthropic/claude-3-opus",
                "coding": "openai/gpt-4-turbo",
                "verification": "anthropic/claude-3-opus"
            }
        }
    }' \
    "Update LLM settings with valid values"

# Test 5: Update LLM Settings (Invalid Temperature)
echo "=========================================="
echo "Test 5: Update LLM Settings (Invalid)"
echo "=========================================="
echo "Testing: Invalid temperature (should fail)"
test_endpoint "PUT" "/api/settings" \
    '{
        "llm": {
            "temperature": 3.0
        }
    }' \
    "Update LLM settings with invalid temperature (expected to fail)"

# Test 6: Enable 2FA
echo "=========================================="
echo "Test 6: Enable 2FA"
echo "=========================================="
test_endpoint "POST" "/api/settings/security/2fa/enable" "" "Enable two-factor authentication"

# Test 7: Disable 2FA
echo "=========================================="
echo "Test 7: Disable 2FA"
echo "=========================================="
test_endpoint "POST" "/api/settings/security/2fa/disable" "" "Disable two-factor authentication"

# Test 8: Test Database Connection
echo "=========================================="
echo "Test 8: Test Database Connection"
echo "=========================================="
test_endpoint "POST" "/api/settings/database/test" "" "Test database connection"

# Test 9: Add Team Member
echo "=========================================="
echo "Test 9: Add Team Member"
echo "=========================================="
test_endpoint "POST" "/api/settings/team/members" \
    '{
        "name": "John Doe",
        "email": "john@example.com",
        "role": "Editor"
    }' \
    "Add new team member"

# Test 10: Update Team Member Role
echo "=========================================="
echo "Test 10: Update Team Member Role"
echo "=========================================="
test_endpoint "PUT" "/api/settings/team/members/user_123/role" \
    '{
        "role": "Admin"
    }' \
    "Update team member role"

# Test 11: Revoke Session
echo "=========================================="
echo "Test 11: Revoke Session"
echo "=========================================="
test_endpoint "DELETE" "/api/settings/security/sessions/session_123" "" "Revoke user session"

# Test 12: Remove Team Member
echo "=========================================="
echo "Test 12: Remove Team Member"
echo "=========================================="
test_endpoint "DELETE" "/api/settings/team/members/user_456" "" "Remove team member"

# Test 13: Unauthorized Access (No Token)
echo "=========================================="
echo "Test 13: Unauthorized Access"
echo "=========================================="
echo "Testing: Access without authentication token"
response=$(curl -s -w "\n%{http_code}" \
    -X "GET" \
    -H "Content-Type: application/json" \
    "$API_BASE_URL/api/settings")
http_code=$(echo "$response" | tail -n1)
if [ "$http_code" -eq 401 ] || [ "$http_code" -eq 403 ]; then
    echo -e "  ${GREEN}✓ Correctly denied (HTTP $http_code)${NC}"
else
    echo -e "  ${RED}✗ Should have been denied (HTTP $http_code)${NC}"
fi
echo ""

# Summary
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "${GREEN}All endpoint tests completed!${NC}"
echo ""
echo "Manual testing checklist:"
echo "  [ ] Navigate to Settings page in browser"
echo "  [ ] Test each tab (General, Security, Notifications, LLM, Database, Team)"
echo "  [ ] Verify form inputs update state"
echo "  [ ] Click 'Save Changes' and verify toast notification"
echo "  [ ] Test connection status indicators"
echo "  [ ] Test 2FA enable button"
echo "  [ ] Test session revocation"
echo "  [ ] Verify error handling for invalid inputs"
echo "  [ ] Test responsive design on mobile"
echo ""
echo -e "${YELLOW}Note: Some features require database persistence and full authentication${NC}"
echo "      to be implemented. See SETTINGS_ASSESSMENT.md for details."
echo ""

