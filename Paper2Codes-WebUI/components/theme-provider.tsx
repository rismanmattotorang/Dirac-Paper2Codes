"use client"

import * as React from "react"
import { useEffect, useState } from "react"

type Theme = "light" | "dark" | "system"

interface ThemeContextType {
  theme: Theme | undefined
  setTheme: (theme: Theme) => void
  toggleTheme: () => void
}

const ThemeContext = React.createContext<ThemeContextType | undefined>(undefined)

export function ThemeProvider({
  children,
  defaultTheme = "light",
  enableSystem = true,
}: {
  children: React.ReactNode
  attribute?: string
  defaultTheme?: Theme
  enableSystem?: boolean
}) {
  const [theme, setThemeState] = useState<Theme | undefined>(undefined)
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    if (typeof window === "undefined") {
      return
    }

    setMounted(true)

    // Get initial theme
    const stored = localStorage.getItem("theme") as Theme | null
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches
    const resolvedDefault: Theme = defaultTheme === "system" ? "system" : defaultTheme
    const initial = stored ?? resolvedDefault

    const normalizedInitial =
      initial === "system" && !enableSystem ? (prefersDark ? "dark" : "light") : initial

    setThemeState(normalizedInitial)
    updateTheme(normalizedInitial)
  }, [defaultTheme, enableSystem])

  const getResolvedTheme = (value: Theme): "light" | "dark" => {
    if (value === "system") {
      if (enableSystem && typeof window !== "undefined") {
        return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
      }
      return "light"
    }
    return value
  }

  const updateTheme = (intent: Theme) => {
    const root = document.documentElement
    const resolved = getResolvedTheme(intent)

    if (resolved === "dark") {
      root.classList.add("dark")
    } else {
      root.classList.remove("dark")
    }
    localStorage.setItem("theme", intent)
  }

  const setTheme = (newTheme: Theme) => {
    setThemeState(newTheme)
    updateTheme(newTheme)
  }

  const toggleTheme = () => {
    const current = theme ?? defaultTheme
    const resolved = getResolvedTheme(current)
    const nextTheme = resolved === "dark" ? "light" : "dark"
    setTheme(nextTheme)
  }

  useEffect(() => {
    if (typeof window === "undefined") {
      return
    }

    if (!enableSystem) {
      return
    }

    const media = window.matchMedia("(prefers-color-scheme: dark)")
    const handleChange = () => {
      setThemeState((current) => {
        if (current === "system") {
          updateTheme("system")
        }
        return current
      })
    }

    media.addEventListener("change", handleChange)
    return () => {
      media.removeEventListener("change", handleChange)
    }
  }, [enableSystem])

  if (!mounted) {
    return <>{children}</>
  }

  return <ThemeContext.Provider value={{ theme, setTheme, toggleTheme }}>{children}</ThemeContext.Provider>
}

export function useTheme() {
  const context = React.useContext(ThemeContext)
  if (context === undefined) {
    // Return a safe default during SSR or when outside provider
    return {
      theme: undefined as Theme | undefined,
      setTheme: () => {},
      toggleTheme: () => {},
    }
  }
  return context
}
