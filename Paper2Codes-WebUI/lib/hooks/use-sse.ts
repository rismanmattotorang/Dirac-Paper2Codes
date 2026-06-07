/**
 * React hook for Server-Sent Events (SSE) streaming
 * 
 * Provides a hook for subscribing to SSE streams with automatic reconnection
 * and event handling
 */

'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import { API_BASE_URL } from '@/lib/config';

export interface UseSSEOptions {
  url: string;
  enabled?: boolean;
  onMessage?: (event: MessageEvent) => void;
  onError?: (error: Event) => void;
  onOpen?: () => void;
  onClose?: () => void;
  reconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
}

export interface UseSSEReturn {
  isConnected: boolean;
  isConnecting: boolean;
  error: Error | null;
  connect: () => void;
  disconnect: () => void;
  reconnectAttempts: number;
}

/**
 * Hook to connect to Server-Sent Events (SSE) stream
 * 
 * @example
 * ```tsx
 * const { isConnected, error } = useSSE({
 *   url: '/api/papers/123/generate/stream',
 *   onMessage: (event) => {
 *     const data = JSON.parse(event.data);
 *     console.log('Received:', data);
 *   },
 * });
 * ```
 */
export function useSSE(options: UseSSEOptions): UseSSEReturn {
  const {
    url,
    enabled = true,
    onMessage,
    onError,
    onOpen,
    onClose,
    reconnect = true,
    reconnectInterval = 3000,
    maxReconnectAttempts = 10,
  } = options;

  const [isConnected, setIsConnected] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [reconnectAttempts, setReconnectAttempts] = useState(0);

  const eventSourceRef = useRef<EventSource | null>(null);
  const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const shouldReconnectRef = useRef(true);

  const getAuthToken = useCallback(() => {
    // Get auth token from storage (localStorage, cookie, etc.)
    if (typeof window !== 'undefined') {
      return localStorage.getItem('auth_token') || '';
    }
    return '';
  }, []);

  const buildUrl = useCallback(() => {
    const baseUrl = API_BASE_URL.replace(/\/$/, '');
    const fullUrl = url.startsWith('http') ? url : `${baseUrl}${url}`;
    
    // Add auth token as query parameter if needed
    const token = getAuthToken();
    if (token) {
      const urlObj = new URL(fullUrl);
      urlObj.searchParams.append('token', token);
      return urlObj.toString();
    }
    
    return fullUrl;
  }, [url, getAuthToken]);

  const connect = useCallback(() => {
    if (!enabled || isConnecting || (eventSourceRef.current?.readyState === EventSource.OPEN)) {
      return;
    }

    setIsConnecting(true);
    setError(null);

    try {
      const fullUrl = buildUrl();
      const eventSource = new EventSource(fullUrl);
      
      eventSource.onopen = () => {
        setIsConnected(true);
        setIsConnecting(false);
        setError(null);
        setReconnectAttempts(0);
        onOpen?.();
      };

      eventSource.onmessage = (event: MessageEvent) => {
        onMessage?.(event);
      };

      eventSource.onerror = (event: Event) => {
        setIsConnected(false);
        setIsConnecting(false);
        
        const errorEvent = new Error('SSE connection error');
        setError(errorEvent);
        onError?.(event);

        // Close current connection
        eventSource.close();
        eventSourceRef.current = null;

        // Attempt reconnect if enabled and not exceeded max attempts
        if (
          shouldReconnectRef.current &&
          reconnect &&
          reconnectAttempts < maxReconnectAttempts
        ) {
          const attempts = reconnectAttempts + 1;
          setReconnectAttempts(attempts);
          
          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, reconnectInterval * attempts); // Exponential backoff
        } else if (reconnectAttempts >= maxReconnectAttempts) {
          setError(new Error(`Max reconnect attempts (${maxReconnectAttempts}) exceeded`));
        }
      };

      eventSourceRef.current = eventSource;
    } catch (err) {
      setIsConnecting(false);
      setError(err instanceof Error ? err : new Error('Failed to create SSE connection'));
    }
  }, [
    enabled,
    isConnecting,
    url,
    buildUrl,
    onMessage,
    onError,
    onOpen,
    reconnect,
    reconnectInterval,
    reconnectAttempts,
    maxReconnectAttempts,
    getAuthToken,
  ]);

  const disconnect = useCallback(() => {
    shouldReconnectRef.current = false;
    
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }

    setIsConnected(false);
    setIsConnecting(false);
    onClose?.();
  }, [onClose]);

  // Auto-connect when enabled
  useEffect(() => {
    if (enabled) {
      shouldReconnectRef.current = true;
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
    };
  }, [disconnect]);

  return {
    isConnected,
    isConnecting,
    error,
    connect,
    disconnect,
    reconnectAttempts,
  };
}

/**
 * Hook for streaming generation progress
 */
export function useGenerationProgress(paperId: string | null) {
  const [progress, setProgress] = useState(0);
  const [status, setStatus] = useState<string>('pending');
  const [message, setMessage] = useState<string | null>(null);
  const [data, setData] = useState<Record<string, unknown> | null>(null);

  const { isConnected, error } = useSSE({
    url: paperId ? `/api/papers/${paperId}/generate/stream` : '',
    enabled: !!paperId,
    onMessage: (event) => {
      try {
        const parsed = JSON.parse(event.data);
        if (typeof parsed.progress === 'number') {
          setProgress(parsed.progress);
        }
        if (parsed.status) {
          setStatus(parsed.status);
        }
        if (parsed.message) {
          setMessage(parsed.message);
        }
        setData(parsed);
      } catch (err) {
        console.error('Failed to parse SSE message:', err);
      }
    },
  });

  return {
    progress,
    status,
    message,
    data,
    isConnected,
    error,
  };
}

/**
 * Hook for streaming task logs
 */
export function useTaskLogs(taskId: string | null) {
  const [logs, setLogs] = useState<Array<{ level: string; message: string; timestamp: string }>>([]);

  const { isConnected, error } = useSSE({
    url: taskId ? `/api/tasks/${taskId}/logs/stream` : '',
    enabled: !!taskId,
    onMessage: (event) => {
      try {
        const parsed = JSON.parse(event.data);
        setLogs((prev) => [...prev, {
          level: parsed.level || 'info',
          message: parsed.message || '',
          timestamp: parsed.timestamp || new Date().toISOString(),
        }]);
      } catch (err) {
        console.error('Failed to parse SSE log message:', err);
      }
    },
  });

  return {
    logs,
    isConnected,
    error,
  };
}

/**
 * Hook for streaming LLM responses
 */
export function useLLMStream(prompt: string | null, model?: string) {
  const [tokens, setTokens] = useState<string[]>([]);
  const [isComplete, setIsComplete] = useState(false);

  const { isConnected, error, disconnect } = useSSE({
    url: prompt ? `/api/llm/stream?prompt=${encodeURIComponent(prompt)}${model ? `&model=${encodeURIComponent(model)}` : ''}` : '',
    enabled: !!prompt,
    onMessage: (event) => {
      try {
        const parsed = JSON.parse(event.data);
        if (parsed.type === 'token') {
          setTokens((prev) => [...prev, parsed.content || '']);
        } else if (parsed.type === 'done') {
          setIsComplete(true);
          disconnect();
        }
      } catch (err) {
        console.error('Failed to parse SSE LLM message:', err);
      }
    },
  });

  const reset = useCallback(() => {
    setTokens([]);
    setIsComplete(false);
  }, []);

  return {
    tokens,
    content: tokens.join(''),
    isComplete,
    isConnected,
    error,
    reset,
  };
}

