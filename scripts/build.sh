#!/bin/bash

# Paper2Codes Build Script
# This script builds both the core backend and WebUI frontend

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CORE_DIR="$PROJECT_ROOT/Paper2Codes-Core"
WEBUI_DIR="$PROJECT_ROOT/Paper2Codes-WebUI"

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
BUILD_CORE=true
BUILD_WEBUI=true
SKIP_DEPENDENCIES=false
CORE_BUILD_SUCCESS=false
WEBUI_BUILD_SUCCESS=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --core-only)
            BUILD_WEBUI=false
            shift
            ;;
        --webui-only)
            BUILD_CORE=false
            shift
            ;;
        --skip-deps)
            SKIP_DEPENDENCIES=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Usage: $0 [--core-only] [--webui-only] [--skip-deps]"
            exit 1
            ;;
    esac
done

print_header "Building Paper2Codes Application"

# Build Core Backend
if [ "$BUILD_CORE" = true ]; then
    print_header "Building Core Backend"
    
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        print_error "Rust/Cargo is not installed. Please install Rust first."
        print_error "Visit: https://www.rust-lang.org/tools/install"
        CORE_BUILD_SUCCESS=false
    else
        print_info "Building Rust backend (release mode)..."
        cd "$CORE_DIR"
        
        if [ "$SKIP_DEPENDENCIES" = false ]; then
            print_info "Updating dependencies..."
            cargo update 2>&1 | grep -v "^[[:space:]]*$" || true
        fi
        
        if cargo build --release --features api 2>&1; then
            BACKEND_BINARY="$CORE_DIR/target/release/paper2codes"
            if [ -f "$BACKEND_BINARY" ]; then
                print_info "Backend built successfully: $BACKEND_BINARY"
                CORE_BUILD_SUCCESS=true
            else
                print_error "Backend binary not found after build"
                print_warn "Build may have completed but binary was not generated"
                CORE_BUILD_SUCCESS=false
            fi
        else
            print_error "Failed to build backend. There are compilation errors."
            print_warn "Please fix the compilation errors before running the application."
            CORE_BUILD_SUCCESS=false
        fi
    fi
fi

