"use client"

import { IconActivity, IconCheckCircle, IconAlertCircle, IconClock, IconChevronDown, IconLoader } from "@/components/ui/icons"
import { useState } from "react"
import { useActiveTasks } from "@/lib/hooks/use-dashboard"
import type { Task } from "@/lib/api/types"

type TaskStatus = "running" | "queued" | "completed" | "error"

interface DisplayTask {
  id: string
  title: string
  status: TaskStatus
  progress?: number
  startTime: string
  tokens: number
  model: string
  completionTime?: string
}

export function TaskMonitor() {
  const { tasks: apiTasks, loading, error } = useActiveTasks()
  const [expandedTask, setExpandedTask] = useState<string | null>(null)

  const mapStatus = (status: Task["status"]): TaskStatus => {
    if (typeof status === "string") {
      const normalized = status.toLowerCase()
      if (normalized.includes("failed")) {
        return "error"
      }
      if (normalized.includes("completed")) {
        return "completed"
      }
      if (normalized.includes("inprogress") || normalized.includes("in_progress") || normalized.includes("progress")) {
        return "running"
      }
      if (normalized.includes("pending") || normalized.includes("created")) {
        return "queued"
      }
    } else if (status && typeof status === "object" && "Failed" in status) {
      return "error"
    }

    return "queued"
  }

  const describeTaskType = (task: Task): string => {
    const { task_type: type } = task

    if (typeof type === "string") {
      return type
    }

    if ("Analysis" in type) {
      return `Analysis (${type.Analysis.module_id})`
    }

    if ("Coding" in type) {
      return `Coding (${type.Coding.module_id})`
    }

    if ("Verification" in type) {
      return "Verification"
    }

    if ("Fix" in type) {
      return `Fix (${type.Fix.module_id})`
    }

    return "Planning"
  }

  // Convert API tasks to display format
  const tasks: DisplayTask[] = apiTasks.map((task: Task) => {
    const displayStatus = mapStatus(task.status)
    const startTime = task.created_at ? new Date(task.created_at).toLocaleTimeString() : "Pending"

    return {
      id: task.id,
      title: `${describeTaskType(task)}${task.description ? `: ${task.description}` : ""}`,
      status: displayStatus,
      progress: displayStatus === 'running' ? 45 : undefined,
      startTime,
      tokens: 0, // TODO: Get from task metadata
      model: 'GPT-4 Turbo', // TODO: Get from task metadata
    }
  })

  const statusConfig = {
    running: {
      icon: IconActivity,
      color: "text-blue-600 dark:text-blue-400",
      bgColor: "bg-blue-500/10",
      borderColor: "border-blue-200/50 dark:border-blue-800/50",
      label: "Running",
      pulse: true,
    },
    queued: {
      icon: IconClock,
      color: "text-amber-600 dark:text-amber-400",
      bgColor: "bg-amber-500/10",
      borderColor: "border-amber-200/50 dark:border-amber-800/50",
      label: "Queued",
      pulse: false,
    },
    completed: {
      icon: IconCheckCircle,
      color: "text-green-600 dark:text-green-400",
      bgColor: "bg-green-500/10",
      borderColor: "border-green-200/50 dark:border-green-800/50",
      label: "Completed",
      pulse: false,
    },
    error: {
      icon: IconAlertCircle,
      color: "text-red-600 dark:text-red-400",
      bgColor: "bg-red-500/10",
      borderColor: "border-red-200/50 dark:border-red-800/50",
      label: "Error",
      pulse: false,
    },
  }

  return (
    <div className="rounded-xl border border-border bg-card shadow-sm hover:shadow-md transition-shadow duration-300">
      <div className="p-6 lg:p-8">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center">
              <IconActivity className="w-5 h-5 text-primary" />
            </div>
            <div>
              <h2 className="text-xl font-bold tracking-tight text-foreground">Active Tasks</h2>
              <p className="text-xs text-muted-foreground mt-0.5">Real-time task monitoring</p>
            </div>
          </div>
        </div>

        {loading && (
          <div className="flex items-center justify-center py-12">
            <IconLoader className="w-8 h-8 text-muted-foreground animate-spin" />
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/20 p-4">
            <div className="flex items-center gap-2">
              <IconAlertCircle className="w-4 h-4 text-red-600 dark:text-red-400" />
              <p className="text-sm text-red-900 dark:text-red-100">Failed to load tasks</p>
            </div>
          </div>
        )}

        {!loading && !error && tasks.length === 0 && (
          <div className="text-center py-12">
            <IconActivity className="w-12 h-12 text-muted-foreground/50 mx-auto mb-3" />
            <p className="text-sm text-muted-foreground">No active tasks</p>
          </div>
        )}

        {!loading && !error && tasks.length > 0 && <div className="space-y-3">
          {tasks.map((task) => {
            const isExpanded = expandedTask === task.id
            const config = statusConfig[task.status]
            const StatusIcon = config.icon

            return (
              <div
                key={task.id}
                onClick={() => setExpandedTask(isExpanded ? null : task.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault()
                    setExpandedTask(isExpanded ? null : task.id)
                  }
                }}
                role="button"
                tabIndex={0}
                aria-expanded={isExpanded}
                aria-label={`${task.title}, ${config.label}, click to ${isExpanded ? "collapse" : "expand"} details`}
                className={`group relative overflow-hidden rounded-xl border ${config.borderColor} bg-gradient-to-br from-muted/30 to-transparent p-4 cursor-pointer transition-all duration-300 hover:shadow-md hover:scale-[1.01] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 ${
                  isExpanded ? "shadow-sm" : ""
                }`}
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="flex items-start gap-3 flex-1 min-w-0">
                    <div
                      className={`w-10 h-10 rounded-lg ${config.bgColor} flex items-center justify-center flex-shrink-0 ${
                        config.pulse ? "animate-pulse" : ""
                      }`}
                    >
                      <StatusIcon className={`w-5 h-5 ${config.color}`} />
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="font-semibold text-foreground text-sm leading-tight mb-1">{task.title}</p>
                      {isExpanded && (
                        <div className="mt-3 pt-3 border-t border-border/50 space-y-3 animate-in slide-in-from-top-2 duration-200">
                          <div className="grid grid-cols-2 gap-4">
                            <div className="space-y-1">
                              <p className="text-xs font-medium text-muted-foreground">Model</p>
                              <p className="text-sm font-semibold text-foreground">{task.model}</p>
                            </div>
                            <div className="space-y-1">
                              <p className="text-xs font-medium text-muted-foreground">Tokens Used</p>
                              <p className="text-sm font-semibold text-foreground">
                                {task.tokens.toLocaleString()}
                              </p>
                            </div>
                          </div>
                          {task.startTime && (
                            <div className="space-y-1">
                              <p className="text-xs font-medium text-muted-foreground">Started</p>
                              <p className="text-sm font-semibold text-foreground">{task.startTime}</p>
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  </div>
                  <div className="flex items-start gap-2 flex-shrink-0">
                    <span
                      className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold ${config.color} ${config.bgColor} border ${config.borderColor}`}
                    >
                      {config.label}
                    </span>
                    <IconChevronDown
                      className={`w-4 h-4 text-muted-foreground transition-transform duration-200 flex-shrink-0 mt-1 ${
                        isExpanded ? "rotate-180" : ""
                      }`}
                    />
                  </div>
                </div>

                {task.status === "running" && (
                  <div className="mt-4 space-y-2">
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-medium text-muted-foreground">Progress</span>
                      <span className="font-bold text-foreground">{task.progress}%</span>
                    </div>
                    <div className="relative w-full h-2 bg-muted rounded-full overflow-hidden">
                      <div
                        className="absolute inset-y-0 left-0 bg-gradient-to-r from-blue-500 via-purple-500 to-blue-600 rounded-full transition-all duration-500 ease-out shadow-sm"
                        style={{ width: `${task.progress}%` }}
                      >
                        <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/30 to-transparent animate-shimmer" />
                      </div>
                    </div>
                  </div>
                )}
              </div>
            )
          })}
        </div>}
      </div>
    </div>
  )
}
