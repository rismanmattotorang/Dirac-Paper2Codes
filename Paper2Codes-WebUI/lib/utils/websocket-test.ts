/**
 * WebSocket connection test utility
 * Used for testing WebSocket connectivity and debugging
 */

import { WS_BASE_URL } from '@/lib/config';

export interface WebSocketTestResult {
  success: boolean;
  connected: boolean;
  error?: string;
  latency?: number;
  messageReceived?: boolean;
  timestamp: string;
}

/**
 * Test WebSocket connection
 */
export async function testWebSocketConnection(
  channels: string[] = ['tasks'],
  timeout: number = 5000,
): Promise<WebSocketTestResult> {
  const startTime = Date.now();
  const result: WebSocketTestResult = {
    success: false,
    connected: false,
    timestamp: new Date().toISOString(),
  };

  return new Promise((resolve) => {
    try {
      // Get auth token
      const token =
        typeof window !== 'undefined' ? localStorage.getItem('auth_token') : null;

      // Build WebSocket URL
      const urlObj = new URL(WS_BASE_URL);
      if (channels.length > 0) {
        urlObj.searchParams.set('channels', channels.join(','));
      }
      if (token) {
        urlObj.searchParams.set('token', token);
      }

      const ws = new WebSocket(urlObj.toString());
      let messageReceived = false;
      let timeoutId: ReturnType<typeof setTimeout>;

      // Set timeout
      timeoutId = setTimeout(() => {
        ws.close();
        if (!result.connected) {
          result.error = 'Connection timeout';
          resolve(result);
        } else if (!messageReceived) {
          result.success = true;
          result.connected = true;
          result.latency = Date.now() - startTime;
          result.error = 'Connected but no message received within timeout';
          resolve(result);
        }
      }, timeout);

      ws.onopen = () => {
        result.connected = true;
        const latency = Date.now() - startTime;
        result.latency = latency;

        // Send a test message
        ws.send(
          JSON.stringify({
            type: 'subscribe',
            channels: channels,
          }),
        );
      };

      ws.onmessage = (event) => {
        messageReceived = true;
        clearTimeout(timeoutId);

        try {
          const message = JSON.parse(event.data);
          result.messageReceived = true;
          result.success = true;
          result.latency = Date.now() - startTime;

          // Close connection after receiving message
          ws.close();
          resolve(result);
        } catch (error) {
          result.error = `Failed to parse message: ${error instanceof Error ? error.message : 'Unknown error'}`;
          ws.close();
          resolve(result);
        }
      };

      ws.onerror = (error) => {
        clearTimeout(timeoutId);
        result.error = 'WebSocket error occurred';
        result.connected = false;
        ws.close();
        resolve(result);
      };

      ws.onclose = (event) => {
        clearTimeout(timeoutId);
        if (!result.success && !result.error) {
          if (event.code === 1006) {
            result.error = 'Connection closed abnormally (server may be down)';
          } else if (event.code === 1000) {
            // Normal closure
            if (result.connected && messageReceived) {
              result.success = true;
            }
          } else {
            result.error = `Connection closed with code ${event.code}: ${event.reason || 'Unknown reason'}`;
          }
        }
        if (!result.error && result.connected) {
          result.success = true;
        }
        resolve(result);
      };
    } catch (error) {
      result.error = `Failed to create WebSocket: ${error instanceof Error ? error.message : 'Unknown error'}`;
      resolve(result);
    }
  });
}

/**
 * Test WebSocket connection with multiple attempts
 */
export async function testWebSocketConnectionWithRetries(
  channels: string[] = ['tasks'],
  maxAttempts: number = 3,
  timeout: number = 5000,
): Promise<WebSocketTestResult[]> {
  const results: WebSocketTestResult[] = [];

  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    console.log(`WebSocket test attempt ${attempt}/${maxAttempts}...`);
    const result = await testWebSocketConnection(channels, timeout);
    results.push(result);

    if (result.success && result.connected) {
      console.log(`WebSocket test successful on attempt ${attempt}`);
      break;
    }

    // Wait before retry (exponential backoff)
    if (attempt < maxAttempts) {
      const delay = Math.min(1000 * Math.pow(2, attempt - 1), 5000);
      await new Promise((resolve) => setTimeout(resolve, delay));
    }
  }

  return results;
}

/**
 * Log WebSocket test results
 */
export function logWebSocketTestResults(results: WebSocketTestResult[]): void {
  console.group('WebSocket Connection Test Results');
  results.forEach((result, index) => {
    console.log(`Attempt ${index + 1}:`, {
      success: result.success ? '✅' : '❌',
      connected: result.connected ? 'Yes' : 'No',
      latency: result.latency ? `${result.latency}ms` : 'N/A',
      messageReceived: result.messageReceived ? 'Yes' : 'No',
      error: result.error || 'None',
      timestamp: result.timestamp,
    });
  });
  console.groupEnd();
}

