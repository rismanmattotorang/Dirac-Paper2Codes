/**
 * Real-time task monitor component with WebSocket integration
 * Displays live task status updates with progress bars
 */

'use client';

import { useTasks } from '@/lib/hooks/use-tasks';
import { useWebSocket } from '@/lib/hooks/use-websocket';
import { IconActivity, IconCheckCircle, IconAlertCircle, IconClock, IconChevronDown, IconWifi, IconWifiOff } from '@/components/ui/icons';
import { useState, useMemo, useCallback } from 'react';
import { formatDistanceToNow } from 'date-fns';
import type { Task, TaskStatus } from '@/lib/api/types';

interface RealtimeTaskMonitorProps {
  maxTasks?: number;
  showConnectionStatus?: boolean;
}

export function RealtimeTaskMonitor({
  maxTasks = 10,
  showConnectionStatus = true,
}: RealtimeTaskMonitorProps) {
  const [expandedTask, setExpandedTask] = useState<string | null>(null);
  const { tasks, isLoading, error } = useTasks({ enabled: true });
  const { isConnected, connectionState } = useWebSocket({
    channels: ['tasks'],
    enabled: true,
  });

  // Get tasks sorted by updated_at (most recent first) and limit
  const displayedTasks = useMemo(() => {
    return [...tasks]
      .sort((a, b) => {
        const aTime = new Date(a.updated_at).getTime();
        const bTime = new Date(b.updated_at).getTime();
        return bTime - aTime;
      })
      .slice(0, maxTasks);
  }, [tasks, maxTasks]);

  const getTaskProgress = useCallback((task: Task): number => {
    // Calculate progress based on task status and type
    if (task.status === 'Completed') {
      return 100;
    }
    if (task.status === 'Pending') {
      return 0;
    }
    if (task.status === 'InProgress') {
      // Estimate progress based on task type
      // This could be enhanced with actual progress data from backend
      return 50;
    }
    if (typeof task.status === 'object' && 'Failed' in task.status) {
      return 0;
    }
    return 0;
  }, []);

  const getStatusConfig = useCallback((status: TaskStatus) => {
    if (status === 'Completed') {
      return {
        icon: IconCheckCircle,
        color: 'text-green-600 dark:text-green-400',
        bgColor: 'bg-green-500/10',
        borderColor: 'border-green-200/50 dark:border-green-800/50',
        label: 'Completed',
        pulse: false,
      };
    }
    if (status === 'Pending') {
      return {
        icon: IconClock,
        color: 'text-amber-600 dark:text-amber-400',
        bgColor: 'bg-amber-500/10',
        borderColor: 'border-amber-200/50 dark:border-amber-800/50',
        label: 'Pending',
        pulse: false,
      };
    }
    if (status === 'InProgress') {
      return {
        icon: IconActivity,
        color: 'text-blue-600 dark:text-blue-400',
        bgColor: 'bg-blue-500/10',
        borderColor: 'border-blue-200/50 dark:border-blue-800/50',
        label: 'In Progress',
        pulse: true,
      };
    }
    // Failed status
    return {
      icon: IconAlertCircle,
      color: 'text-red-600 dark:text-red-400',
      bgColor: 'bg-red-500/10',
      borderColor: 'border-red-200/50 dark:border-red-800/50',
      label: typeof status === 'object' && 'Failed' in status ? status.Failed : 'Failed',
      pulse: false,
    };
  }, []);

  const getTaskTypeLabel = useCallback((taskType: Task['task_type']): string => {
    if (taskType === 'Planning') {
      return 'Planning';
    }
    if (typeof taskType === 'object') {
      if ('Analysis' in taskType) {
        return 'Analysis';
      }
      if ('Coding' in taskType) {
        return 'Coding';
      }
      if ('Verification' in taskType) {
        return 'Verification';
      }
      if ('Fix' in taskType) {
        return 'Fix';
      }
    }
    return 'Unknown';
  }, []);

  if (isLoading && tasks.length === 0) {
    return (
      <div className="rounded-xl border border-border bg-card shadow-sm p-6 lg:p-8">
        <div className="flex items-center justify-center py-12">
          <div className="text-center">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
            <p className="text-sm text-muted-foreground">Loading tasks...</p>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-xl border border-border bg-card shadow-sm p-6 lg:p-8">
        <div className="flex items-center justify-center py-12">
          <div className="text-center">
            <IconAlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <p className="text-sm text-red-600 dark:text-red-400">
              Failed to load tasks: {error.message}
            </p>
          </div>
        </div>
      </div>
    );
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
              <h2 className="text-xl font-bold tracking-tight text-foreground">
                Active Tasks
              </h2>
              <p className="text-xs text-muted-foreground mt-0.5">
                Real-time task monitoring
              </p>
            </div>
          </div>
          {showConnectionStatus && (
            <div className="flex items-center gap-2">
              {isConnected ? (
                <>
                  <IconWifi className="w-4 h-4 text-green-500" />
                  <span className="text-xs text-muted-foreground">Connected</span>
                </>
              ) : (
                <>
                  <IconWifiOff className="w-4 h-4 text-amber-500" />
                  <span className="text-xs text-muted-foreground">
                    {connectionState === 'connecting' ? 'Connecting...' : 'Disconnected'}
                  </span>
                </>
              )}
            </div>
          )}
        </div>

        {displayedTasks.length === 0 ? (
          <div className="text-center py-12">
            <IconClock className="w-12 h-12 text-muted-foreground/50 mx-auto mb-4" />
            <p className="text-sm text-muted-foreground">No tasks found</p>
          </div>
        ) : (
          <div className="space-y-3">
            {displayedTasks.map((task) => {
              const isExpanded = expandedTask === task.id;
              const config = getStatusConfig(task.status);
              const StatusIcon = config.icon;
              const progress = getTaskProgress(task);
              const taskTypeLabel = getTaskTypeLabel(task.task_type);

              return (
                <div
                  key={task.id}
                  onClick={() => setExpandedTask(isExpanded ? null : task.id)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      setExpandedTask(isExpanded ? null : task.id);
                    }
                  }}
                  role="button"
                  tabIndex={0}
                  aria-expanded={isExpanded}
                  aria-label={`${task.description}, ${config.label}, click to ${isExpanded ? 'collapse' : 'expand'} details`}
                  className={`group relative overflow-hidden rounded-xl border ${config.borderColor} bg-gradient-to-br from-muted/30 to-transparent p-4 cursor-pointer transition-all duration-300 hover:shadow-md hover:scale-[1.01] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 ${
                    isExpanded ? 'shadow-sm' : ''
                  }`}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex items-start gap-3 flex-1 min-w-0">
                      <div
                        className={`w-10 h-10 rounded-lg ${config.bgColor} flex items-center justify-center flex-shrink-0 ${
                          config.pulse ? 'animate-pulse' : ''
                        }`}
                      >
                        <StatusIcon className={`w-5 h-5 ${config.color}`} />
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="font-semibold text-foreground text-sm leading-tight mb-1">
                          {task.description}
                        </p>
                        <div className="flex items-center gap-2 mt-1">
                          <span className="text-xs text-muted-foreground">
                            {taskTypeLabel}
                          </span>
                          <span className="text-xs text-muted-foreground">•</span>
                          <span className="text-xs text-muted-foreground">
                            {formatDistanceToNow(new Date(task.updated_at), {
                              addSuffix: true,
                            })}
                          </span>
                        </div>
                        {isExpanded && (
                          <div className="mt-3 pt-3 border-t border-border/50 space-y-3 animate-in slide-in-from-top-2 duration-200">
                            <div className="grid grid-cols-2 gap-4">
                              <div className="space-y-1">
                                <p className="text-xs font-medium text-muted-foreground">
                                  Task ID
                                </p>
                                <p className="text-sm font-mono text-foreground truncate">
                                  {task.id}
                                </p>
                              </div>
                              <div className="space-y-1">
                                <p className="text-xs font-medium text-muted-foreground">
                                  Created
                                </p>
                                <p className="text-sm font-semibold text-foreground">
                                  {formatDistanceToNow(new Date(task.created_at), {
                                    addSuffix: true,
                                  })}
                                </p>
                              </div>
                            </div>
                            {task.agent_id && (
                              <div className="space-y-1">
                                <p className="text-xs font-medium text-muted-foreground">
                                  Agent ID
                                </p>
                                <p className="text-sm font-mono text-foreground truncate">
                                  {task.agent_id}
                                </p>
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
                          isExpanded ? 'rotate-180' : ''
                        }`}
                      />
                    </div>
                  </div>

                  {(task.status === 'InProgress' || task.status === 'Pending') && (
                    <div className="mt-4 space-y-2">
                      <div className="flex items-center justify-between text-xs">
                        <span className="font-medium text-muted-foreground">
                          Progress
                        </span>
                        <span className="font-bold text-foreground">{progress}%</span>
                      </div>
                      <div className="relative w-full h-2 bg-muted rounded-full overflow-hidden">
                        <div
                          className={`absolute inset-y-0 left-0 rounded-full transition-all duration-500 ease-out shadow-sm ${
                            task.status === 'InProgress'
                              ? 'bg-gradient-to-r from-blue-500 via-purple-500 to-blue-600'
                              : 'bg-gradient-to-r from-amber-500 to-amber-600'
                          }`}
                          style={{ width: `${progress}%` }}
                        >
                          {task.status === 'InProgress' && (
                            <div className="absolute inset-0 bg-gradient-to-r from-transparent via-white/30 to-transparent animate-shimmer" />
                          )}
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

