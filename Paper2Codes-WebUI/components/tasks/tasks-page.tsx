"use client"

import { IconZap, IconClock, IconCheckCircle, IconAlertCircle, IconActivity, IconLoader, IconPlus } from "@/components/ui/icons"
import { useTasks } from "@/lib/hooks/use-tasks"
import type { Task, TaskStatus } from "@/lib/api/types"
import { formatDistanceToNow } from "date-fns"
import { useMemo } from "react"

export function TasksPage() {
  const { tasks, isLoading, error } = useTasks({ enabled: true, page: 1, per_page: 100 })
  
  // Calculate statistics
  const stats = useMemo(() => {
    const running = tasks.filter(t => t.status === "InProgress").length
    const queued = tasks.filter(t => t.status === "Pending").length
    const completed = tasks.filter(t => t.status === "Completed").length
    const failed = tasks.filter(t => typeof t.status === "object" && "Failed" in t.status).length
    
    return { running, queued, completed, failed }
  }, [tasks])

  const getStatusIcon = (status: TaskStatus) => {
    if (status === "InProgress") {
      return <IconZap className="w-5 h-5 text-blue-500 animate-pulse" />
    }
    if (status === "Pending") {
      return <IconClock className="w-5 h-5 text-yellow-500" />
    }
    if (status === "Completed") {
      return <IconCheckCircle className="w-5 h-5 text-green-500" />
    }
    return <IconAlertCircle className="w-5 h-5 text-red-500" />
  }

  const getStatusBadge = (status: TaskStatus) => {
    const baseClass = "px-3 py-1 rounded-full text-xs font-medium"
    if (status === "InProgress") {
      return `${baseClass} bg-blue-50 dark:bg-blue-950/30 text-blue-700 dark:text-blue-300`
    }
    if (status === "Pending") {
      return `${baseClass} bg-yellow-50 dark:bg-yellow-950/30 text-yellow-700 dark:text-yellow-300`
    }
    if (status === "Completed") {
      return `${baseClass} bg-green-50 dark:bg-green-950/30 text-green-700 dark:text-green-300`
    }
    return `${baseClass} bg-red-50 dark:bg-red-950/30 text-red-700 dark:text-red-300`
  }
  
  const getStatusText = (status: TaskStatus): string => {
    if (status === "InProgress") return "running"
    if (status === "Pending") return "queued"
    if (status === "Completed") return "completed"
    if (typeof status === "object" && "Failed" in status) return "failed"
    return "unknown"
  }
  
  const getTaskTypeLabel = (taskType: Task["task_type"]): string => {
    if (taskType === "Planning") return "Planning"
    if (typeof taskType === "object") {
      if ("Analysis" in taskType) return "Analysis"
      if ("Coding" in taskType) return "Coding"
      if ("Verification" in taskType) return "Verification"
      if ("Fix" in taskType) return "Fix"
    }
    return "Unknown"
  }
  
  const getProgress = (task: Task): number => {
    if (task.status === "Completed") return 100
    if (task.status === "InProgress") return 50
    return 0
  }

  if (error) {
    return (
      <div className="p-4 sm:p-6 lg:p-8 space-y-6 max-w-7xl mx-auto">
        <div className="space-y-2">
          <h1 className="text-3xl font-bold text-neutral-900 dark:text-white">Task Queue</h1>
          <p className="text-neutral-600 dark:text-neutral-400">Monitor active and completed processing tasks</p>
        </div>
        <div className="p-8 rounded-lg bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700">
          <div className="text-center">
            <IconAlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-neutral-900 dark:text-white mb-2">Failed to load tasks</h3>
            <p className="text-sm text-neutral-600 dark:text-neutral-400">{error.message}</p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="p-4 sm:p-6 lg:p-8 space-y-6 max-w-7xl mx-auto">
      {/* Header */}
      <div className="space-y-2">
        <h1 className="text-3xl font-bold text-neutral-900 dark:text-white">Task Queue</h1>
        <p className="text-neutral-600 dark:text-neutral-400">Monitor active and completed processing tasks</p>
      </div>

      {/* Loading State */}
      {isLoading && tasks.length === 0 && (
        <div className="p-12 rounded-lg bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700">
          <div className="text-center">
            <IconLoader className="w-12 h-12 text-blue-500 mx-auto mb-4 animate-spin" />
            <p className="text-sm text-neutral-600 dark:text-neutral-400">Loading tasks...</p>
          </div>
        </div>
      )}

      {/* Stats */}
      {!isLoading && tasks.length > 0 && (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="p-4 rounded-lg bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700">
            <div className="flex items-center justify-between">
              <p className="text-sm text-neutral-600 dark:text-neutral-400 mb-1">Running</p>
              <IconActivity className="w-5 h-5 text-blue-500" />
            </div>
            <p className="text-2xl font-bold text-neutral-900 dark:text-white">{stats.running}</p>
          </div>
          <div className="p-4 rounded-lg bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700">
            <div className="flex items-center justify-between">
              <p className="text-sm text-neutral-600 dark:text-neutral-400 mb-1">Queued</p>
              <IconClock className="w-5 h-5 text-yellow-500" />
            </div>
            <p className="text-2xl font-bold text-neutral-900 dark:text-white">{stats.queued}</p>
          </div>
          <div className="p-4 rounded-lg bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700">
            <div className="flex items-center justify-between">
              <p className="text-sm text-neutral-600 dark:text-neutral-400 mb-1">Completed</p>
              <IconCheckCircle className="w-5 h-5 text-green-500" />
            </div>
            <p className="text-2xl font-bold text-neutral-900 dark:text-white">{stats.completed}</p>
          </div>
          <div className="p-4 rounded-lg bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700">
            <div className="flex items-center justify-between">
              <p className="text-sm text-neutral-600 dark:text-neutral-400 mb-1">Failed</p>
              <IconAlertCircle className="w-5 h-5 text-red-500" />
            </div>
            <p className="text-2xl font-bold text-neutral-900 dark:text-white">{stats.failed}</p>
          </div>
        </div>
      )}

      {/* Empty State */}
      {!isLoading && tasks.length === 0 && (
        <div className="p-12 rounded-lg bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700">
          <div className="text-center">
            <IconActivity className="w-12 h-12 text-neutral-400 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-neutral-900 dark:text-white mb-2">No tasks found</h3>
            <p className="text-sm text-neutral-600 dark:text-neutral-400">Tasks will appear here when papers are being processed</p>
          </div>
        </div>
      )}

      {/* Task List */}
      {tasks.length > 0 && (
        <div className="space-y-4">
          <h2 className="text-lg font-semibold text-neutral-900 dark:text-white">
            {stats.running > 0 || stats.queued > 0 ? "Active Tasks" : "Recent Tasks"}
          </h2>
          <div className="space-y-3">
            {tasks.map((task) => {
              const progress = getProgress(task)
              const statusText = getStatusText(task.status)
              const taskTypeLabel = getTaskTypeLabel(task.task_type)
              
              return (
                <div
                  key={task.id}
                  className="p-4 rounded-lg bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700 hover:shadow-lg transition-shadow"
                >
                  <div className="flex items-start justify-between gap-4 mb-3">
                    <div className="flex items-start gap-3 flex-1">
                      {getStatusIcon(task.status)}
                      <div className="flex-1 min-w-0">
                        <h3 className="font-semibold text-neutral-900 dark:text-white">{task.description}</h3>
                        <div className="flex items-center gap-2 mt-1">
                          <p className="text-sm text-neutral-600 dark:text-neutral-400">Agent: {taskTypeLabel}</p>
                          <span className="text-neutral-400">•</span>
                          <p className="text-sm text-neutral-600 dark:text-neutral-400">
                            {formatDistanceToNow(new Date(task.updated_at), { addSuffix: true })}
                          </p>
                        </div>
                      </div>
                    </div>
                    <span className={getStatusBadge(task.status)}>{statusText}</span>
                  </div>

                  {(statusText === "running" || statusText === "queued") && (
                    <div className="space-y-2">
                      <div className="flex justify-between text-xs">
                        <span className="text-neutral-600 dark:text-neutral-400">Progress</span>
                        <span className="font-medium text-neutral-900 dark:text-white">{progress}%</span>
                      </div>
                      <div className="w-full h-2 bg-neutral-200 dark:bg-neutral-700 rounded-full overflow-hidden">
                        <div
                          className="h-full bg-gradient-to-r from-blue-500 to-purple-500 rounded-full transition-all"
                          style={{ width: `${progress}%` }}
                        ></div>
                      </div>
                    </div>
                  )}

                  {statusText === "completed" && (
                    <p className="text-xs text-green-600 dark:text-green-400">
                      Completed {formatDistanceToNow(new Date(task.updated_at), { addSuffix: true })}
                    </p>
                  )}
                  
                  {statusText === "failed" && typeof task.status === "object" && "Failed" in task.status && (
                    <p className="text-xs text-red-600 dark:text-red-400">
                      Failed: {task.status.Failed}
                    </p>
                  )}
                </div>
              )
            })}
          </div>
        </div>
      )}
    </div>
  )
}
