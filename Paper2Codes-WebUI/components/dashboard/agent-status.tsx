"use client"

import { IconBrain, IconDatabase, IconGitBranch, IconShield, IconAlertCircle, IconLoader } from "@/components/ui/icons"
import { useAgentStatus } from "@/lib/hooks/use-dashboard"

const agentIcons = {
  "Planning": IconBrain,
  "Analysis": IconDatabase,
  "Coding": IconGitBranch,
  "Verification": IconShield,
}

export function AgentStatus() {
  const { agents: agentData, loading, error } = useAgentStatus()

  const agents = agentData.map(agent => ({
    ...agent,
    icon: agentIcons[agent.name as keyof typeof agentIcons] || IconBrain,
  }))

  return (
    <div className="rounded-xl border border-border bg-card shadow-sm hover:shadow-md transition-shadow duration-300">
      <div className="p-6 lg:p-8">
        <div className="mb-6">
          <h3 className="text-xl font-bold tracking-tight text-foreground mb-1">Agent Status</h3>
          <p className="text-xs text-muted-foreground">System agent health monitoring</p>
        </div>

        {loading && (
          <div className="flex items-center justify-center py-8">
            <IconLoader className="w-6 h-6 text-muted-foreground animate-spin" />
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-amber-200 dark:border-amber-800 bg-amber-50 dark:bg-amber-950/20 p-4">
            <div className="flex items-center gap-2">
              <IconAlertCircle className="w-4 h-4 text-amber-600 dark:text-amber-400" />
              <div>
                <p className="text-sm font-semibold text-amber-900 dark:text-amber-100">Failed to load agent status</p>
                {error.includes('Cannot connect') && (
                  <p className="text-xs text-amber-700 dark:text-amber-300 mt-1">
                    Please ensure the backend is running on http://127.0.0.1:8080
                  </p>
                )}
                {!error.includes('Cannot connect') && (
                  <p className="text-xs text-amber-700 dark:text-amber-300 mt-1">{error}</p>
                )}
              </div>
            </div>
          </div>
        )}

        {!loading && !error && <div className="space-y-3">
          {agents.map((agent, idx) => {
            const Icon = agent.icon
            const isActive = agent.status === "running"

            return (
              <div
                key={idx}
                role="status"
                aria-live="polite"
                aria-label={`${agent.name} agent is ${isActive ? "active" : "idle"}`}
                className={`group relative overflow-hidden rounded-xl border transition-all duration-300 ${
                  isActive
                    ? "border-green-200/50 dark:border-green-800/50 bg-gradient-to-br from-green-50/30 to-transparent dark:from-green-950/20"
                    : "border-border bg-gradient-to-br from-muted/30 to-transparent"
                } hover:shadow-md hover:scale-[1.02] focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2`}
              >
                <div className="p-4">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3 flex-1 min-w-0">
                      <div
                        className={`w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0 transition-all duration-300 ${
                          isActive
                            ? "bg-green-500/10 text-green-600 dark:text-green-400 group-hover:scale-110"
                            : "bg-muted text-muted-foreground"
                        }`}
                      >
                        <Icon className="w-5 h-5" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <p
                          className={`text-sm font-semibold truncate ${
                            isActive ? "text-foreground" : "text-muted-foreground"
                          }`}
                        >
                          {agent.name}
                        </p>
                        <p className="text-xs text-muted-foreground truncate">{agent.description}</p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2 flex-shrink-0">
                      <div className="relative">
                        <div
                          className={`w-2.5 h-2.5 rounded-full transition-all duration-300 ${
                            isActive
                              ? "bg-green-500 shadow-lg shadow-green-500/50"
                              : "bg-muted-foreground/40"
                          }`}
                        />
                        {isActive && (
                          <div className="absolute inset-0 w-2.5 h-2.5 rounded-full bg-green-500 animate-ping opacity-75" />
                        )}
                      </div>
                      <span
                        className={`text-xs font-medium px-2 py-0.5 rounded-md ${
                          isActive
                            ? "text-green-700 dark:text-green-300 bg-green-500/10"
                            : "text-muted-foreground bg-muted"
                        }`}
                      >
                        {isActive ? "Active" : "Idle"}
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            )
          })}
        </div>}
      </div>
    </div>
  )
}
