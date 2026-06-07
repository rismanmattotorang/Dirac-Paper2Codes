"use client"

import { useRouter, type PageId } from "@/components/app-router"
import { IconHome, IconFileText, IconZap, IconCode, IconToolbox, IconBarChart, IconSettings, IconX, IconBrain } from "@/components/ui/icons"
import { useEffect } from "react"

interface SidebarProps {
  open: boolean
  onToggle: (open: boolean) => void
}

export function Sidebar({ open, onToggle }: SidebarProps) {
  const { currentPage, navigate } = useRouter()

  const menuItems = [
    { id: "dashboard" as PageId, label: "Dashboard", icon: IconHome, description: "Overview and metrics" },
    { id: "papers" as PageId, label: "Papers", icon: IconFileText, description: "Manage research papers" },
    { id: "skills" as PageId, label: "Domain Skills", icon: IconBrain, description: "Choose a domain specialisation" },
    { id: "tasks" as PageId, label: "Task Queue", icon: IconZap, description: "Active processing tasks" },
    { id: "code" as PageId, label: "Generated Code", icon: IconCode, description: "View generated code" },
    { id: "tools" as PageId, label: "Tools & Prompts", icon: IconToolbox, description: "Inspect system prompts" },
    { id: "analytics" as PageId, label: "Analytics", icon: IconBarChart, description: "Performance insights" },
    { id: "settings" as PageId, label: "Settings", icon: IconSettings, description: "App configuration" },
  ] as const

  const handleNavigation = (id: PageId) => {
    navigate(id)
    onToggle(false)
  }

  // Close sidebar on escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && open) {
        onToggle(false)
      }
    }
    document.addEventListener("keydown", handleEscape)
    return () => document.removeEventListener("keydown", handleEscape)
  }, [open, onToggle])

  // Prevent body scroll when mobile menu is open
  useEffect(() => {
    if (open) {
      document.body.style.overflow = "hidden"
    } else {
      document.body.style.overflow = ""
    }
    return () => {
      document.body.style.overflow = ""
    }
  }, [open])

  return (
    <>
      {open && (
        <div
          className="fixed inset-0 z-30 bg-black/50 backdrop-blur-sm lg:hidden animate-in fade-in duration-200"
          onClick={() => onToggle(false)}
          aria-hidden="true"
        />
      )}

      <aside
        className={`fixed lg:static inset-y-0 left-0 z-40 w-64 bg-sidebar border-r border-sidebar-border transition-transform duration-300 ease-in-out ${
          open ? "translate-x-0" : "-translate-x-full lg:translate-x-0"
        }`}
        aria-label="Main navigation"
      >
        <div className="h-full flex flex-col overflow-y-auto overscroll-contain">
          {/* Header */}
          <div className="px-6 py-5 border-b border-sidebar-border flex items-center justify-between lg:justify-start shrink-0">
            <div className="flex items-center gap-3 min-w-0">
              <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary to-secondary flex items-center justify-center shadow-sm flex-shrink-0">
                <IconCode className="w-6 h-6 text-primary-foreground" aria-hidden="true" />
              </div>
              <div className="hidden lg:block min-w-0">
                <h2 className="font-bold text-sidebar-foreground text-base leading-tight">P2C</h2>
                <p className="text-xs text-muted-foreground leading-tight">v0.1.0</p>
              </div>
            </div>
            <button
              onClick={() => onToggle(false)}
              className="lg:hidden p-1.5 hover:bg-sidebar-accent active:bg-sidebar-accent/80 rounded-lg transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring focus-visible:ring-offset-2"
              aria-label="Close sidebar"
              type="button"
            >
              <IconX className="w-5 h-5 text-sidebar-foreground" aria-hidden="true" />
            </button>
          </div>

          {/* Navigation */}
          <nav className="flex-1 px-3 py-4 space-y-1" aria-label="Main navigation">
            {menuItems.map((item) => {
              const Icon = item.icon
              const isActive = currentPage === item.id
              return (
                <button
                  key={item.id}
                  onClick={() => handleNavigation(item.id)}
                  className={`group relative w-full flex items-center gap-3 px-3 py-2.5 rounded-lg font-medium text-sm transition-all duration-200 ${
                    isActive
                      ? "bg-sidebar-accent text-sidebar-accent-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-sidebar-accent/50 hover:text-sidebar-foreground"
                  } focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring focus-visible:ring-offset-2`}
                  aria-current={isActive ? "page" : undefined}
                  aria-label={`${item.label}${item.description ? `, ${item.description}` : ""}`}
                  type="button"
                >
                  <Icon
                    className={`w-5 h-5 flex-shrink-0 transition-transform duration-200 ${
                      isActive ? "scale-110" : "group-hover:scale-105"
                    }`}
                    aria-hidden="true"
                  />
                  <span className="flex-1 text-left">{item.label}</span>
                  {isActive && (
                    <div className="absolute right-2 w-1.5 h-1.5 rounded-full bg-sidebar-primary" aria-hidden="true" />
                  )}
                </button>
              )
            })}
          </nav>

          {/* Footer */}
          <div className="p-4 border-t border-sidebar-border space-y-3 shrink-0">
            <div className="px-4 py-3 rounded-lg bg-sidebar-accent/80 border border-sidebar-border/50">
              <p className="text-xs font-semibold text-sidebar-accent-foreground mb-0.5">Processing</p>
              <p className="text-xs text-sidebar-accent-foreground/80">2 papers queued</p>
            </div>
            <button
              className="w-full btn-secondary text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
              aria-label="View documentation"
              type="button"
            >
              View Docs
            </button>
          </div>
        </div>
      </aside>
    </>
  )
}
