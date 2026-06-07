#!/bin/bash
# Paper2Codes Comprehensive Test Suite
# This script runs all tests including unit, integration, and end-to-end tests

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CORE_DIR="$PROJECT_ROOT/Paper2Codes-Core"
WEBUI_DIR="$PROJECT_ROOT/Paper2Codes-WebUI"
TEST_RESULTS_DIR="$PROJECT_ROOT/test-results"
TEST_LOG_FILE="$TEST_RESULTS_DIR/test-run.log"

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly CYAN='\033[0;36m'
readonly MAGENTA='\033[0;35m'
readonly NC='\033[0m' # No Color
readonly BOLD='\033[1m'

# Test configuration
readonly RUST_TESTS=true
readonly FRONTEND_TESTS=true
readonly E2E_TESTS=true
readonly INTEGRATION_TESTS=true
readonly COVERAGE=false  # Set to true to generate coverage reports
readonly PARALLEL=true   # Run tests in parallel when possible
readonly VERBOSE=false   # Set to true for verbose output

# Statistics
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0
TEST_SUITE_FAILURES=0

# Function to print colored messages
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1" | tee -a "$TEST_LOG_FILE"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$TEST_LOG_FILE"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$TEST_LOG_FILE"
}

print_header() {
    echo -e "${BLUE}${BOLD}========================================${NC}" | tee -a "$TEST_LOG_FILE"
    echo -e "${BLUE}${BOLD}$1${NC}" | tee -a "$TEST_LOG_FILE"
    echo -e "${BLUE}${BOLD}========================================${NC}" | tee -a "$TEST_LOG_FILE"
}

print_success() {
    echo -e "${GREEN}${BOLD}✓${NC} $1" | tee -a "$TEST_LOG_FILE"
}

print_failure() {
    echo -e "${RED}${BOLD}✗${NC} $1" | tee -a "$TEST_LOG_FILE"
}

print_step() {
    echo -e "${CYAN}[TEST]${NC} $1" | tee -a "$TEST_LOG_FILE"
}

# Create test results directory
mkdir -p "$TEST_RESULTS_DIR"
rm -f "$TEST_LOG_FILE"

# Start logging
exec 1> >(tee -a "$TEST_LOG_FILE")
exec 2> >(tee -a "$TEST_LOG_FILE" >&2)

print_header "Paper2Codes Comprehensive Test Suite"
echo ""

# Check prerequisites
print_step "Checking prerequisites..."

# Check Rust
if ! command -v cargo &> /dev/null; then
    print_error "Rust/Cargo is not installed. Please install Rust first."
    exit 1
fi
print_success "Rust/Cargo found: $(cargo --version | cut -d' ' -f2)"

# Check Node.js (for frontend tests)
if [ "$FRONTEND_TESTS" = true ] || [ "$E2E_TESTS" = true ]; then
    if ! command -v node &> /dev/null; then
        print_warn "Node.js is not installed. Skipping frontend tests."
        FRONTEND_TESTS=false
        E2E_TESTS=false
    else
        print_success "Node.js found: $(node -v)"
        
        # Check npm
        if ! command -v npm &> /dev/null; then
            print_warn "npm is not installed. Skipping frontend tests."
            FRONTEND_TESTS=false
            E2E_TESTS=false
        else
            print_success "npm found: $(npm -v)"
        fi
    fi
fi

# Check for test dependencies
print_step "Installing test dependencies..."

cd "$CORE_DIR"
if [ "$COVERAGE" = true ] && ! command -v cargo-tarpaulin &> /dev/null; then
    print_info "Installing cargo-tarpaulin for coverage..."
    cargo install cargo-tarpaulin || print_warn "Failed to install cargo-tarpaulin"
fi

cd "$WEBUI_DIR"
if [ "$FRONTEND_TESTS" = true ] && [ ! -d "$WEBUI_DIR/node_modules" ]; then
    print_info "Installing frontend dependencies..."
    npm install --silent || print_warn "Failed to install frontend dependencies"
fi

echo ""

