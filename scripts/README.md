# Paper2Codes Scripts Documentation

This directory contains utility scripts for managing the Paper2Codes application.

## Available Scripts

### Startup Scripts

#### `start-app.sh`
Comprehensive application startup script with full health checks and dependency verification.

**Features:**
- ✅ Checks all prerequisites (Rust, Node.js, npm, Docker)
- ✅ Automatically builds backend if binary is missing
- ✅ Installs frontend dependencies if needed
- ✅ Starts SurrealDB if available via Docker
- ✅ Health checks for both backend and frontend
- ✅ Comprehensive error handling and logging
- ✅ Process management with PID files
- ✅ Lock file to prevent concurrent startups

**Usage:**
```bash
./scripts/start-app.sh
```

**Environment Variables:**
- `BACKEND_HOST` - Backend host (default: 127.0.0.1)
- `BACKEND_PORT` - Backend port (default: 8080)
- `FRONTEND_PORT` - Frontend port (default: 3000)

**Output:**
- Backend logs: `logs/backend.log`
- Frontend logs: `logs/frontend.log`
- PID files: `.backend.pid`, `.frontend.pid`

#### `start-backend.sh`
Starts only the backend API server.

**Usage:**
```bash
./scripts/start-backend.sh
```

#### `start-frontend.sh`
Starts only the frontend WebUI.

**Usage:**
```bash
./scripts/start-frontend.sh
```

#### `start.sh`
Starts both backend and frontend with improved error handling and status reporting.

**Features:**
- ✅ Starts backend and frontend in sequence
- ✅ Waits for backend to initialize before starting frontend
- ✅ Supports Deno tasks for frontend (if available)
- ✅ Graceful error handling - continues with available components
- ✅ Detailed status reporting

**Usage:**
```bash
# Start both components
./scripts/start.sh

# Start only backend
./scripts/start.sh --backend-only

# Start only frontend
./scripts/start.sh --frontend-only
```

**Environment Variables:**
- `BACKEND_HOST` - Backend host (default: 127.0.0.1)
- `BACKEND_PORT` - Backend port (default: 8080)
- `FRONTEND_PORT` - Frontend port (default: 3000)

### Stop Scripts

#### `stop-app.sh`
Safely stops the entire application with graceful shutdown.

**Features:**
- ✅ Graceful shutdown (SIGTERM first, SIGKILL if needed)
- ✅ Cleans up PID files
- ✅ Removes lock files
- ✅ Waits for processes to terminate

**Usage:**
```bash
./scripts/stop-app.sh
```

#### `stop-backend.sh`
Stops only the backend server.

**Usage:**
```bash
./scripts/stop-backend.sh
```

#### `stop-frontend.sh`
Stops only the frontend server.

**Usage:**
```bash
./scripts/stop-frontend.sh
```

#### `stop.sh`
Stops both backend and frontend with improved status reporting.

**Features:**
- ✅ Stops frontend and backend gracefully
- ✅ Detailed status reporting for each component
- ✅ Handles cases where components aren't running

**Usage:**
```bash
# Stop both components
./scripts/stop.sh

# Stop only backend
./scripts/stop.sh --backend-only

# Stop only frontend
./scripts/stop.sh --frontend-only
```

### Build Scripts

#### `build.sh`
Builds both the core backend and WebUI frontend.

**Features:**
- ✅ Builds Rust backend in release mode
- ✅ Builds Next.js frontend for production
- ✅ Supports Deno tasks for frontend (if available)
- ✅ Continues building even if one component fails
- ✅ Detailed build summary with success/failure status
- ✅ Auto-detects available package managers (npm/pnpm/yarn)

**Usage:**
```bash
# Build both components
./scripts/build.sh

# Build only backend
./scripts/build.sh --core-only

# Build only frontend
./scripts/build.sh --webui-only

# Skip dependency updates
./scripts/build.sh --skip-deps
```

**Output:**
- Backend binary: `Paper2Codes-Core/target/release/paper2codes`
- Frontend build: `Paper2Codes-WebUI/.next`

**Note:** If compilation errors occur in the backend, the script will report them but continue to build the frontend.

### Test Scripts

#### `test-app.sh`
Comprehensive test suite runner with multiple test types.

**Features:**
- ✅ Unit tests (Rust)
- ✅ Integration tests (Rust)
- ✅ Documentation tests (Rust)
- ✅ Frontend tests (TypeScript, ESLint, Jest)
- ✅ End-to-end tests (API)
- ✅ Code coverage reports (optional)
- ✅ Clippy linting
- ✅ Format checking
- ✅ Detailed test results and reports

**Usage:**
```bash
# Run all tests
./scripts/test-app.sh

# Run with coverage
./scripts/test-app.sh --coverage

# Run only Rust tests
./scripts/test-app.sh --no-frontend --no-e2e

# Verbose output
./scripts/test-app.sh --verbose

# Help
./scripts/test-app.sh --help
```

**Options:**
- `--no-rust` - Skip Rust backend tests
- `--no-frontend` - Skip frontend tests
- `--no-e2e` - Skip end-to-end tests
- `--no-integration` - Skip integration tests
- `--coverage` - Generate coverage reports
- `--verbose, -v` - Verbose output
- `--help, -h` - Show help message

**Output:**
- Test results: `test-results/`
- Test logs: `test-results/test-run.log`
- Coverage reports: `test-results/coverage/` (if enabled)

