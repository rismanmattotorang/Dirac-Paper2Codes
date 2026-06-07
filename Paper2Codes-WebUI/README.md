# Paper2Codes WebUI

Modern web interface for the Paper2Codes framework, built with Next.js and Deno tooling.

## Overview

The WebUI provides a user-friendly interface for interacting with the Paper2Codes Core backend, enabling users to:

- Upload and process scientific papers
- Monitor code generation progress in real-time
- View generated code and documentation
- Access verification results
- Manage papers and generated repositories
- Track task status and progress via WebSocket updates

**For comprehensive integration details, API specifications, and WebSocket protocols, see [INTEGRATION.md](../INTEGRATION.md).**

## Technology Stack

- **Framework**: Next.js 16 (App Router)
- **Runtime**: Node.js 18+ (required for Next.js)
- **Development Tools**: Deno 2.4+ (for linting, formatting, type checking)
- **UI**: React 19, Tailwind CSS, Radix UI
- **Language**: TypeScript

## Prerequisites

- **Node.js** 18+ ([Install Node.js](https://nodejs.org/)) - Required for Next.js runtime
- **Deno** 2.4+ ([Install Deno](https://deno.land/manual/getting_started/installation)) - Required for development tooling

## Getting Started

### Installation

Dependencies are automatically managed by Deno when you run the tasks. No separate installation step is required!

### Development

**Using Deno Tasks (Recommended)**:

Deno tasks provide a unified interface, but Next.js runs on Node.js (as required):

```bash
deno task dev        # Start development server (Node.js runtime)
deno task build      # Build for production (Node.js runtime)
deno task start      # Start production server (Node.js runtime)
deno task lint       # Lint code with Deno
deno task fmt        # Format code with Deno
deno task fmt:check  # Check formatting without modifying files
deno task typecheck  # Type check with Deno
```

**Using npm/pnpm (Alternative)**:

You can also use npm directly:

```bash
npm install          # Install dependencies
npm run dev          # Start development server
npm run build        # Build for production
npm start            # Start production server
```

### Configuration

Create a `.env.local` file in the root of this directory:

```env
# API Configuration
NEXT_PUBLIC_API_URL=http://localhost:8080
NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws

# Feature Flags
NEXT_PUBLIC_ENABLE_ANALYTICS=true
NEXT_PUBLIC_ENABLE_REAL_TIME=true

# Environment
NEXT_PUBLIC_ENV=development
```

For detailed configuration options and API endpoint specifications, see [INTEGRATION.md](../INTEGRATION.md).

## Project Structure

```
Paper2Codes-WebUI/
├── app/                    # Next.js App Router pages
│   ├── layout.tsx         # Root layout
│   ├── page.tsx           # Home page
│   └── globals.css        # Global styles
├── components/            # React components
│   ├── dashboard/         # Dashboard components
│   ├── papers/            # Paper management
│   ├── code/              # Code generation UI
│   ├── verification/      # Verification dashboard
│   └── ui/                # Reusable UI components
├── lib/                   # Utility functions
├── hooks/                 # React hooks
├── public/                # Static assets
├── deno.json              # Deno configuration
├── package.json           # Node.js dependencies (for npm compatibility)
├── next.config.mjs        # Next.js configuration
└── tsconfig.json          # TypeScript configuration
```

## Development Notes

### Deno Integration

This project uses Deno for development tooling while Next.js runs on Node.js (as required by Next.js). The `deno.json` file configures:

- TypeScript compiler options
- Development tasks (dev, build, start) - these delegate to Node.js
- Code formatting and linting (pure Deno)
- Import maps for dependencies

**Note**: While Deno has Node.js compatibility, Next.js's compilation system requires native Node.js. The Deno tasks delegate to Node.js for running Next.js, while Deno handles all code quality tools (linting, formatting, type checking).

### Next.js Configuration

Next.js is configured to work alongside Deno tooling. The `next.config.mjs` includes settings for:

- TypeScript build errors (can be ignored during development)
- Image optimization settings

## Building for Production

```bash
# Build the application
deno task build

# Start production server
deno task start
```

The production build will be in the `.next` directory.

## Integration with Core

The WebUI communicates with the Paper2Codes Core backend through:

1. **REST API**: HTTP endpoints for all Core functionality
   - Papers management (upload, list, get, delete, process)
   - Repository management (list, get, download)
   - Task management (list, get, cancel, logs)
   - Module retrieval (get, content)
   - Authentication (login, logout, refresh, me)
   - Search and analytics endpoints

2. **WebSocket**: Real-time bidirectional communication
   - Task progress updates
   - Status change notifications
   - Live log streaming
   - Paper processing updates
   - Generation progress tracking

3. **Shared Storage**: SurrealDB (both Core and WebUI access the same database)

The WebUI includes:
- Type-safe API client with retry logic and error handling
- WebSocket client with automatic reconnection
- React hooks for API calls and WebSocket subscriptions
- State management with React Query and Zustand
- Real-time UI updates based on WebSocket events

For detailed API specifications, endpoint definitions, WebSocket message protocols, and integration architecture, see [INTEGRATION.md](../INTEGRATION.md).

**Prerequisites**: Ensure the Core backend API server is running and accessible at the configured API URL (default: `http://localhost:8080`).

## Contributing

When contributing to the WebUI:

1. Use Deno for formatting and linting: `deno task fmt` and `deno task lint`
2. Use Deno to run the development server: `deno task dev`
3. Follow the existing component structure
4. Ensure TypeScript types are properly defined
5. Test with the Core backend running

## License

MIT License - see the main project LICENSE file.