# Function to run Rust tests
run_rust_tests() {
    print_header "Rust Backend Tests"
    
    cd "$CORE_DIR"
    local test_output_file="$TEST_RESULTS_DIR/rust-tests.log"
    
    # Unit tests
    print_step "Running unit tests..."
    local unit_args="--lib"
    if [ "$VERBOSE" = true ]; then
        unit_args="$unit_args -- --nocapture"
    fi
    
    if cargo test $unit_args 2>&1 | tee "$test_output_file"; then
        print_success "Unit tests passed"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        print_failure "Unit tests failed"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        TEST_SUITE_FAILURES=$((TEST_SUITE_FAILURES + 1))
        return 1
    fi
    
    # Integration tests
    if [ "$INTEGRATION_TESTS" = true ]; then
        print_step "Running integration tests..."
        local integration_args="--test '*'"
        if [ "$VERBOSE" = true ]; then
            integration_args="$integration_args -- --nocapture"
        fi
        
        if cargo test $integration_args 2>&1 | tee -a "$test_output_file"; then
            print_success "Integration tests passed"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            print_failure "Integration tests failed"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            TEST_SUITE_FAILURES=$((TEST_SUITE_FAILURES + 1))
            return 1
        fi
    fi
    
    # Doc tests
    print_step "Running documentation tests..."
    if cargo test --doc 2>&1 | tee -a "$test_output_file"; then
        print_success "Documentation tests passed"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        print_failure "Documentation tests failed"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        TEST_SUITE_FAILURES=$((TEST_SUITE_FAILURES + 1))
        return 1
    fi
    
    # Coverage report (if enabled)
    if [ "$COVERAGE" = true ] && command -v cargo-tarpaulin &> /dev/null; then
        print_step "Generating coverage report..."
        cargo tarpaulin --out Html --output-dir "$TEST_RESULTS_DIR/coverage" 2>&1 | tee -a "$test_output_file" || true
        print_info "Coverage report: $TEST_RESULTS_DIR/coverage/tarpaulin-report.html"
    fi
    
    # Clippy linting
    print_step "Running clippy linting..."
    if cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee -a "$test_output_file"; then
        print_success "Clippy checks passed"
    else
        print_failure "Clippy checks failed"
        TEST_SUITE_FAILURES=$((TEST_SUITE_FAILURES + 1))
    fi
    
    # Format check
    print_step "Checking code format..."
    if cargo fmt --all -- --check 2>&1 | tee -a "$test_output_file"; then
        print_success "Format check passed"
    else
        print_warn "Code format issues found. Run 'cargo fmt' to fix."
    fi
    
    echo ""
    return 0
}

# Function to run frontend tests
run_frontend_tests() {
    print_header "Frontend Tests"
    
    cd "$WEBUI_DIR"
    local test_output_file="$TEST_RESULTS_DIR/frontend-tests.log"
    
    if [ ! -d "$WEBUI_DIR/node_modules" ]; then
        print_warn "Frontend dependencies not found. Skipping frontend tests."
        SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
        return 0
    fi
    
    # Type checking
    print_step "Running TypeScript type checking..."
    if npm run type-check 2>&1 | tee "$test_output_file"; then
        print_success "Type checking passed"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        print_warn "Type checking found issues"
        # Don't fail the suite for type check warnings
    fi
    
    # ESLint
    print_step "Running ESLint..."
    if npm run lint 2>&1 | tee -a "$test_output_file"; then
        print_success "ESLint checks passed"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        print_warn "ESLint found issues"
        # Don't fail the suite for lint warnings
    fi
    
    # Jest tests (if available)
    if grep -q "\"test\":" "$WEBUI_DIR/package.json"; then
        print_step "Running Jest tests..."
        local jest_args=""
        if [ "$VERBOSE" = true ]; then
            jest_args="--verbose"
        fi
        
        if npm test -- $jest_args 2>&1 | tee -a "$test_output_file"; then
            print_success "Jest tests passed"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            print_failure "Jest tests failed"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            TEST_SUITE_FAILURES=$((TEST_SUITE_FAILURES + 1))
            return 1
        fi
    else
        print_info "No Jest test script found. Skipping."
    fi
    
    echo ""
    return 0
}

