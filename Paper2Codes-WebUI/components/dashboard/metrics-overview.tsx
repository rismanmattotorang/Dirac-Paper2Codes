"use client"

import { IconTrendingUp, IconZap, IconCheckCircle, IconAlertCircle, IconArrowUp, IconArrowDown, IconLoader } from "@/components/ui/icons"
import { useDashboardMetrics } from "@/lib/hooks/use-dashboard"

export function MetricsOverview() {
  const { metrics: dashboardMetrics, loading, error } = useDashboardMetrics()

  // Show loading state
  if (loading) {
    return (
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 lg:gap-6">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="rounded-xl border border-border bg-card p-6 animate-pulse">
            <div className="flex items-start justify-between mb-4">
              <div className="w-12 h-12 rounded-xl bg-muted" />
              <div className="h-4 w-12 bg-muted rounded" />
            </div>
            <div className="h-8 w-20 bg-muted rounded mb-2" />
            <div className="h-4 w-24 bg-muted rounded" />
          </div>
        ))}
      </div>
    )
  }

  // Show error state
  if (error) {
    // Don't show error if it's just authentication required (expected)
    const isAuthError = error.includes('401') || error.includes('Unauthorized') || error.includes('Authorization');
    
    if (isAuthError) {
      // Show metrics with zero values when auth is required
      return (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 lg:gap-6">
          {[
            { label: "Papers Processed", value: "0", change: "+0%" },
            { label: "Active Tasks", value: "0", change: "+0%" },
            { label: "Success Rate", value: "0%", change: "+0%" },
            { label: "Errors", value: "0", change: "0" },
          ].map((metric, idx) => (
            <div key={idx} className="rounded-xl border border-border bg-card p-6 opacity-60">
              <div className="h-8 w-20 bg-muted rounded mb-2" />
              <p className="text-sm text-muted-foreground">{metric.label}</p>
            </div>
          ))}
        </div>
      );
    }
    
    return (
      <div className="rounded-xl border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/20 p-6">
        <div className="flex items-center gap-3">
          <IconAlertCircle className="w-5 h-5 text-red-600 dark:text-red-400" />
          <div>
            <p className="text-sm font-semibold text-red-900 dark:text-red-100">Failed to load metrics</p>
            <p className="text-xs text-red-700 dark:text-red-300">{error}</p>
          </div>
        </div>
      </div>
    )
  }

  const metrics = [
    {
      label: "Papers Processed",
      value: String(dashboardMetrics?.papersProcessed || 0),
      change: dashboardMetrics?.changeMetrics.papers || "+0%",
      changeType: (dashboardMetrics?.changeMetrics.papers || "").startsWith("+") ? "positive" as const : "negative" as const,
      icon: IconTrendingUp,
      iconBg: "bg-gradient-to-br from-blue-500/10 to-blue-600/20",
      iconColor: "text-blue-600 dark:text-blue-400",
      borderColor: "border-blue-200/50 dark:border-blue-800/50",
      gradient: "from-blue-50/50 to-transparent dark:from-blue-950/20 dark:to-transparent",
    },
    {
      label: "Active Tasks",
      value: String(dashboardMetrics?.activeTasks || 0),
      change: dashboardMetrics?.changeMetrics.tasks || "+0%",
      changeType: (dashboardMetrics?.changeMetrics.tasks || "").startsWith("-") ? "negative" as const : "positive" as const,
      icon: IconZap,
      iconBg: "bg-gradient-to-br from-purple-500/10 to-purple-600/20",
      iconColor: "text-purple-600 dark:text-purple-400",
      borderColor: "border-purple-200/50 dark:border-purple-800/50",
      gradient: "from-purple-50/50 to-transparent dark:from-purple-950/20 dark:to-transparent",
    },
    {
      label: "Success Rate",
      value: `${dashboardMetrics?.successRate || 0}%`,
      change: dashboardMetrics?.changeMetrics.successRate || "+0%",
      changeType: "positive" as const,
      icon: IconCheckCircle,
      iconBg: "bg-gradient-to-br from-green-500/10 to-green-600/20",
      iconColor: "text-green-600 dark:text-green-400",
      borderColor: "border-green-200/50 dark:border-green-800/50",
      gradient: "from-green-50/50 to-transparent dark:from-green-950/20 dark:to-transparent",
    },
    {
      label: "Errors",
      value: String(dashboardMetrics?.errors || 0),
      change: dashboardMetrics?.changeMetrics.errors || "0",
      changeType: "negative" as const,
      icon: IconAlertCircle,
      iconBg: "bg-gradient-to-br from-red-500/10 to-red-600/20",
      iconColor: "text-red-600 dark:text-red-400",
      borderColor: "border-red-200/50 dark:border-red-800/50",
      gradient: "from-red-50/50 to-transparent dark:from-red-950/20 dark:to-transparent",
    },
  ]

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 lg:gap-6" role="region" aria-label="Key metrics overview">
      {metrics.map((metric, idx) => {
        const Icon = metric.icon
        const ChangeIcon = metric.changeType === "positive" ? IconArrowUp : IconArrowDown
        const changeColor =
          metric.changeType === "positive"
            ? "text-green-600 dark:text-green-400"
            : "text-red-600 dark:text-red-400"

        return (
          <article
            key={idx}
            className={`group relative overflow-hidden rounded-xl border ${metric.borderColor} bg-gradient-to-br ${metric.gradient} backdrop-blur-sm shadow-sm hover:shadow-md transition-all duration-300 hover:-translate-y-1 focus-within:ring-2 focus-within:ring-ring focus-within:ring-offset-2`}
            tabIndex={0}
            aria-label={`${metric.label}: ${metric.value}, ${metric.change} change`}
          >
            <div className="p-6">
              <div className="flex items-start justify-between mb-4">
                <div
                  className={`w-12 h-12 rounded-xl flex items-center justify-center ${metric.iconBg} ${metric.iconColor} shadow-sm group-hover:scale-110 transition-transform duration-300`}
                >
                  <Icon className="w-6 h-6" />
                </div>
                <div className={`flex items-center gap-1 text-xs font-semibold ${changeColor}`}>
                  <ChangeIcon className="w-3 h-3" />
                  <span>{metric.change}</span>
                </div>
              </div>
              <div className="space-y-1">
                <p className="text-3xl font-bold tracking-tight text-foreground">{metric.value}</p>
                <p className="text-sm font-medium text-muted-foreground">{metric.label}</p>
              </div>
            </div>
            {/* Subtle shine effect on hover */}
            <div className="absolute inset-0 -translate-x-full group-hover:translate-x-full transition-transform duration-1000 bg-gradient-to-r from-transparent via-white/10 to-transparent pointer-events-none" />
          </article>
        )
      })}
    </div>
  )
}
