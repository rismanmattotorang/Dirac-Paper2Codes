"use client"

import { useTheme } from "@/components/theme-provider"
import { IconMenu, IconBell, IconSun, IconMoon, IconSettings } from "@/components/ui/icons"
import { useState } from "react"
import { useAuth } from "@/lib/auth/auth-context"
import { LoginDialog } from "@/components/auth/login-dialog"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"

interface HeaderProps {
  onSidebarToggle: () => void
}

function initialsFor(name: string): string {
  const parts = name.trim().split(/[\s@._-]+/).filter(Boolean)
  if (parts.length === 0) return "?"
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase()
  return (parts[0][0] + parts[1][0]).toUpperCase()
}

export function Header({ onSidebarToggle }: HeaderProps) {
  const { theme, toggleTheme } = useTheme()
  const [notificationCount] = useState(3)
  const { status, user, logout } = useAuth()
  const [loginOpen, setLoginOpen] = useState(false)

  return (
    <header
      className="sticky top-0 z-50 border-b border-border/40 bg-background/95 backdrop-blur-xl supports-[backdrop-filter]:bg-background/80 shadow-sm"
      role="banner"
    >
      <div className="px-4 sm:px-6 lg:px-8 h-14 flex items-center justify-between">
        <div className="flex items-center gap-3 min-w-0">
          <button
            onClick={onSidebarToggle}
            className="lg:hidden p-2 -ml-2 hover:bg-muted active:bg-muted/80 rounded-lg transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
            aria-label="Toggle sidebar"
            aria-expanded="false"
            type="button"
          >
            <IconMenu className="w-5 h-5 text-foreground" aria-hidden="true" />
          </button>
          <div className="flex items-center gap-2.5 min-w-0">
            <div
              className="w-2.5 h-2.5 rounded-full bg-gradient-to-r from-primary to-secondary shadow-sm shadow-primary/50 flex-shrink-0"
              aria-hidden="true"
            />
            <h1 className="text-lg font-bold text-foreground tracking-tight truncate">Paper2Codes</h1>
          </div>
        </div>

        <nav className="flex items-center gap-1 sm:gap-2" aria-label="Header actions">
          <button
            className="relative p-2.5 hover:bg-muted active:bg-muted/80 rounded-lg transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 group"
            aria-label={`Notifications${notificationCount > 0 ? `, ${notificationCount} unread` : ""}`}
            type="button"
          >
            <IconBell className="w-5 h-5 text-muted-foreground group-hover:text-foreground transition-colors" aria-hidden="true" />
            {notificationCount > 0 && (
              <>
                <span className="absolute top-1.5 right-1.5 w-2 h-2 bg-destructive rounded-full ring-2 ring-background" />
                <span className="sr-only">{notificationCount} unread notifications</span>
              </>
            )}
          </button>

          <button
            onClick={toggleTheme}
            className="p-2.5 hover:bg-muted active:bg-muted/80 rounded-lg transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 group"
            aria-label={`Switch to ${theme === "light" ? "dark" : "light"} mode`}
            type="button"
          >
            {theme === "light" ? (
              <IconMoon className="w-5 h-5 text-muted-foreground group-hover:text-foreground transition-colors" aria-hidden="true" />
            ) : (
              <IconSun className="w-5 h-5 text-muted-foreground group-hover:text-foreground transition-colors" aria-hidden="true" />
            )}
          </button>

          <button
            className="p-2.5 hover:bg-muted active:bg-muted/80 rounded-lg transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 group"
            aria-label="Open settings"
            type="button"
          >
            <IconSettings className="w-5 h-5 text-muted-foreground group-hover:text-foreground transition-colors" aria-hidden="true" />
          </button>

          {status === "authenticated" && user ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button
                  className="ml-1 w-9 h-9 rounded-full bg-gradient-to-br from-primary to-secondary flex items-center justify-center shadow-sm hover:shadow-md transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 active:scale-95"
                  aria-label="User account menu"
                  type="button"
                  data-testid="user-menu-trigger"
                >
                  <span className="text-primary-foreground text-xs font-bold">
                    {initialsFor(user.username || user.email)}
                  </span>
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-56">
                <DropdownMenuLabel>
                  <div className="flex flex-col">
                    <span className="truncate font-medium">{user.username}</span>
                    <span className="truncate text-xs text-muted-foreground">{user.email}</span>
                  </div>
                </DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={() => void logout()} data-testid="logout-button">
                  Sign out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          ) : (
            <button
              onClick={() => setLoginOpen(true)}
              className="ml-1 inline-flex items-center px-3 h-9 rounded-lg text-sm font-medium bg-primary text-primary-foreground shadow-sm hover:shadow-md transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 active:scale-95"
              type="button"
              aria-label="Sign in"
              data-testid="sign-in-button"
            >
              Sign in
            </button>
          )}
        </nav>
      </div>
      <LoginDialog open={loginOpen} onOpenChange={setLoginOpen} />
    </header>
  )
}
