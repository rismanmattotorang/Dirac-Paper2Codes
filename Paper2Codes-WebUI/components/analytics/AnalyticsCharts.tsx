"use client"

import { useQuery } from "@tanstack/react-query"
import {
  BarChart,
  Bar,
  LineChart,
  Line,
  PieChart,
  Pie,
  Cell,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts"
import { IconLoader } from "@/components/ui/icons"
import {
  getPerformanceMetrics,
  getUsageStatistics,
  getAgentPerformance,
  getLLMUsageBreakdown,
  getDomainDistribution,
} from "@/lib/api/analytics"

const COLORS = ["#3b82f6", "#8b5cf6", "#10b981", "#f59e0b", "#ef4444", "#ec4899"]

export function TasksOverTimeChart() {
  const { data, isLoading } = useQuery({
    queryKey: ["usage-statistics", "week"],
    queryFn: () => getUsageStatistics("week"),
    refetchInterval: 30000, // Refresh every 30 seconds
  })

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-72">
        <IconLoader className="w-8 h-8 animate-spin text-neutral-400" />
      </div>
    )
  }

  const chartData = data?.data.daily_breakdown.map((day) => ({
    date: new Date(day.date).toLocaleDateString("en-US", { month: "short", day: "numeric" }),
    papers: day.papers,
    tasks: day.tasks,
    repositories: day.repositories,
  }))

  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart data={chartData}>
        <CartesianGrid strokeDasharray="3 3" className="stroke-neutral-200 dark:stroke-neutral-700" />
        <XAxis
          dataKey="date"
          className="text-xs text-neutral-600 dark:text-neutral-400"
        />
        <YAxis className="text-xs text-neutral-600 dark:text-neutral-400" />
        <Tooltip
          contentStyle={{
            backgroundColor: "var(--background)",
            border: "1px solid var(--border)",
            borderRadius: "8px",
          }}
        />
        <Legend />
        <Bar dataKey="papers" fill="#3b82f6" name="Papers" />
        <Bar dataKey="tasks" fill="#8b5cf6" name="Tasks" />
        <Bar dataKey="repositories" fill="#10b981" name="Repositories" />
      </BarChart>
    </ResponsiveContainer>
  )
}

export function AgentPerformanceChart() {
  const { data, isLoading } = useQuery({
    queryKey: ["agent-performance", "24h"],
    queryFn: () => getAgentPerformance("24h"),
    refetchInterval: 30000,
  })

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-72">
        <IconLoader className="w-8 h-8 animate-spin text-neutral-400" />
      </div>
    )
  }

  const chartData = [
    {
      agent: "Planning",
      executions: data?.data.planning_agent.total_executions || 0,
      success: data?.data.planning_agent.successful_executions || 0,
      avgTime: data?.data.planning_agent.average_execution_time_seconds || 0,
    },
    {
      agent: "Analysis",
      executions: data?.data.analysis_agent.total_executions || 0,
      success: data?.data.analysis_agent.successful_executions || 0,
      avgTime: data?.data.analysis_agent.average_execution_time_seconds || 0,
    },
    {
      agent: "Coding",
      executions: data?.data.coding_agent.total_executions || 0,
      success: data?.data.coding_agent.successful_executions || 0,
      avgTime: data?.data.coding_agent.average_execution_time_seconds || 0,
    },
    {
      agent: "Verification",
      executions: data?.data.verification_agent.total_executions || 0,
      success: data?.data.verification_agent.successful_executions || 0,
      avgTime: data?.data.verification_agent.average_execution_time_seconds || 0,
    },
  ]

  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart data={chartData} layout="vertical">
        <CartesianGrid strokeDasharray="3 3" className="stroke-neutral-200 dark:stroke-neutral-700" />
        <XAxis type="number" className="text-xs text-neutral-600 dark:text-neutral-400" />
        <YAxis
          dataKey="agent"
          type="category"
          className="text-xs text-neutral-600 dark:text-neutral-400"
        />
        <Tooltip
          contentStyle={{
            backgroundColor: "var(--background)",
            border: "1px solid var(--border)",
            borderRadius: "8px",
          }}
        />
        <Legend />
        <Bar dataKey="executions" fill="#3b82f6" name="Total" />
        <Bar dataKey="success" fill="#10b981" name="Successful" />
      </BarChart>
    </ResponsiveContainer>
  )
}

