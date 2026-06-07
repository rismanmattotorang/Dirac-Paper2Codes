"use client"

import { IconTrendingUp, IconTrendingDown, IconCalendar, IconDownload } from "@/components/ui/icons"
import { useQuery } from "@tanstack/react-query"
import { getAnalyticsOverview, exportAnalyticsReport } from "@/lib/api/analytics"
import { Button } from "@/components/ui/button"
import { toast } from "sonner"
import {
  TasksOverTimeChart,
  AgentPerformanceChart,
  DomainDistributionChart,
  LLMUsageChart,
  PerformanceTimeSeriesChart,
} from "./AnalyticsCharts"

export function AnalyticsPage() {
  const { data: overviewData, isLoading } = useQuery({
    queryKey: ["analytics-overview"],
    queryFn: getAnalyticsOverview,
    refetchInterval: 30000, // Refresh every 30 seconds
  })

  const overview = overviewData?.data

  const handleExportReport = async (format: "csv" | "json" | "pdf") => {
    try {
      const blob = await exportAnalyticsReport(format)
      const url = URL.createObjectURL(blob)
      const a = document.createElement("a")
      a.href = url
      a.download = `analytics-report.${format}`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      toast.success(`Report exported as ${format.toUpperCase()}`)
    } catch (error) {
      toast.error("Failed to export report")
      console.error(error)
    }
  }

  const stats = [
    {
      label: "Avg Generation Time",
      value: overview
        ? `${Math.floor(overview.average_generation_time_seconds / 60)}m ${Math.floor(overview.average_generation_time_seconds % 60)}s`
        : "--",
      trend: "down" as const,
      change: "12%",
    },
    {
      label: "Success Rate",
      value: overview ? `${(overview.success_rate * 100).toFixed(1)}%` : "--",
      trend: "up" as const,
      change: "5.1%",
    },
    {
      label: "Active Tasks",
      value: overview ? overview.active_tasks.toString() : "--",
      trend: "up" as const,
      change: "3.2%",
    },
    {
      label: "Total Repositories",
      value: overview ? overview.total_repositories.toString() : "--",
      trend: "up" as const,
      change: "8.5%",
    },
  ]

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-neutral-900 dark:text-neutral-200">Analytics</h1>
          <p className="text-neutral-600 dark:text-neutral-400 mt-1">
            Performance insights and pipeline metrics
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => handleExportReport("csv")}
          >
            <IconDownload className="w-4 h-4 mr-2" />
            Export CSV
          </Button>
          <button className="flex items-center gap-2 px-4 py-2 bg-blue-50 dark:bg-blue-950/30 text-blue-600 dark:text-blue-300 rounded-lg hover:bg-blue-100 dark:hover:bg-blue-950/50 font-medium transition-colors">
            <IconCalendar className="w-4 h-4" />
            Last 7 Days
          </button>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {stats.map((stat, idx) => {
          const TrendIcon = stat.trend === "up" ? IconTrendingUp : IconTrendingDown
          const trendColor =
            stat.trend === "up" ? "text-green-600 dark:text-green-400" : "text-blue-600 dark:text-blue-400"

          return (
            <div
              key={idx}
              className="p-5 bg-white dark:bg-neutral-800 rounded-lg border border-neutral-200 dark:border-neutral-700"
            >
              <div className="flex items-start justify-between mb-3">
                <span className="text-xs font-semibold text-neutral-500 dark:text-neutral-400 uppercase">
                  {stat.label}
                </span>
                <TrendIcon className={`w-4 h-4 ${trendColor}`} />
              </div>
              <p className="text-2xl font-bold text-neutral-900 dark:text-neutral-100">{stat.value}</p>
              <p className={`text-xs font-semibold mt-2 ${trendColor}`}>
                {stat.trend === "up" ? "+" : "-"}
                {stat.change}
              </p>
            </div>
          )
        })}
      </div>

      {/* Charts Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Tasks Over Time */}
        <div className="lg:col-span-2 p-6 bg-white dark:bg-neutral-800 rounded-lg border border-neutral-200 dark:border-neutral-700">
          <h3 className="font-semibold text-neutral-900 dark:text-neutral-100 mb-4">
            Activity Over Time
          </h3>
          <TasksOverTimeChart />
        </div>

        {/* Domain Distribution */}
        <div className="p-6 bg-white dark:bg-neutral-800 rounded-lg border border-neutral-200 dark:border-neutral-700">
          <h3 className="font-semibold text-neutral-900 dark:text-neutral-100 mb-4">
            Papers by Domain
          </h3>
          <DomainDistributionChart />
        </div>
      </div>

      {/* Agent Performance and Performance Metrics */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="p-6 bg-white dark:bg-neutral-800 rounded-lg border border-neutral-200 dark:border-neutral-700">
          <h3 className="font-semibold text-neutral-900 dark:text-neutral-100 mb-4">
            Agent Performance
          </h3>
          <AgentPerformanceChart />
        </div>

        <div className="p-6 bg-white dark:bg-neutral-800 rounded-lg border border-neutral-200 dark:border-neutral-700">
          <h3 className="font-semibold text-neutral-900 dark:text-neutral-100 mb-4">
            Response Time Metrics
          </h3>
          <PerformanceTimeSeriesChart />
        </div>
      </div>

      {/* LLM Usage */}
      <div className="p-6 bg-white dark:bg-neutral-800 rounded-lg border border-neutral-200 dark:border-neutral-700">
        <h3 className="font-semibold text-neutral-900 dark:text-neutral-100 mb-4">LLM Usage</h3>
        <LLMUsageChart />
      </div>
    </div>
  )
}
