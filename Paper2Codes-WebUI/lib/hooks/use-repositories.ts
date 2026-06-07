/**
 * React hooks for repository-related API operations with React Query and WebSocket integration
 */

'use client';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '@/lib/api/client';
import { useWebSocket, type WebSocketMessage } from './use-websocket';
import { useCallback, useEffect } from 'react';
import type {
  Repository,
  PaginationQuery,
} from '@/lib/api/types';
import { ApiClientError } from '@/lib/api/errors';

export interface UseRepositoriesOptions extends PaginationQuery {
  enabled?: boolean;
  refetchInterval?: number;
}

export interface UseRepositoriesReturn {
  repositories: Repository[];
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
 * Hook to fetch and manage repositories list with React Query
 */
export function useRepositories(
  options: UseRepositoriesOptions = {},
): UseRepositoriesReturn {
  const { enabled = true, refetchInterval, ...pagination } = options;
  const queryClient = useQueryClient();

  // Set up WebSocket connection for real-time updates
  const { isConnected, subscribe, unsubscribe } = useWebSocket({
    channels: ['repositories'],
    enabled,
    onMessage: useCallback(
      (message: WebSocketMessage) => {
        // Invalidate repositories query when receiving repository updates
        if (
          message.type === 'generation_progress' ||
          message.type === 'verification_complete' ||
          message.type === 'repository_updated' ||
          message.type === 'repository_created'
        ) {
          queryClient.invalidateQueries({
            queryKey: ['repositories', pagination],
          });
        }
      },
      [queryClient, pagination],
    ),
  });

  // Subscribe to repositories channel when connected
  useEffect(() => {
    if (isConnected) {
      subscribe('repositories');
      return () => {
        unsubscribe('repositories');
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
    queryKey: ['repositories', pagination],
    queryFn: async () => {
      const response = await apiClient.listRepositories(pagination);
      return response;
    },
    enabled,
    refetchInterval,
    staleTime: 30 * 1000, // 30 seconds
  });

  return {
    repositories: data?.data ?? [],
    isLoading,
    isError,
    error: error instanceof ApiClientError ? error : null,
    pagination: data?.pagination ?? null,
    refetch: () => {
      refetch();
    },
  };
}

export interface UseRepositoryOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export interface UseRepositoryReturn {
  repository: Repository | null;
  isLoading: boolean;
  isError: boolean;
  error: ApiClientError | null;
  refetch: () => void;
}

/**
 * Hook to fetch a single repository with React Query and WebSocket updates
 */
export function useRepository(
  id: string | null,
  options: UseRepositoryOptions = {},
): UseRepositoryReturn {
  const { enabled = true, refetchInterval } = options;
  const queryClient = useQueryClient();

  // Set up WebSocket connection for real-time updates for this specific repository
  const { isConnected, subscribe, unsubscribe } = useWebSocket({
    channels: id ? [`repos:${id}`] : [],
    enabled: enabled && !!id,
    onMessage: useCallback(
      (message: WebSocketMessage) => {
        // Invalidate repository query when receiving updates for this repository
        if (
          (message.type === 'generation_progress' ||
            message.type === 'verification_complete') &&
          message.data &&
          typeof message.data === 'object' &&
          'repository_id' in message.data &&
          message.data.repository_id === id
        ) {
          queryClient.invalidateQueries({ queryKey: ['repositories', id] });
        }
      },
      [queryClient, id],
    ),
  });

  // Subscribe to repository-specific channel when connected
  useEffect(() => {
    if (isConnected && id) {
      subscribe(`repos:${id}`);
      return () => {
        unsubscribe(`repos:${id}`);
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
    queryKey: ['repositories', id],
    queryFn: async () => {
      if (!id) {
        throw new Error('Repository ID is required');
      }
      const response = await apiClient.getRepository(id);
      return response.data;
    },
    enabled: enabled && !!id,
    refetchInterval,
    staleTime: 30 * 1000, // 30 seconds
  });

  return {
    repository: data ?? null,
    isLoading,
    isError,
    error: error instanceof ApiClientError ? error : null,
    refetch: () => {
      refetch();
    },
  };
}
