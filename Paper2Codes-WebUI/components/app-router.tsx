"use client"

import type React from "react"

import { createContext, useContext, useState } from "react"
import { Dashboard } from "./dashboard/dashboard"
import { PapersPage } from "./papers/papers-page"
import { AnalyticsPage } from "./analytics/analytics-page"
import { SettingsPage } from "./settings/settings-page"
import { CodeGenerationPage } from "./code/code-generation-page"
import { TasksPage } from "./tasks/tasks-page"
import { ToolsPage } from "./tools/tools-page"

export type PageId = "dashboard" | "papers" | "tasks" | "code" | "tools" | "analytics" | "settings"

interface RouterContextType {
  currentPage: PageId
  navigate: (page: PageId) => void
}

const RouterContext = createContext<RouterContextType | undefined>(undefined)

export function AppRouter({ children }: { children: React.ReactNode }) {
  const [currentPage, setCurrentPage] = useState<PageId>("dashboard")

  const navigate = (page: PageId) => {
    setCurrentPage(page)
  }

  return <RouterContext.Provider value={{ currentPage, navigate }}>{children}</RouterContext.Provider>
}

export function useRouter() {
  const context = useContext(RouterContext)
  if (!context) {
    throw new Error("useRouter must be used within AppRouter")
  }
  return context
}

export function PageRenderer({ page }: { page: PageId }) {
  return (
    <div className="transition-opacity duration-200 animate-in fade-in">
      {(() => {
        switch (page) {
          case "dashboard":
            return <Dashboard />
          case "papers":
            return <PapersPage />
          case "tasks":
            return <TasksPage />
          case "code":
            return <CodeGenerationPage />
          case "analytics":
            return <AnalyticsPage />
          case "tools":
            return <ToolsPage />
          case "settings":
            return <SettingsPage />
          default:
            return <Dashboard />
        }
      })()}
    </div>
  )
}
