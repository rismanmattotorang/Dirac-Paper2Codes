/**
 * Authentication: token storage, auth API calls, and the refresh-on-401 helper.
 *
 * These functions deliberately use the raw `fetch` API (not `ApiClient`) so the
 * refresh path cannot recurse through the client's own 401 interceptor.
 */

import { API_BASE_URL } from '@/lib/config';

export const ACCESS_TOKEN_KEY = 'auth_token';
export const REFRESH_TOKEN_KEY = 'refresh_token';
export const USER_KEY = 'auth_user';

export interface AuthUser {
  id: string;
  email: string;
  username: string;
  roles: string[];
}

interface LoginPayload {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: AuthUser;
}

const baseURL = API_BASE_URL.replace(/\/$/, '');

const hasWindow = () => typeof window !== 'undefined';

export function getAccessToken(): string {
  return hasWindow() ? localStorage.getItem(ACCESS_TOKEN_KEY) || '' : '';
}

export function getRefreshToken(): string {
  return hasWindow() ? localStorage.getItem(REFRESH_TOKEN_KEY) || '' : '';
}

export function getStoredUser(): AuthUser | null {
  if (!hasWindow()) return null;
  const raw = localStorage.getItem(USER_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as AuthUser;
  } catch {
    return null;
  }
}

/** Emit a DOM event so React state (AuthProvider) can react to token changes. */
function emitAuthChange() {
  if (hasWindow()) {
    window.dispatchEvent(new CustomEvent('auth:changed'));
  }
}

function persistSession(payload: LoginPayload) {
  if (!hasWindow()) return;
  localStorage.setItem(ACCESS_TOKEN_KEY, payload.access_token);
  localStorage.setItem(REFRESH_TOKEN_KEY, payload.refresh_token);
  localStorage.setItem(USER_KEY, JSON.stringify(payload.user));
  emitAuthChange();
}

export function clearSession() {
  if (!hasWindow()) return;
  localStorage.removeItem(ACCESS_TOKEN_KEY);
  localStorage.removeItem(REFRESH_TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
  emitAuthChange();
}

/** Unwrap the backend's `{ success, data, error }` envelope. */
async function unwrap<T>(response: Response): Promise<T> {
  const text = await response.text();
  const body = text ? JSON.parse(text) : {};
  if (!response.ok || body?.success === false) {
    const message =
      body?.error?.message || body?.message || response.statusText || 'Request failed';
    throw new Error(message);
  }
  // Handlers wrap the result in `data`.
  return (body?.data ?? body) as T;
}

export async function login(email: string, password: string): Promise<AuthUser> {
  const response = await fetch(`${baseURL}/api/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, password }),
  });
  const payload = await unwrap<LoginPayload>(response);
  persistSession(payload);
  return payload.user;
}

export async function register(
  email: string,
  username: string,
  password: string,
): Promise<AuthUser> {
  const response = await fetch(`${baseURL}/api/auth/register`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email, username, password }),
  });
  const payload = await unwrap<LoginPayload>(response);
  persistSession(payload);
  return payload.user;
}

export async function fetchCurrentUser(): Promise<AuthUser> {
  const token = getAccessToken();
  const response = await fetch(`${baseURL}/api/auth/me`, {
    headers: token ? { Authorization: `Bearer ${token}` } : {},
  });
  return unwrap<AuthUser>(response);
}

export async function logout(): Promise<void> {
  const refresh_token = getRefreshToken();
  if (refresh_token) {
    try {
      await fetch(`${baseURL}/api/auth/logout`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refresh_token }),
      });
    } catch {
      // Best-effort: clear locally regardless of network outcome.
    }
  }
  clearSession();
}

// Dedupe concurrent refreshes: many in-flight requests may 401 at once, but we
// only want a single refresh round-trip; they all await the same promise.
let refreshPromise: Promise<boolean> | null = null;

async function performRefresh(): Promise<boolean> {
  const refresh_token = getRefreshToken();
  if (!refresh_token) return false;
  try {
    const response = await fetch(`${baseURL}/api/auth/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token }),
    });
    if (!response.ok) {
      clearSession();
      return false;
    }
    const payload = await unwrap<LoginPayload>(response);
    persistSession(payload);
    return true;
  } catch {
    clearSession();
    return false;
  }
}

/**
 * Attempt to refresh the access token, coalescing concurrent callers. Returns
 * `true` if a fresh access token is now stored.
 */
export function attemptRefresh(): Promise<boolean> {
  if (!refreshPromise) {
    refreshPromise = performRefresh().finally(() => {
      refreshPromise = null;
    });
  }
  return refreshPromise;
}

/** True for the auth endpoints themselves, which must never trigger a refresh. */
export function isAuthEndpoint(endpoint: string): boolean {
  return endpoint.includes('/api/auth/');
}