export function DomainDistributionChart() {
  const { data, isLoading } = useQuery({
    queryKey: ["domain-distribution"],
    queryFn: getDomainDistribution,
    refetchInterval: 60000,
  })

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-72">
        <IconLoader className="w-8 h-8 animate-spin text-neutral-400" />
      </div>
    )
  }

  const chartData = data?.data.map((item, index) => ({
    name: item.domain,
    value: item.count,
    percentage: item.percentage,
  }))

  return (
    <div className="flex flex-col items-center">
      <ResponsiveContainer width="100%" height={200}>
        <PieChart>
          <Pie
            data={chartData}
            cx="50%"
            cy="50%"
            labelLine={false}
            outerRadius={80}
            fill="#8884d8"
            dataKey="value"
            label={(entry) => `${entry.percentage.toFixed(1)}%`}
          >
            {chartData?.map((entry, index) => (
              <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
            ))}
          </Pie>
          <Tooltip
            contentStyle={{
              backgroundColor: "var(--background)",
              border: "1px solid var(--border)",
              borderRadius: "8px",
            }}
          />
        </PieChart>
      </ResponsiveContainer>
      <div className="mt-4 grid grid-cols-2 gap-2 w-full">
        {chartData?.map((item, index) => (
          <div key={index} className="flex items-center gap-2">
            <div
              className="w-3 h-3 rounded"
              style={{ backgroundColor: COLORS[index % COLORS.length] }}
            />
            <span className="text-sm text-neutral-700 dark:text-neutral-300">{item.name}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

export function LLMUsageChart() {
  const { data, isLoading } = useQuery({
    queryKey: ["llm-usage"],
    queryFn: getLLMUsageBreakdown,
    refetchInterval: 60000,
  })

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <IconLoader className="w-8 h-8 animate-spin text-neutral-400" />
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {data?.data.map((item, index) => (
        <div key={index} className="space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="font-medium text-neutral-900 dark:text-neutral-100">
              {item.model}
            </span>
            <span className="text-neutral-600 dark:text-neutral-400">
              {item.tokens.toLocaleString()} tokens
            </span>
          </div>
          <div className="w-full bg-neutral-200 dark:bg-neutral-700 rounded-full h-2 overflow-hidden">
            <div
              className="h-full rounded-full transition-all"
              style={{
                width: `${item.percentage}%`,
                backgroundColor: COLORS[index % COLORS.length],
              }}
            />
          </div>
          <div className="flex items-center justify-between text-xs text-neutral-500 dark:text-neutral-400">
            <span>{item.requests.toLocaleString()} requests</span>
            <span>${item.cost_usd.toFixed(2)}</span>
          </div>
        </div>
      ))}
    </div>
  )
}

export function PerformanceTimeSeriesChart() {
  const { data, isLoading } = useQuery({
    queryKey: ["performance-metrics", "24h"],
    queryFn: () => getPerformanceMetrics("24h"),
    refetchInterval: 30000,
  })

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-72">
        <IconLoader className="w-8 h-8 animate-spin text-neutral-400" />
      </div>
    )
  }

  const chartData = [
    {
      metric: "API Response",
      p50: data?.data.api_response_times.p50_ms || 0,
      p95: data?.data.api_response_times.p95_ms || 0,
      p99: data?.data.api_response_times.p99_ms || 0,
    },
    {
      metric: "Database",
      p50: data?.data.database_query_times.p50_ms || 0,
      p95: data?.data.database_query_times.p95_ms || 0,
      p99: data?.data.database_query_times.p99_ms || 0,
    },
    {
      metric: "LLM Request",
      p50: data?.data.llm_request_times.p50_ms || 0,
      p95: data?.data.llm_request_times.p95_ms || 0,
      p99: data?.data.llm_request_times.p99_ms || 0,
    },
  ]

  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart data={chartData}>
        <CartesianGrid strokeDasharray="3 3" className="stroke-neutral-200 dark:stroke-neutral-700" />
        <XAxis
          dataKey="metric"
          className="text-xs text-neutral-600 dark:text-neutral-400"
        />
        <YAxis
          label={{ value: "Time (ms)", angle: -90, position: "insideLeft" }}
          className="text-xs text-neutral-600 dark:text-neutral-400"
        />
        <Tooltip
          contentStyle={{
            backgroundColor: "var(--background)",
            border: "1px solid var(--border)",
            borderRadius: "8px",
          }}
        />
        <Legend />
        <Bar dataKey="p50" fill="#10b981" name="P50" />
        <Bar dataKey="p95" fill="#f59e0b" name="P95" />
        <Bar dataKey="p99" fill="#ef4444" name="P99" />
      </BarChart>
    </ResponsiveContainer>
  )
}

