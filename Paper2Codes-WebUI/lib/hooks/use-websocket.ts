/**
 * React hook for WebSocket connections with exponential backoff reconnection logic
 * and React Query integration for state synchronization
 */

'use client';

import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { WS_BASE_URL, config } from '@/lib/config';

export interface WebSocketMessage {
  type: string;
  channel: string;
  data: unknown;
  timestamp: string;
}

export interface UseWebSocketOptions {
  url?: string;
  channels?: string[];
  onMessage?: (message: WebSocketMessage) => void;
  autoReconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
  enabled?: boolean;
  heartbeatInterval?: number;
}

export interface UseWebSocketReturn {
  isConnected: boolean;
  isConnecting: boolean;
  connectionState: 'connecting' | 'connected' | 'disconnected' | 'error';
  messages: WebSocketMessage[];
  send: (message: unknown) => void;
  subscribe: (channel: string) => void;
  unsubscribe: (channel: string) => void;
  reconnect: () => void;
  disconnect: () => void;
  lastError: Error | null;
}

const realtimeDefaults = config.realtime;

const DEFAULT_RECONNECT_INTERVAL = realtimeDefaults.reconnectIntervalMs;
const DEFAULT_MAX_RECONNECT_ATTEMPTS = realtimeDefaults.maxReconnectAttempts;
const DEFAULT_HEARTBEAT_INTERVAL = realtimeDefaults.heartbeatIntervalMs;
const MAX_RECONNECT_DELAY = 30000; // Max 30 seconds

/**
 * Calculate exponential backoff delay with jitter
 * Formula: baseDelay * (2 ^ attempt) + random(0, baseDelay)
 */
function calculateBackoffDelay(attempt: number, baseDelay: number): number {
  const exponentialDelay = baseDelay * Math.pow(2, attempt);
  const jitter = Math.random() * baseDelay;
  return Math.min(exponentialDelay + jitter, MAX_RECONNECT_DELAY);
}

