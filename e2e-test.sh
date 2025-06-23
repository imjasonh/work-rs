#!/bin/bash
set -euo pipefail

# End-to-end tests for Cloudflare Worker
# Usage: ./e2e-test.sh <base_url>

BASE_URL="${1:-http://localhost:8787}"
echo "Running E2E tests against: $BASE_URL"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to test endpoints
test_endpoint() {
    local test_name="$1"
    local method="$2"
    local endpoint="$3"
    local expected_status="$4"
    local data="${5:-}"
    local expected_content="${6:-}"

    echo -n "Testing $test_name... "

    if [ -n "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint" \
            -H "Content-Type: application/json" \
            -d "$data")
    else
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$BASE_URL$endpoint")
    fi

    status_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | sed '$d')

    if [ "$status_code" -eq "$expected_status" ]; then
        if [ -z "$expected_content" ] || echo "$body" | grep -q "$expected_content"; then
            echo -e "${GREEN}✓ PASSED${NC}"
            ((TESTS_PASSED++))
        else
            echo -e "${RED}✗ FAILED${NC} - Expected content not found"
            echo "  Expected: $expected_content"
            echo "  Got: $body"
            ((TESTS_FAILED++))
        fi
    else
        echo -e "${RED}✗ FAILED${NC} - Expected status $expected_status, got $status_code"
        echo "  Response: $body"
        ((TESTS_FAILED++))
    fi
}

echo "=== Basic Connectivity Tests ==="
test_endpoint "GET /" "GET" "/" 200 "" "Hello from Rust Workers"

echo -e "\n=== Counter Tests ==="
# Get initial counter value
test_endpoint "GET /counter" "GET" "/counter" 200 "" '"count"'

# Increment counter
test_endpoint "POST /counter/increment" "POST" "/counter/increment" 200 "" '"count"'

# Reset counter
test_endpoint "DELETE /counter" "DELETE" "/counter" 200 "" "Counter reset"

echo -e "\n=== Session Tests ==="
SESSION_ID="test-session-$(date +%s)"

# Create session
test_endpoint "PUT /session/$SESSION_ID" "PUT" "/session/$SESSION_ID" 200 '{"user_id":"test-user","data":{"theme":"dark"}}' "Session updated"

# Get session
test_endpoint "GET /session/$SESSION_ID" "GET" "/session/$SESSION_ID" 200 "" '"user_id":"test-user"'

# Delete session
test_endpoint "DELETE /session/$SESSION_ID" "DELETE" "/session/$SESSION_ID" 200 "" "Session cleared"

echo -e "\n=== R2 Storage Tests ==="
FILE_NAME="test-file-$(date +%s).txt"
FILE_CONTENT="Hello from E2E test at $(date)"

# Upload file
test_endpoint "PUT /files/$FILE_NAME" "PUT" "/files/$FILE_NAME" 200 "$FILE_CONTENT" '"sha256"'

# Download file
test_endpoint "GET /files/$FILE_NAME" "GET" "/files/$FILE_NAME" 200 "" "$FILE_CONTENT"

# List files
test_endpoint "GET /files/" "GET" "/files/" 200 "" "$FILE_NAME"

# Delete file
test_endpoint "DELETE /files/$FILE_NAME" "DELETE" "/files/$FILE_NAME" 200 "" "File deleted"

echo -e "\n=== Path Sanitization Security Tests ==="
# Test directory traversal attempts
test_endpoint "GET /files/../etc/passwd" "GET" "/files/../etc/passwd" 400 "" ""
test_endpoint "PUT /files/../../etc/passwd" "PUT" "/files/../../etc/passwd" 400 "malicious content" ""
test_endpoint "GET /session/../../../etc/passwd" "GET" "/session/../../../etc/passwd" 400 "" ""

echo -e "\n=== Test Summary ==="
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
