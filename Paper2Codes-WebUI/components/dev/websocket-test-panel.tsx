/**
 * WebSocket test panel component for development
 * Allows testing WebSocket connection and viewing test results
 */

'use client';

import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  testWebSocketConnection,
  testWebSocketConnectionWithRetries,
  logWebSocketTestResults,
  type WebSocketTestResult,
} from '@/lib/utils/websocket-test';
import { IconLoader, IconCheckCircle, IconAlertCircle, IconWifi } from '@/components/ui/icons';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function WebSocketTestPanel() {
  const [isTesting, setIsTesting] = useState(false);
  const [results, setResults] = useState<WebSocketTestResult[]>([]);
  const [channels, setChannels] = useState('tasks,papers,repositories');

  const handleTest = async () => {
    setIsTesting(true);
    setResults([]);

    try {
      const channelList = channels
        .split(',')
        .map((c) => c.trim())
        .filter((c) => c.length > 0);

      const testResults = await testWebSocketConnectionWithRetries(
        channelList.length > 0 ? channelList : ['tasks'],
        3,
        5000,
      );

      setResults(testResults);
      logWebSocketTestResults(testResults);
    } catch (error) {
      console.error('WebSocket test error:', error);
      setResults([
        {
          success: false,
          connected: false,
          error: error instanceof Error ? error.message : 'Unknown error',
          timestamp: new Date().toISOString(),
        },
      ]);
    } finally {
      setIsTesting(false);
    }
  };

  const lastResult = results[results.length - 1];
  const overallSuccess = results.some((r) => r.success && r.connected);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <IconWifi className="w-5 h-5" />
          WebSocket Connection Test
        </CardTitle>
        <CardDescription>
          Test WebSocket connectivity to the backend server
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <label className="text-sm font-medium">Channels (comma-separated)</label>
          <input
            type="text"
            value={channels}
            onChange={(e) => setChannels(e.target.value)}
            placeholder="tasks,papers,repositories"
            className="w-full px-3 py-2 border border-border rounded-md bg-background text-foreground"
            disabled={isTesting}
          />
        </div>

        <Button
          onClick={handleTest}
          disabled={isTesting}
          className="w-full"
        >
          {isTesting ? (
            <>
              <IconLoader className="w-4 h-4 mr-2 animate-spin" />
              Testing...
            </>
          ) : (
            'Test Connection'
          )}
        </Button>

        {results.length > 0 && (
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              {overallSuccess ? (
                <>
                  <IconCheckCircle className="w-5 h-5 text-green-500" />
                  <span className="text-sm font-medium text-green-600 dark:text-green-400">
                    Connection Successful
                  </span>
                </>
              ) : (
                <>
                  <IconAlertCircle className="w-5 h-5 text-red-500" />
                  <span className="text-sm font-medium text-red-600 dark:text-red-400">
                    Connection Failed
                  </span>
                </>
              )}
            </div>

            <div className="space-y-2 max-h-64 overflow-y-auto">
              {results.map((result, index) => (
                <div
                  key={index}
                  className="p-3 rounded-lg border border-border bg-muted/50 text-sm"
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="font-medium">Attempt {index + 1}</span>
                    {result.success ? (
                      <IconCheckCircle className="w-4 h-4 text-green-500" />
                    ) : (
                      <IconAlertCircle className="w-4 h-4 text-red-500" />
                    )}
                  </div>
                  <div className="space-y-1 text-xs text-muted-foreground">
                    <div>
                      Status: {result.connected ? 'Connected' : 'Not Connected'}
                    </div>
                    {result.latency && (
                      <div>Latency: {result.latency}ms</div>
                    )}
                    {result.messageReceived && (
                      <div>Message Received: Yes</div>
                    )}
                    {result.error && (
                      <div className="text-red-600 dark:text-red-400">
                        Error: {result.error}
                      </div>
                    )}
                    <div>Time: {new Date(result.timestamp).toLocaleTimeString()}</div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