export function useWebSocket(options: UseWebSocketOptions = {}): UseWebSocketReturn {
  const {
    url = WS_BASE_URL,
    channels = [],
    onMessage,
    autoReconnect = config.realtime.autoReconnect,
    reconnectInterval = DEFAULT_RECONNECT_INTERVAL,
    maxReconnectAttempts = DEFAULT_MAX_RECONNECT_ATTEMPTS,
    enabled = true,
    heartbeatInterval = DEFAULT_HEARTBEAT_INTERVAL,
  } = options;

  const queryClient = useQueryClient();

  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [connectionState, setConnectionState] = useState<
    'connecting' | 'connected' | 'disconnected' | 'error'
  >('disconnected');
  const [messages, setMessages] = useState<WebSocketMessage[]>([]);
  const [lastError, setLastError] = useState<Error | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const messageQueueRef = useRef<unknown[]>([]);
  const heartbeatIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const isManualDisconnectRef = useRef(false);
  const channelsRef = useRef<string[]>(channels);
  const onMessageRef = useRef(onMessage);

  // Keep refs updated
  useEffect(() => {
    channelsRef.current = channels;
    onMessageRef.current = onMessage;
  }, [channels, onMessage]);

  /**
   * Send heartbeat to keep connection alive
   */
  const startHeartbeat = useCallback(() => {
    if (heartbeatIntervalRef.current) {
      clearInterval(heartbeatIntervalRef.current);
    }

      heartbeatIntervalRef.current = setInterval(() => {
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        try {
          // Heartbeat doesn't need to be sent - backend sends them to us
          // We just respond to keep the connection alive
        } catch (error) {
          console.warn('Failed to send heartbeat:', error);
        }
      }
    }, heartbeatInterval);
  }, [heartbeatInterval]);

  /**
   * Stop heartbeat
   */
  const stopHeartbeat = useCallback(() => {
    if (heartbeatIntervalRef.current) {
      clearInterval(heartbeatIntervalRef.current);
      heartbeatIntervalRef.current = null;
    }
  }, []);

  /**
   * Clear reconnection timeout
   */
  const clearReconnectTimeout = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
  }, []);

  /**
   * Handle WebSocket messages and sync with React Query
   */
  const handleMessage = useCallback(
    (message: WebSocketMessage) => {
      setMessages((prev) => {
        // Keep only last 100 messages to prevent memory issues
        const newMessages = [...prev, message].slice(-100);
        return newMessages;
      });

      // Call custom message handler
      onMessageRef.current?.(message);

      // Sync with React Query based on message type
      if (message.type === 'task_update' && message.data) {
        const taskData = message.data as { task_id: string; [key: string]: unknown };
        // Invalidate and refetch task queries
        queryClient.invalidateQueries({ queryKey: ['tasks', taskData.task_id] });
        queryClient.invalidateQueries({ queryKey: ['tasks'] });
      } else if (message.type === 'paper_processed' && message.data) {
        const paperData = message.data as { paper_id: string; [key: string]: unknown };
        queryClient.invalidateQueries({ queryKey: ['papers', paperData.paper_id] });
        queryClient.invalidateQueries({ queryKey: ['papers'] });
      } else if (message.type === 'generation_progress' && message.data) {
        const genData = message.data as { repository_id: string; [key: string]: unknown };
        queryClient.invalidateQueries({ queryKey: ['repositories', genData.repository_id] });
      }
    },
    [queryClient],
  );

  /**
   * Connect to WebSocket server
   */
  const connect = useCallback(() => {
    if (!enabled) {
      return;
    }

    // Clean up existing connection
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    clearReconnectTimeout();
    setIsConnecting(true);
    setConnectionState('connecting');
    setLastError(null);

    try {
      // Get auth token
      const token =
        typeof window !== 'undefined' ? localStorage.getItem('auth_token') : null;

      // Build WebSocket URL with query parameters
      const urlObj = new URL(url);
      if (channelsRef.current.length > 0) {
        urlObj.searchParams.set('channels', channelsRef.current.join(','));
      }
      if (token) {
        urlObj.searchParams.set('token', token);
      }

      const ws = new WebSocket(urlObj.toString());

      ws.onopen = () => {
        setIsConnected(true);
        setIsConnecting(false);
        setConnectionState('connected');
        reconnectAttemptsRef.current = 0;
        isManualDisconnectRef.current = false;

        // Start heartbeat
        startHeartbeat();

        // Subscribe to channels
        if (channelsRef.current.length > 0) {
          ws.send(JSON.stringify({ type: 'subscribe', channels: channelsRef.current }));
        }

        // Send queued messages
        while (messageQueueRef.current.length > 0) {
          const msg = messageQueueRef.current.shift();
          if (msg && ws.readyState === WebSocket.OPEN) {
            try {
              ws.send(JSON.stringify(msg));
            } catch (error) {
              console.warn('Failed to send queued message:', error);
            }
          }
        }
      };

      ws.onmessage = (event) => {
        try {
          // Handle heartbeat responses
          if (event.data === 'pong' || event.data === JSON.stringify({ type: 'pong' })) {
            return;
          }

          const message: WebSocketMessage = JSON.parse(event.data);
          handleMessage(message);
        } catch (error) {
          console.warn('Failed to parse WebSocket message:', error);
          setLastError(
            error instanceof Error ? error : new Error('Failed to parse message'),
          );
        }
      };

      ws.onerror = (error) => {
        // Don't use console.error as it triggers Next.js error overlay
        // WebSocket errors are expected during reconnection attempts
        console.warn('WebSocket connection error - will retry if applicable');
        setIsConnecting(false);
        setConnectionState('error');
        setLastError(new Error('WebSocket connection error'));
      };

      ws.onclose = (event) => {
        setIsConnected(false);
        setIsConnecting(false);
        setConnectionState('disconnected');
        stopHeartbeat();

        // Only attempt reconnection if not manually disconnected and auto-reconnect is enabled
        if (
          !isManualDisconnectRef.current &&
          autoReconnect &&
          reconnectAttemptsRef.current < maxReconnectAttempts
        ) {
          reconnectAttemptsRef.current++;
          const delay = calculateBackoffDelay(
            reconnectAttemptsRef.current - 1,
            reconnectInterval,
          );

          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, delay);
        } else if (reconnectAttemptsRef.current >= maxReconnectAttempts) {
          setLastError(new Error('Max reconnection attempts reached'));
          setConnectionState('error');
        }
      };

      wsRef.current = ws;
    } catch (error) {
      console.warn('Failed to create WebSocket connection - will retry:', error);
      setIsConnecting(false);
      setConnectionState('error');
      setLastError(
        error instanceof Error ? error : new Error('Failed to create connection'),
      );

      // Attempt reconnection on error
      if (autoReconnect && reconnectAttemptsRef.current < maxReconnectAttempts) {
        reconnectAttemptsRef.current++;
        const delay = calculateBackoffDelay(
          reconnectAttemptsRef.current - 1,
          reconnectInterval,
        );
        reconnectTimeoutRef.current = setTimeout(() => {
          connect();
        }, delay);
      }
    }
  }, [
    url,
    enabled,
    autoReconnect,
    reconnectInterval,
    maxReconnectAttempts,
    startHeartbeat,
    stopHeartbeat,
    clearReconnectTimeout,
    handleMessage,
  ]);

  /**
   * Disconnect from WebSocket server
   */
  const disconnect = useCallback(() => {
    isManualDisconnectRef.current = true;
    clearReconnectTimeout();
    stopHeartbeat();

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    setIsConnected(false);
    setIsConnecting(false);
    setConnectionState('disconnected');
  }, [clearReconnectTimeout, stopHeartbeat]);

  /**
   * Manually reconnect
   */
  const reconnect = useCallback(() => {
    disconnect();
    reconnectAttemptsRef.current = 0;
    setTimeout(() => {
      connect();
    }, 100);
  }, [connect, disconnect]);

  /**
   * Send message through WebSocket
   */
  const send = useCallback(
    (message: unknown) => {
      if (wsRef.current?.readyState === WebSocket.OPEN) {
        try {
          wsRef.current.send(JSON.stringify(message));
        } catch (error) {
          console.warn('Failed to send message:', error);
          // Queue message if send fails
          messageQueueRef.current.push(message);
        }
      } else {
        // Queue message for later
        messageQueueRef.current.push(message);
      }
    },
    [],
  );

  /**
   * Subscribe to a channel
   */
  const subscribe = useCallback(
    (channel: string) => {
      if (!channelsRef.current.includes(channel)) {
        channelsRef.current.push(channel);
      }
      send({ type: 'subscribe', channels: [channel] });
    },
    [send],
  );

  /**
   * Unsubscribe from a channel
   */
  const unsubscribe = useCallback(
    (channel: string) => {
      channelsRef.current = channelsRef.current.filter((c) => c !== channel);
      send({ type: 'unsubscribe', channels: [channel] });
    },
    [send],
  );

  // Connect on mount and when enabled changes
  useEffect(() => {
    if (enabled) {
      connect();
    } else {
      disconnect();
    }

    return () => {
      disconnect();
    };
  }, [enabled, connect, disconnect]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      disconnect();
      clearReconnectTimeout();
      stopHeartbeat();
    };
  }, [disconnect, clearReconnectTimeout, stopHeartbeat]);

  return useMemo(
    () => ({
      isConnected,
      isConnecting,
      connectionState,
      messages,
      send,
      subscribe,
      unsubscribe,
      reconnect,
      disconnect,
      lastError,
    }),
    [
      isConnected,
      isConnecting,
      connectionState,
      messages,
      send,
      subscribe,
      unsubscribe,
      reconnect,
      disconnect,
      lastError,
    ],
  );
}