# Build WebUI Frontend
if [ "$BUILD_WEBUI" = true ]; then
    print_header "Building WebUI Frontend"
    
    # Check if Deno is available (preferred)
    USE_DENO=false
    if command -v deno &> /dev/null; then
        DENO_VERSION=$(deno --version | head -n1)
        print_info "Deno detected: $DENO_VERSION"
        # Check if deno.json exists and has build task
        if [ -f "$WEBUI_DIR/deno.json" ]; then
            USE_DENO=true
            print_info "Using Deno tasks for build"
        fi
    fi
    
    # Check if Node.js is installed (required for Next.js runtime)
    if ! command -v node &> /dev/null; then
        print_error "Node.js is not installed. Please install Node.js 18+ first."
        print_error "Visit: https://nodejs.org/"
        WEBUI_BUILD_SUCCESS=false
    else
        # Check Node.js version
        NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
        if [ "$NODE_VERSION" -lt 18 ]; then
            print_error "Node.js version 18+ is required. Current version: $(node -v)"
            WEBUI_BUILD_SUCCESS=false
        else
            cd "$WEBUI_DIR"
            
            # Install dependencies if needed
            if [ "$SKIP_DEPENDENCIES" = false ]; then
                if [ "$USE_DENO" = true ] && [ -f "$WEBUI_DIR/deno.json" ]; then
                    print_info "Checking dependencies with Deno..."
                    # Deno doesn't need separate install step for Next.js deps
                    # But we still need node_modules for Next.js
                    if [ ! -d "$WEBUI_DIR/node_modules" ]; then
                        print_info "Installing Node.js dependencies (required for Next.js)..."
                        if command -v npm &> /dev/null; then
                            npm install
                        elif command -v pnpm &> /dev/null; then
                            pnpm install
                        elif command -v yarn &> /dev/null; then
                            yarn install
                        else
                            print_error "No package manager found (npm/pnpm/yarn). Please install npm."
                            WEBUI_BUILD_SUCCESS=false
                            continue
                        fi
                    fi
                else
                    # Use npm/pnpm/yarn directly
                    if [ ! -d "$WEBUI_DIR/node_modules" ]; then
                        print_info "Installing dependencies..."
                        if command -v npm &> /dev/null; then
                            npm install
                        elif command -v pnpm &> /dev/null; then
                            pnpm install
                        elif command -v yarn &> /dev/null; then
                            yarn install
                        else
                            print_error "No package manager found (npm/pnpm/yarn). Please install npm."
                            WEBUI_BUILD_SUCCESS=false
                            continue
                        fi
                    fi
                fi
            fi
            
            # Build Next.js application
            print_info "Building Next.js application (production mode)..."
            if [ "$USE_DENO" = true ] && [ -f "$WEBUI_DIR/deno.json" ]; then
                if deno task build 2>&1; then
                    if [ -d "$WEBUI_DIR/.next" ]; then
                        print_info "Frontend built successfully using Deno: $WEBUI_DIR/.next"
                        WEBUI_BUILD_SUCCESS=true
                    else
                        print_error "Frontend build output not found"
                        WEBUI_BUILD_SUCCESS=false
                    fi
                else
                    print_error "Failed to build frontend with Deno task"
                    WEBUI_BUILD_SUCCESS=false
                fi
            else
                # Fallback to npm/pnpm/yarn
                if command -v npm &> /dev/null; then
                    if npm run build 2>&1; then
                        if [ -d "$WEBUI_DIR/.next" ]; then
                            print_info "Frontend built successfully using npm: $WEBUI_DIR/.next"
                            WEBUI_BUILD_SUCCESS=true
                        else
                            print_error "Frontend build output not found"
                            WEBUI_BUILD_SUCCESS=false
                        fi
                    else
                        print_error "Failed to build frontend"
                        WEBUI_BUILD_SUCCESS=false
                    fi
                elif command -v pnpm &> /dev/null; then
                    if pnpm run build 2>&1; then
                        if [ -d "$WEBUI_DIR/.next" ]; then
                            print_info "Frontend built successfully using pnpm: $WEBUI_DIR/.next"
                            WEBUI_BUILD_SUCCESS=true
                        else
                            print_error "Frontend build output not found"
                            WEBUI_BUILD_SUCCESS=false
                        fi
                    else
                        print_error "Failed to build frontend"
                        WEBUI_BUILD_SUCCESS=false
                    fi
                else
                    print_error "No package manager found (npm/pnpm/yarn)"
                    WEBUI_BUILD_SUCCESS=false
                fi
            fi
        fi
    fi
fi

# Print summary
print_header "Build Summary"
if [ "$BUILD_CORE" = true ]; then
    if [ "$CORE_BUILD_SUCCESS" = true ]; then
        print_info "✓ Core backend: SUCCESS"
        print_info "  Binary: $CORE_DIR/target/release/paper2codes"
    else
        print_error "✗ Core backend: FAILED"
        print_warn "  Please fix compilation errors before starting the application"
    fi
fi

if [ "$BUILD_WEBUI" = true ]; then
    if [ "$WEBUI_BUILD_SUCCESS" = true ]; then
        print_info "✓ WebUI frontend: SUCCESS"
        print_info "  Build output: $WEBUI_DIR/.next"
    else
        print_error "✗ WebUI frontend: FAILED"
    fi
fi

echo ""
if [ "$CORE_BUILD_SUCCESS" = true ] && [ "$WEBUI_BUILD_SUCCESS" = true ]; then
    print_info "Build Complete! Both components built successfully."
    print_info "To start the application, run: ./scripts/start.sh"
    exit 0
elif [ "$CORE_BUILD_SUCCESS" = true ] || [ "$WEBUI_BUILD_SUCCESS" = true ]; then
    print_warn "Partial build completed. Some components failed to build."
    exit 1
else
    print_error "Build failed for all components."
    exit 1
fi

