"use client"

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react"
import type { ReactNode } from "react"
import {
  type AuthUser,
  fetchCurrentUser,
  getAccessToken,
  getStoredUser,
  login as apiLogin,
  logout as apiLogout,
  register as apiRegister,
} from "@/lib/api/auth"

type AuthStatus = "loading" | "authenticated" | "anonymous"

interface AuthContextValue {
  status: AuthStatus
  user: AuthUser | null
  login: (email: string, password: string) => Promise<void>
  register: (email: string, username: string, password: string) => Promise<void>
  logout: () => Promise<void>
}

const AuthContext = createContext<AuthContextValue | undefined>(undefined)

/**
 * Provides auth state to the app. A stored access token is validated against
 * `/api/auth/me` on mount; if none exists the app runs in an "anonymous" state
 * (the dashboard stays usable, and a Sign-in affordance is shown). This keeps
 * the login flow available without forcing a hard gate on deployments where the
 * API does not require authentication.
 */
export function AuthProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<AuthStatus>("loading")
  const [user, setUser] = useState<AuthUser | null>(null)

  const refreshFromStorage = useCallback(async () => {
    if (!getAccessToken()) {
      setUser(null)
      setStatus("anonymous")
      return
    }
    // Optimistically show the cached user, then validate the token.
    setUser(getStoredUser())
    try {
      const current = await fetchCurrentUser()
      setUser(current)
      setStatus("authenticated")
    } catch {
      setUser(null)
      setStatus("anonymous")
    }
  }, [])

  useEffect(() => {
    void refreshFromStorage()
    // React to token changes from anywhere (login, logout, refresh failure).
    const handler = () => void refreshFromStorage()
    window.addEventListener("auth:changed", handler)
    return () => window.removeEventListener("auth:changed", handler)
  }, [refreshFromStorage])

  const login = useCallback(async (email: string, password: string) => {
    const u = await apiLogin(email, password)
    setUser(u)
    setStatus("authenticated")
  }, [])

  const register = useCallback(
    async (email: string, username: string, password: string) => {
      const u = await apiRegister(email, username, password)
      setUser(u)
      setStatus("authenticated")
    },
    [],
  )

  const logout = useCallback(async () => {
    await apiLogout()
    setUser(null)
    setStatus("anonymous")
  }, [])

  const value = useMemo<AuthContextValue>(
    () => ({ status, user, login, register, logout }),
    [status, user, login, register, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth() {
  const ctx = useContext(AuthContext)
  if (!ctx) {
    throw new Error("useAuth must be used within an AuthProvider")
  }
  return ctx
}
