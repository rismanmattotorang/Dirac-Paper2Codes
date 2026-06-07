"use client"

import { IconCheck, IconAlertCircle, IconFileText, IconClock, IconLoader, IconActivity } from "@/components/ui/icons"
import { useRecentActivity } from "@/lib/hooks/use-dashboard"

const activityIcons = {
  success: IconCheck,
  error: IconAlertCircle,
  info: IconFileText,
}

export function RecentActivity() {
  const { activities: activityData, loading, error } = useRecentActivity()
  
  const activities = activityData.map(activity => ({
    icon: activityIcons[activity.type],
    text: activity.message,
    time: activity.time,
    type: activity.type,
  }))

  const typeConfig = {
    success: {
      iconBg: "bg-green-500/10",
      iconColor: "text-green-600 dark:text-green-400",
      borderColor: "border-green-200/50 dark:border-green-800/50",
      bgGradient: "from-green-50/20 to-transparent dark:from-green-950/10",
    },
    error: {
      iconBg: "bg-red-500/10",
      iconColor: "text-red-600 dark:text-red-400",
      borderColor: "border-red-200/50 dark:border-red-800/50",
      bgGradient: "from-red-50/20 to-transparent dark:from-red-950/10",
    },
    info: {
      iconBg: "bg-blue-500/10",
      iconColor: "text-blue-600 dark:text-blue-400",
      borderColor: "border-blue-200/50 dark:border-blue-800/50",
      bgGradient: "from-blue-50/20 to-transparent dark:from-blue-950/10",
    },
  }

  return (
    <div className="rounded-xl border border-border bg-card shadow-sm hover:shadow-md transition-shadow duration-300">
      <div className="p-6 lg:p-8">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h3 className="text-xl font-bold tracking-tight text-foreground mb-1">Recent Activity</h3>
            <p className="text-xs text-muted-foreground">Latest system events and updates</p>
          </div>
        </div>

        {loading && (
          <div className="flex items-center justify-center py-8">
            <IconLoader className="w-6 h-6 text-muted-foreground animate-spin" />
          </div>
        )}

        {error && (
          <div className="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/20 p-4">
            <div className="flex items-center gap-2">
              <IconAlertCircle className="w-4 h-4 text-red-600 dark:text-red-400" />
              <p className="text-sm text-red-900 dark:text-red-100">Failed to load activity</p>
            </div>
          </div>
        )}

        {!loading && !error && activities.length === 0 && (
          <div className="text-center py-8">
            <IconActivity className="w-12 h-12 text-muted-foreground/50 mx-auto mb-3" />
            <p className="text-sm text-muted-foreground">No recent activity</p>
          </div>
        )}

        {!loading && !error && activities.length > 0 && <div className="space-y-3">
          {activities.map((activity, idx) => {
            const Icon = activity.icon
            const config = typeConfig[activity.type]

            return (
              <div
                key={idx}
                className={`group relative overflow-hidden rounded-xl border ${config.borderColor} bg-gradient-to-br ${config.bgGradient} p-4 transition-all duration-300 hover:shadow-md hover:scale-[1.01]`}
              >
                <div className="flex items-start gap-3">
                  <div
                    className={`w-10 h-10 rounded-xl ${config.iconBg} flex items-center justify-center flex-shrink-0 group-hover:scale-110 transition-transform duration-300`}
                  >
                    <Icon className={`w-5 h-5 ${config.iconColor}`} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-semibold text-foreground leading-tight mb-1.5">{activity.text}</p>
                    <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                      <IconClock className="w-3 h-3" />
                      <span>{activity.time}</span>
                    </div>
                  </div>
                </div>
              </div>
            )
          })}
        </div>}

        {!loading && !error && activities.length > 0 && (
          <button
            className="mt-6 w-full text-sm font-medium text-primary hover:text-primary/80 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 rounded-md py-2"
            aria-label="View all activity"
            type="button"
          >
            View all activity →
          </button>
        )}
      </div>
    </div>
  )
}
