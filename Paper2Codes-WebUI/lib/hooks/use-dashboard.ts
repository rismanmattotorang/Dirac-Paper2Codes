/**
 * Dashboard data fetching hooks
 */

import { useEffect, useState } from 'react';
import { apiClient } from '@/lib/api/client';
import type { Task, Paper } from '@/lib/api/types';

export interface DashboardMetrics {
  papersProcessed: number;
  activeTasks: number;
  successRate: number;
  errors: number;
  changeMetrics: {
    papers: string;
    tasks: string;
    successRate: string;
    errors: string;
  };
}

export interface AgentStatus {
  name: string;
  status: 'idle' | 'running' | 'error';
  description: string;
}

export interface Activity {
  id: string;
  type: 'success' | 'error' | 'info';
  message: string;
  time: string;
}

const describeTaskType = (taskType: Task['task_type']): string => {
  if (typeof taskType === 'string') {
    return taskType;
  }

  if ('Analysis' in taskType) {
    return `Analysis (${taskType.Analysis.module_id})`;
  }

  if ('Coding' in taskType) {
    return `Coding (${taskType.Coding.module_id})`;
  }

  if ('Verification' in taskType) {
    return 'Verification';
  }

  if ('Fix' in taskType) {
    return `Fix (${taskType.Fix.module_id})`;
  }

  return 'Planning';
};

const mapTaskStatusToActivity = (status: Task['status']): Activity['type'] => {
  if (typeof status === 'string') {
    if (status === 'Completed') {
      return 'success';
    }
    if (status === 'InProgress' || status === 'Pending') {
      return 'info';
    }
  }

  if (status && typeof status === 'object' && 'Failed' in status) {
    return 'error';
  }

  return 'info';
};

/**
 * Hook to fetch dashboard metrics from analytics overview
 */
