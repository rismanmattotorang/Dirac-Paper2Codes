"use client"

import { PaperUploadCard } from "./paper-upload-card"
import { TaskMonitor } from "./task-monitor"
import { MetricsOverview } from "./metrics-overview"
import { RecentActivity } from "./recent-activity"
import { AgentStatus } from "./agent-status"

export function Dashboard() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-background via-background to-muted/20">
      <div className="p-4 sm:p-6 lg:p-8 space-y-8 max-w-7xl mx-auto">
        {/* Header Section */}
        <header className="space-y-2 animate-in fade-in slide-in-from-top-4 duration-500">
          <h1 className="text-4xl font-bold tracking-tight text-foreground">
            Dashboard
          </h1>
          <p className="text-muted-foreground text-lg max-w-2xl">
            Monitor your paper-to-code generation pipeline in real-time
          </p>
        </header>

        {/* Metrics Overview */}
        <div className="animate-in fade-in slide-in-from-bottom-4 duration-500 delay-100">
          <MetricsOverview />
        </div>

        {/* Main Content Grid */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 lg:gap-8">
          {/* Left Column - Main Content */}
          <div className="lg:col-span-2 space-y-6 lg:space-y-8">
            <div className="animate-in fade-in slide-in-from-left-4 duration-500 delay-200">
              <PaperUploadCard />
            </div>
            <div className="animate-in fade-in slide-in-from-left-4 duration-500 delay-300">
              <TaskMonitor />
            </div>
          </div>

          {/* Right Column - Sidebar */}
          <div className="space-y-6 lg:space-y-8">
            <div className="animate-in fade-in slide-in-from-right-4 duration-500 delay-200">
              <AgentStatus />
            </div>
            <div className="animate-in fade-in slide-in-from-right-4 duration-500 delay-300">
              <RecentActivity />
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
