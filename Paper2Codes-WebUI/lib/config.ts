/**
 * Environment configuration for Paper2Codes WebUI
 */

type Maybe<T> = T | undefined | null;

const parseNumber = (value: Maybe<string>, fallback: number): number => {
  if (!value) return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
};

const parseBoolean = (value: Maybe<string>, fallback: boolean): boolean => {
  if (value === undefined || value === null) return fallback;
  const normalized = value.toString().toLowerCase();
  if (['true', '1', 'yes', 'on'].includes(normalized)) return true;
  if (['false', '0', 'no', 'off'].includes(normalized)) return false;
  return fallback;
};

// Get API URL - use 127.0.0.1 for consistency with backend
const getApiBaseUrl = () => {
  // Check environment variable first
  if (process.env.NEXT_PUBLIC_API_URL) {
    return process.env.NEXT_PUBLIC_API_URL;
  }

  // Default to 127.0.0.1:8080 for consistency with backend
  if (typeof window !== 'undefined') {
    return 'http://127.0.0.1:8080';
  }
  // Server-side, use the same 127.0.0.1 host
  return 'http://127.0.0.1:8080';
};

const getWsUrl = () => {
  // Check environment variable first
  if (process.env.NEXT_PUBLIC_WS_URL) {
    return process.env.NEXT_PUBLIC_WS_URL;
  }

  // Construct from API URL
  const apiUrl = getApiBaseUrl();
  // Convert http:// to ws:// and https:// to wss://
  return apiUrl.replace(/^http/, 'ws') + '/ws';
};

const API_TIMEOUT_DEFAULT = 30_000;
const WS_RECONNECT_INTERVAL_DEFAULT = 5_000;
const WS_MAX_RECONNECT_ATTEMPTS_DEFAULT = 10;
const WS_HEARTBEAT_INTERVAL_DEFAULT = 30_000;

export const config = {
  api: {
    baseUrl: getApiBaseUrl(),
    wsUrl: getWsUrl(),
    timeoutMs: parseNumber(process.env.NEXT_PUBLIC_API_TIMEOUT, API_TIMEOUT_DEFAULT),
  },
  features: {
    analytics: parseBoolean(process.env.NEXT_PUBLIC_ENABLE_ANALYTICS, true),
    realTime: parseBoolean(process.env.NEXT_PUBLIC_ENABLE_REAL_TIME, true),
  },
  realtime: {
    autoReconnect: parseBoolean(process.env.NEXT_PUBLIC_WS_RECONNECT_ENABLED, true),
    reconnectIntervalMs: parseNumber(
      process.env.NEXT_PUBLIC_WS_RECONNECT_INTERVAL,
      WS_RECONNECT_INTERVAL_DEFAULT,
    ),
    maxReconnectAttempts: parseNumber(
      process.env.NEXT_PUBLIC_WS_MAX_RECONNECT_ATTEMPTS,
      WS_MAX_RECONNECT_ATTEMPTS_DEFAULT,
    ),
    heartbeatIntervalMs: parseNumber(
      process.env.NEXT_PUBLIC_WS_HEARTBEAT_INTERVAL,
      WS_HEARTBEAT_INTERVAL_DEFAULT,
    ),
  },
  polling: {
    enabled: parseBoolean(process.env.NEXT_PUBLIC_POLLING_ENABLED, true),
    intervalMs: parseNumber(process.env.NEXT_PUBLIC_POLLING_INTERVAL, 5_000),
  },
  env: process.env.NEXT_PUBLIC_ENV || 'development',
} as const;

export const API_BASE_URL = config.api.baseUrl;
export const API_TIMEOUT_MS = config.api.timeoutMs;
export const WS_BASE_URL = config.api.wsUrl;
