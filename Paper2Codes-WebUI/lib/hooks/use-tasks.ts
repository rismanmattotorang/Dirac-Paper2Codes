/**
 * React hooks for task-related API operations with React Query and WebSocket integration
 */

'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '@/lib/api/client';
import { useWebSocket, type WebSocketMessage } from './use-websocket';
import { useCallback, useEffect } from 'react';
import type {
  Task,
  PaginatedResponse,
  PaginationQuery,
} from '@/lib/api/types';
import { ApiClientError } from '@/lib/api/errors';

export interface UseTasksOptions extends PaginationQuery {
  enabled?: boolean;
  refetchInterval?: number;
}

export interface UseTasksReturn {
  tasks: Task[];
  isLoading: boolean;
  isError: boolean;
  error: ApiClientError | null;
  pagination: {
    page: number;
    per_page: number;
    total: number;
    total_pages: number;
  } | null;
  refetch: () => void;
}

/**
 * Hook to fetch and manage tasks list with React Query
 */
export function useTasks(options: UseTasksOptions = {}): UseTasksReturn {
  const { enabled = true, refetchInterval, ...pagination } = options;
  const queryClient = useQueryClient();

  // Set up WebSocket connection for real-time updates
  const { isConnected, subscribe, unsubscribe } = useWebSocket({
    channels: ['tasks'],
    enabled,
    onMessage: useCallback(
      (message: WebSocketMessage) => {
        // Invalidate tasks query when receiving task updates
        if (message.type === 'task_update' || message.type === 'task_created') {
          queryClient.invalidateQueries({ queryKey: ['tasks', pagination] });
        }
      },
      [queryClient, pagination],
    ),
  });

  // Subscribe to tasks channel when connected
  useEffect(() => {
    if (isConnected) {
      subscribe('tasks');
      return () => {
        unsubscribe('tasks');
      };
    }
  }, [isConnected, subscribe, unsubscribe]);

  const {
    data,
    isLoading,
    isError,
    error,
    refetch,
  } = useQuery({
    queryKey: ['tasks', pagination],
    queryFn: async () => {
      try {
        const response = await apiClient.listTasks(pagination);
        if (!response || typeof response !== 'object') {
          throw new ApiClientError('INVALID_RESPONSE', 'Invalid response format');
        }
        return response;
      } catch (err) {
        if (err instanceof ApiClientError) {
          throw err;
        }
        throw new ApiClientError(
          'UNKNOWN_ERROR',
          err instanceof Error ? err.message : 'Failed to fetch tasks',
        );
      }
    },
    enabled,
    refetchInterval,
    staleTime: 30 * 1000, // 30 seconds
    retry: (failureCount, error) => {
      if (error instanceof ApiClientError && error.statusCode) {
        if (error.statusCode === 401 || error.statusCode === 403) {
          return false;
        }
      }
      return failureCount < 2;
    },
  });

  return {
    tasks: Array.isArray(data?.data) ? data.data : [],
    isLoading,
    isError,
    error: error instanceof ApiClientError ? error : (error ? new ApiClientError('UNKNOWN_ERROR', String(error)) : null),
    pagination: data?.pagination ?? null,
    refetch: () => {
      refetch();
    },
  };
}

export interface UseTaskOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export interface UseTaskReturn {
  task: Task | null;
  isLoading: boolean;
  isError: boolean;
  error: ApiClientError | null;
  refetch: () => void;
}

/**
 * Hook to fetch a single task with React Query and WebSocket updates
 */
export function useTask(
  id: string | null,
  options: UseTaskOptions = {},
): UseTaskReturn {
  const { enabled = true, refetchInterval } = options;
  const queryClient = useQueryClient();

  // Set up WebSocket connection for real-time updates for this specific task
  const { isConnected, subscribe, unsubscribe } = useWebSocket({
    channels: id ? [`tasks:${id}`] : [],
    enabled: enabled && !!id,
    onMessage: useCallback(
      (message: WebSocketMessage) => {
        // Invalidate task query when receiving updates for this task
        if (
          message.type === 'task_update' &&
          message.data &&
          typeof message.data === 'object' &&
          'task_id' in message.data &&
          message.data.task_id === id
        ) {
          queryClient.invalidateQueries({ queryKey: ['tasks', id] });
        }
      },
      [queryClient, id],
    ),
  });

  // Subscribe to task-specific channel when connected
  useEffect(() => {
    if (isConnected && id) {
      subscribe(`tasks:${id}`);
      return () => {
        unsubscribe(`tasks:${id}`);
      };
    }
  }, [isConnected, id, subscribe, unsubscribe]);

  const {
    data,
    isLoading,
    isError,
    error,
    refetch,
  } = useQuery({
    queryKey: ['tasks', id],
    queryFn: async () => {
      if (!id) {
        throw new Error('Task ID is required');
      }
      const response = await apiClient.getTask(id);
      return response.data;
    },
    enabled: enabled && !!id,
    refetchInterval,
    staleTime: 30 * 1000, // 30 seconds
  });

  return {
    task: data ?? null,
    isLoading,
    isError,
    error: error instanceof ApiClientError ? error : null,
    refetch: () => {
      refetch();
    },
  };
}

/**
 * Hook to create a new task
 */
export function useCreateTask() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (taskData: {
      task_type: string;
      description: string;
      paper_id?: string;
      module_id?: string;
    }) => {
      return await apiClient.createTask(taskData);
    },
    onSuccess: () => {
      // Invalidate tasks list to refetch
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    },
  });
}

/**
 * Hook to update task status
 */
export function useUpdateTaskStatus() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      taskId,
      status,
    }: {
      taskId: string;
      status: string;
    }) => {
      return await apiClient.updateTaskStatus(taskId, status);
    },
    onSuccess: (data, variables) => {
      // Invalidate both the specific task and the tasks list
      queryClient.invalidateQueries({ queryKey: ['tasks', variables.taskId] });
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    },
  });
}

/**
 * Hook to cancel a task
 */
export function useCancelTask() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (taskId: string) => {
      return await apiClient.cancelTask(taskId);
    },
    onSuccess: (data, taskId) => {
      // Invalidate both the specific task and the tasks list
      queryClient.invalidateQueries({ queryKey: ['tasks', taskId] });
      queryClient.invalidateQueries({ queryKey: ['tasks'] });
    },
  });
}