# Function to run end-to-end tests
run_e2e_tests() {
    print_header "End-to-End Tests"
    
    local test_output_file="$TEST_RESULTS_DIR/e2e-tests.log"
    
    # Check if backend is running
    local backend_url="${BACKEND_HOST:-127.0.0.1}:${BACKEND_PORT:-8080}"
    print_step "Checking backend availability..."
    
    if curl -sf "http://$backend_url/api/health" > /dev/null 2>&1; then
        print_success "Backend is running"
    else
        print_warn "Backend is not running. Starting backend for E2E tests..."
        
        cd "$CORE_DIR"
        if [ ! -f "$CORE_DIR/target/release/paper2codes" ]; then
            print_info "Building backend..."
            cargo build --release --features api || {
                print_error "Failed to build backend for E2E tests"
                SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
                return 0
            }
        fi
        
        # Start backend in background
        local e2e_backend_pid_file="$PROJECT_ROOT/.e2e-backend.pid"
        local e2e_backend_log="$PROJECT_ROOT/logs/e2e-backend.log"
        mkdir -p "$PROJECT_ROOT/logs"
        
        "$CORE_DIR/target/release/paper2codes" api \
            --host "${BACKEND_HOST:-127.0.0.1}" \
            --port "${BACKEND_PORT:-8080}" \
            > "$e2e_backend_log" 2>&1 &
        local e2e_backend_pid=$!
        echo "$e2e_backend_pid" > "$e2e_backend_pid_file"
        
        # Wait for backend to start
        print_info "Waiting for backend to start..."
        local backend_ready=false
        for i in {1..30}; do
            if curl -sf "http://$backend_url/api/health" > /dev/null 2>&1; then
                backend_ready=true
                break
            fi
            sleep 1
        done
        
        if [ "$backend_ready" = false ]; then
            print_error "Backend failed to start for E2E tests"
            kill "$e2e_backend_pid" 2>/dev/null || true
            rm -f "$e2e_backend_pid_file"
            SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
            return 0
        fi
        
        print_success "Backend started for E2E tests (PID: $e2e_backend_pid)"
        
        # Cleanup function for E2E backend
        cleanup_e2e_backend() {
            if [ -f "$e2e_backend_pid_file" ]; then
                local pid=$(cat "$e2e_backend_pid_file")
                kill "$pid" 2>/dev/null || true
                rm -f "$e2e_backend_pid_file"
            fi
        }
        trap cleanup_e2e_backend EXIT
    fi
    
    # Run API E2E tests
    print_step "Running API E2E tests..."
    cd "$CORE_DIR"
    
    if cargo test --test e2e_test --features api 2>&1 | tee "$test_output_file"; then
        print_success "API E2E tests passed"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        print_failure "API E2E tests failed"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        TEST_SUITE_FAILURES=$((TEST_SUITE_FAILURES + 1))
    fi
    
    echo ""
    return 0
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-rust)
            RUST_TESTS=false
            shift
            ;;
        --no-frontend)
            FRONTEND_TESTS=false
            shift
            ;;
        --no-e2e)
            E2E_TESTS=false
            shift
            ;;
        --no-integration)
            INTEGRATION_TESTS=false
            shift
            ;;
        --coverage)
            COVERAGE=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Paper2Codes Test Suite"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --no-rust          Skip Rust backend tests"
            echo "  --no-frontend      Skip frontend tests"
            echo "  --no-e2e           Skip end-to-end tests"
            echo "  --no-integration   Skip integration tests"
            echo "  --coverage         Generate coverage reports"
            echo "  --verbose, -v      Verbose output"
            echo "  --help, -h         Show this help message"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Record start time
START_TIME=$(date +%s)

# Run test suites
if [ "$RUST_TESTS" = true ]; then
    run_rust_tests || true
fi

if [ "$FRONTEND_TESTS" = true ]; then
    run_frontend_tests || true
fi

if [ "$E2E_TESTS" = true ]; then
    run_e2e_tests || true
fi

# Calculate duration
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Print summary
print_header "Test Summary"
echo ""
echo -e "${BOLD}Results:${NC}"
echo -e "  ${GREEN}Passed:${NC}  $PASSED_TESTS"
echo -e "  ${RED}Failed:${NC}  $FAILED_TESTS"
echo -e "  ${YELLOW}Skipped:${NC} $SKIPPED_TESTS"
echo -e "  ${BOLD}Duration:${NC} ${DURATION}s"
echo ""
echo -e "${BOLD}Test Results Directory:${NC} $TEST_RESULTS_DIR"
echo -e "${BOLD}Test Log:${NC} $TEST_LOG_FILE"
echo ""

if [ "$COVERAGE" = true ] && [ -d "$TEST_RESULTS_DIR/coverage" ]; then
    echo -e "${BOLD}Coverage Report:${NC} $TEST_RESULTS_DIR/coverage/tarpaulin-report.html"
    echo ""
fi

# Exit with error if any test suite failed
if [ $TEST_SUITE_FAILURES -gt 0 ]; then
    print_failure "Some test suites failed. Check logs for details."
    exit 1
else
    print_success "All test suites passed!"
    exit 0
fi