#### `run_tests.sh` (Core)
Runs tests for the Rust backend only.

**Usage:**
```bash
cd Paper2Codes-Core
./scripts/run_tests.sh
```

### Setup Scripts

#### `setup.sh` (Core)
Sets up the development environment.

**Usage:**
```bash
cd Paper2Codes-Core
./scripts/setup.sh
```

#### `start_surrealdb.sh` (Core)
Starts SurrealDB for development.

**Usage:**
```bash
cd Paper2Codes-Core
./scripts/start_surrealdb.sh
```

## Quick Start Guide

### Building the Application

1. **Build everything:**
   ```bash
   ./scripts/build.sh
   ```

2. **Build only specific components:**
   ```bash
   ./scripts/build.sh --core-only    # Build only backend
   ./scripts/build.sh --webui-only   # Build only frontend
   ```

**Note:** If you encounter compilation errors in the Rust backend, the script will continue and attempt to build the frontend. Fix the compilation errors before starting the application.

### Starting the Application

1. **Build first (if not already built):**
   ```bash
   ./scripts/build.sh
   ```

2. **Start everything:**
   ```bash
   ./scripts/start.sh
   ```
   
   Or use the comprehensive startup script:
   ```bash
   ./scripts/start-app.sh
   ```

3. **Access the services:**
   - Backend API: http://127.0.0.1:8080
   - Frontend WebUI: http://localhost:3000
   - Health Check: http://127.0.0.1:8080/api/health

4. **View logs:**
   ```bash
   tail -f logs/backend.log
   tail -f logs/frontend.log
   ```

### Stopping the Application

```bash
./scripts/stop-app.sh
```

### Running Tests

1. **Run all tests:**
   ```bash
   ./scripts/test-app.sh
   ```

2. **Run with coverage:**
   ```bash
   ./scripts/test-app.sh --coverage
   ```

3. **View test results:**
   ```bash
   cat test-results/test-run.log
   ```

## Troubleshooting

### Backend fails to start

1. Check if port 8080 is already in use:
   ```bash
   lsof -i :8080
   ```

2. Check backend logs:
   ```bash
   tail -50 logs/backend.log
   ```

3. Verify configuration:
   ```bash
   paper2codes config
   ```

4. Check API keys are set:
   ```bash
   echo $OPENROUTER_API_KEY
   ```

### Frontend fails to start

1. Check if port 3000 is already in use:
   ```bash
   lsof -i :3000
   ```

2. Check frontend logs:
   ```bash
   tail -50 logs/frontend.log
   ```

3. Reinstall dependencies:
   ```bash
   cd Paper2Codes-WebUI
   rm -rf node_modules package-lock.json
   npm install
   ```
   
   Or if using Deno:
   ```bash
   cd Paper2Codes-WebUI
   deno task build
   ```

### Build fails

1. **Backend build errors:**
   - Fix compilation errors reported by Cargo
   - Check Rust version: `rustc --version` (should be 1.70+)
   - Update dependencies: `cd Paper2Codes-Core && cargo update`

2. **Frontend build errors:**
   - Check Node.js version: `node -v` (should be 18+)
   - Clear build cache: `cd Paper2Codes-WebUI && rm -rf .next node_modules`
   - Reinstall dependencies: `npm install` or use Deno tasks
   - Check for TypeScript errors: `deno task typecheck`

### Tests are failing

1. Check test logs:
   ```bash
   cat test-results/test-run.log
   ```

2. Run tests individually:
   ```bash
   cd Paper2Codes-Core
   cargo test --lib  # Unit tests
   cargo test --test '*'  # Integration tests
   ```

3. Check for linting issues:
   ```bash
   cd Paper2Codes-Core
   cargo clippy --all-targets --all-features
   cargo fmt --all -- --check
   ```

## Script Architecture

All scripts follow these principles:

1. **Error Handling:** Use `set -euo pipefail` for strict error handling
2. **Color Output:** Use ANSI color codes for better readability
3. **Logging:** All output is logged to files for debugging
4. **Idempotency:** Scripts can be safely run multiple times
5. **Cleanup:** Proper cleanup of resources and processes
6. **Health Checks:** Wait for services to be ready before continuing
7. **Process Management:** Use PID files for process tracking

## Environment Variables

Common environment variables used by scripts:

- `BACKEND_HOST` - Backend API host (default: 127.0.0.1)
- `BACKEND_PORT` - Backend API port (default: 8080)
- `FRONTEND_PORT` - Frontend port (default: 3000)
- `OPENROUTER_API_KEY` - OpenRouter API key for LLM access
- `OPENAI_API_KEY` - OpenAI API key (alternative)
- `ANTHROPIC_API_KEY` - Anthropic API key (alternative)
- `RUST_LOG` - Rust logging level (default: info)

## File Locations

- **Logs:** `logs/`
- **PID Files:** `.backend.pid`, `.frontend.pid`
- **Lock Files:** `.startup.lock`
- **Test Results:** `test-results/`
- **Config:** `~/.config/paper2codes/config.toml`

## Contributing

When adding new scripts:

1. Add proper error handling
2. Use colored output for user feedback
3. Log all operations to files
4. Clean up resources on exit
5. Document usage in this README
6. Make scripts executable (`chmod +x`)
7. Test scripts on both macOS and Linux

## See Also

- [Main README](../README.md) - Project overview
- [Architecture](../ARCHITECTURE.md) - System architecture
- [Specs](../SPECS.md) - Technical specifications