export function useDashboardMetrics() {
  const [metrics, setMetrics] = useState<DashboardMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    const fetchMetrics = async () => {
      try {
        setLoading(true);
        setError(null);

        const response = await apiClient.getAnalyticsOverview();
        
        if (mounted && response) {
          // Handle both direct data and wrapped response
          const data = response.data || response as any;
          
          // Calculate metrics with safe defaults
          const papersProcessed = data?.total_papers ?? 0;
          const activeTasks = data?.active_tasks ?? 0;
          const successRate = data?.success_rate 
            ? Math.round(data.success_rate * 100 * 10) / 10 
            : 0;
          const errors = data?.failed_tasks ?? 0;

          // Mock change metrics (in production, would calculate from historical data)
          const changeMetrics = {
            papers: '+12%',
            tasks: activeTasks > 5 ? '-3%' : '+5%',
            successRate: '+5.1%',
            errors: errors > 0 ? `+${errors}` : '0',
          };

          setMetrics({
            papersProcessed,
            activeTasks,
            successRate,
            errors,
            changeMetrics,
          });
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch dashboard metrics:', err);
          const errorMessage = err instanceof Error 
            ? err.message 
            : 'Failed to fetch metrics';
          
          // Check if it's a network error
          if (errorMessage.includes('Failed to fetch') || errorMessage.includes('Network error') || errorMessage.includes('Cannot connect')) {
            setError('Cannot connect to backend. Please ensure the backend is running on http://127.0.0.1:8080');
          } else {
            // Don't show other errors (like auth) - just log them
            console.warn('Dashboard metrics error (non-critical):', errorMessage);
            setError(null); // Clear error for non-critical issues
          }
          
          // Set fallback data on error
          setMetrics({
            papersProcessed: 0,
            activeTasks: 0,
            successRate: 0,
            errors: 0,
            changeMetrics: {
              papers: '0%',
              tasks: '0%',
              successRate: '0%',
              errors: '0',
            },
          });
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    fetchMetrics();

    // Refresh metrics every 30 seconds
    const interval = setInterval(fetchMetrics, 30000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return { metrics, loading, error };
}

/**
 * Hook to fetch active tasks for task monitor
 */
export function useActiveTasks() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    const fetchTasks = async () => {
      try {
        setLoading(true);
        setError(null);

        const response = await apiClient.listTasks({ page: 1, per_page: 10 });
        
        if (mounted && response) {
          // Handle both paginated and direct array responses
          const tasksData = Array.isArray(response.data) ? response.data : [];
          setTasks(tasksData);
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch tasks:', err);
          setError(err instanceof Error ? err.message : 'Failed to fetch tasks');
          setTasks([]);
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    fetchTasks();

    // Refresh tasks every 10 seconds
    const interval = setInterval(fetchTasks, 10000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return { tasks, loading, error };
}

/**
 * Hook to fetch recent papers
 */
export function useRecentPapers(limit: number = 5) {
  const [papers, setPapers] = useState<Paper[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    const fetchPapers = async () => {
      try {
        setLoading(true);
        setError(null);

        const response = await apiClient.listPapers({ page: 1, per_page: limit });
        
        if (mounted && response) {
          // Handle both paginated and direct array responses
          const papersData = Array.isArray(response.data) ? response.data : [];
          setPapers(papersData);
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch papers:', err);
          setError(err instanceof Error ? err.message : 'Failed to fetch papers');
          setPapers([]);
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    fetchPapers();

    return () => {
      mounted = false;
    };
  }, [limit]);

  return { papers, loading, error };
}

/**
 * Hook to fetch agent status from performance metrics
 */
export function useAgentStatus() {
  const [agents, setAgents] = useState<AgentStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    const fetchAgentStatus = async () => {
      try {
        setLoading(true);
        setError(null);

        const response = await apiClient.getAgentPerformance();
        
        if (mounted && response) {
          // Handle both direct data and wrapped response
          const data = response.data || response as any;
          
          // Convert agent performance data to status with safe defaults
          const agentStatuses: AgentStatus[] = [
            {
              name: 'Planning',
              status: (data?.planning_agent?.total_executions ?? 0) > 0 ? 'running' : 'idle',
              description: 'Strategic planning',
            },
            {
              name: 'Analysis',
              status: (data?.analysis_agent?.total_executions ?? 0) > 0 ? 'running' : 'idle',
              description: 'Data extraction',
            },
            {
              name: 'Coding',
              status: (data?.coding_agent?.total_executions ?? 0) > 0 ? 'running' : 'idle',
              description: 'Code generation',
            },
            {
              name: 'Verification',
              status: (data?.verification_agent?.total_executions ?? 0) > 0 ? 'running' : 'idle',
              description: 'Quality assurance',
            },
          ];

          setAgents(agentStatuses);
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch agent status:', err);
          const errorMessage = err instanceof Error 
            ? err.message 
            : 'Failed to fetch agent status';
          
          // Check if it's a network error
          if (errorMessage.includes('Failed to fetch') || errorMessage.includes('Network error') || errorMessage.includes('Cannot connect')) {
            setError('Cannot connect to backend. Please ensure the backend is running on http://127.0.0.1:8080');
          } else {
            // Don't show other errors (like auth) - just log them
            console.warn('Agent status error (non-critical):', errorMessage);
            setError(null); // Clear error for non-critical issues
          }
          
          // Set default idle status on error
          setAgents([
            { name: 'Planning', status: 'idle', description: 'Strategic planning' },
            { name: 'Analysis', status: 'idle', description: 'Data extraction' },
            { name: 'Coding', status: 'idle', description: 'Code generation' },
            { name: 'Verification', status: 'idle', description: 'Quality assurance' },
          ]);
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    fetchAgentStatus();

    // Refresh agent status every 15 seconds
    const interval = setInterval(fetchAgentStatus, 15000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return { agents, loading, error };
}

/**
 * Hook to fetch recent activity
 * This would ideally come from a dedicated activity log endpoint
 * For now, we'll derive it from tasks and other data
 */
export function useRecentActivity() {
  const [activities, setActivities] = useState<Activity[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;

    const fetchActivity = async () => {
      try {
        setLoading(true);
        setError(null);

        // Fetch recent tasks to derive activity
        const tasksResponse = await apiClient.listTasks({ page: 1, per_page: 5 });
        
        if (mounted && tasksResponse) {
          // Handle both paginated and direct array responses
          const tasks = Array.isArray(tasksResponse.data) ? tasksResponse.data : [];
          
          // Convert tasks to activity items with safe defaults
          const activityItems: Activity[] = tasks
            .filter(task => task && task.id && task.created_at)
            .map((task) => {
              const timeAgo = calculateTimeAgo(task.created_at);
              const taskType = describeTaskType(task.task_type);
              const description = task.description || 'No description';
              
              return {
                id: task.id,
                type: mapTaskStatusToActivity(task.status),
                message: `${taskType} - ${description}`,
                time: timeAgo,
              };
            });

          setActivities(activityItems);
        }
      } catch (err) {
        if (mounted) {
          console.error('Failed to fetch activity:', err);
          setError(err instanceof Error ? err.message : 'Failed to fetch activity');
          setActivities([]);
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    };

    fetchActivity();

    // Refresh activity every 20 seconds
    const interval = setInterval(fetchActivity, 20000);

    return () => {
      mounted = false;
      clearInterval(interval);
    };
  }, []);

  return { activities, loading, error };
}

/**
 * Helper to calculate time ago from ISO timestamp
 */
function calculateTimeAgo(timestamp: string): string {
  try {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    
    if (diffMins < 1) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    
    const diffDays = Math.floor(diffHours / 24);
    return `${diffDays}d ago`;
  } catch {
    return 'recently';
  }
}
