# Environment Configuration Guide

This guide explains how to configure environment variables for the Paper2Codes WebUI, particularly for WebSocket connections.

## Environment Variables

Create a `.env.local` file in the `Paper2Codes-WebUI` directory with the following variables:

```bash
# API Configuration
# Backend API base URL (REST API)
NEXT_PUBLIC_API_URL=http://localhost:8080

# WebSocket URL for real-time updates
# Use ws:// for development, wss:// for production
NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws

# Feature Flags
# Enable analytics tracking
NEXT_PUBLIC_ENABLE_ANALYTICS=false

# Enable real-time WebSocket features
NEXT_PUBLIC_ENABLE_REAL_TIME=true

# Environment
# Options: development, staging, production
NEXT_PUBLIC_ENV=development
```

## Configuration Details

### API URL (`NEXT_PUBLIC_API_URL`)

- **Development**: `http://localhost:8080`
- **Production**: `https://api.yourdomain.com`
- This is the base URL for all REST API requests

### WebSocket URL (`NEXT_PUBLIC_WS_URL`)

- **Development**: `ws://localhost:8080/ws`
- **Production**: `wss://api.yourdomain.com/ws` (use `wss://` for secure WebSocket)
- This is the WebSocket endpoint for real-time updates
- Must match the backend WebSocket server endpoint

### Feature Flags

- `NEXT_PUBLIC_ENABLE_ANALYTICS`: Enable/disable analytics tracking
- `NEXT_PUBLIC_ENABLE_REAL_TIME`: Enable/disable WebSocket real-time features

### Environment

- `NEXT_PUBLIC_ENV`: Current environment (affects logging, error handling, etc.)

## Testing WebSocket Connection

### Using the Test Panel

1. Import the `WebSocketTestPanel` component in your development page:

```tsx
import { WebSocketTestPanel } from '@/components/dev/websocket-test-panel';

// In your component
<WebSocketTestPanel />
```

2. The test panel allows you to:
   - Test WebSocket connectivity
   - Specify channels to subscribe to
   - View connection latency
   - See detailed test results

### Using the Test Utility

You can also test programmatically:

```typescript
import { testWebSocketConnection, logWebSocketTestResults } from '@/lib/utils/websocket-test';

// Test single connection
const result = await testWebSocketConnection(['tasks'], 5000);
console.log(result);

// Test with retries
const results = await testWebSocketConnectionWithRetries(['tasks', 'papers'], 3, 5000);
logWebSocketTestResults(results);
```

## Troubleshooting

### WebSocket Connection Fails

1. **Check backend is running**: Ensure the Paper2Codes-Core backend is running on the configured port
2. **Check WebSocket URL**: Verify `NEXT_PUBLIC_WS_URL` matches the backend WebSocket endpoint
3. **Check CORS**: Ensure backend allows WebSocket connections from your frontend origin
4. **Check firewall**: Ensure the WebSocket port is not blocked

### Connection Drops Frequently

1. **Check network stability**: WebSocket connections require stable network
2. **Check backend logs**: Look for errors in the backend WebSocket handler
3. **Check heartbeat**: The WebSocket uses 30-second heartbeats - ensure these are working

### Messages Not Received

1. **Check channel subscriptions**: Ensure you're subscribed to the correct channels
2. **Check backend broadcasting**: Verify the backend is broadcasting messages to the correct channels
3. **Check browser console**: Look for WebSocket errors in the browser console

## Production Deployment

For production:

1. Use `wss://` (secure WebSocket) instead of `ws://`
2. Ensure SSL/TLS certificates are properly configured
3. Set `NEXT_PUBLIC_ENV=production`
4. Disable development features like the test panel
5. Configure proper CORS and security headers on the backend

## Example Configuration Files

### Development (`.env.local`)

```bash
NEXT_PUBLIC_API_URL=http://localhost:8080
NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws
NEXT_PUBLIC_ENABLE_ANALYTICS=false
NEXT_PUBLIC_ENABLE_REAL_TIME=true
NEXT_PUBLIC_ENV=development
```

### Production (`.env.production`)

```bash
NEXT_PUBLIC_API_URL=https://api.paper2codes.com
NEXT_PUBLIC_WS_URL=wss://api.paper2codes.com/ws
NEXT_PUBLIC_ENABLE_ANALYTICS=true
NEXT_PUBLIC_ENABLE_REAL_TIME=true
NEXT_PUBLIC_ENV=production
```

