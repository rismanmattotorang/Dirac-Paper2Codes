/**
 * React hooks for paper-related API operations with React Query and WebSocket integration
 */

'use client';

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '@/lib/api/client';
import { useWebSocket, type WebSocketMessage } from './use-websocket';
import { useCallback, useEffect } from 'react';
import type {
  Paper,
  PaperSegment,
  CreatePaperRequest,
  UpdatePaperRequest,
  PaginationQuery,
  PaginationMeta,
} from '@/lib/api/types';
import { ApiClientError } from '@/lib/api/errors';
import {
  listPapers as listPapersApi,
  getPaper as getPaperApi,
  createPaper as createPaperApi,
  updatePaper as updatePaperApi,
  deletePaper as deletePaperApi,
  getPaperStatus,
  getPaperSegments,
} from '@/lib/api/papers';

export interface UsePapersOptions extends PaginationQuery {
  enabled?: boolean;
  refetchInterval?: number;
}

export interface UsePapersReturn {
  papers: Paper[];
  isLoading: boolean;
  isError: boolean;
  error: ApiClientError | null;
  pagination: PaginationMeta | null;
  refetch: () => void;
  createPaper: ReturnType<typeof useCreatePaper>['mutateAsync'];
  updatePaper: ReturnType<typeof useUpdatePaper>['mutateAsync'];
  deletePaper: ReturnType<typeof useDeletePaper>['mutateAsync'];
}

/**
 * Hook to fetch and manage papers list with React Query
 */
export function usePapers(options: UsePapersOptions = {}): UsePapersReturn {
  const { enabled = true, refetchInterval, ...pagination } = options;
  const queryClient = useQueryClient();

  // Set up WebSocket connection for real-time updates
  const { isConnected, subscribe, unsubscribe } = useWebSocket({
    channels: ['papers'],
    enabled,
    onMessage: useCallback(
      (message: WebSocketMessage) => {
        // Invalidate papers query when receiving paper updates
        if (
          message.type === 'paper_processed' ||
          message.type === 'paper_updated' ||
          message.type === 'paper_created'
        ) {
          queryClient.invalidateQueries({ queryKey: ['papers', pagination] });
        }
      },
      [queryClient, pagination],
    ),
  });

  // Subscribe to papers channel when connected
  useEffect(() => {
    if (isConnected) {
      subscribe('papers');
      return () => {
        unsubscribe('papers');
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
    queryKey: ['papers', pagination],
    queryFn: async () => {
      try {
        const response = await listPapersApi(pagination);
        // Ensure response has expected structure
        if (!response || typeof response !== 'object') {
          throw new ApiClientError('INVALID_RESPONSE', 'Invalid response format');
        }
        return response;
      } catch (err) {
        // Convert any error to ApiClientError
        if (err instanceof ApiClientError) {
          throw err;
        }
        throw new ApiClientError(
          'UNKNOWN_ERROR',
          err instanceof Error ? err.message : 'Failed to fetch papers',
        );
      }
    },
    enabled,
    refetchInterval,
    staleTime: 30 * 1000, // 30 seconds
    retry: (failureCount, error) => {
      // Don't retry on 401/403 errors
      if (error instanceof ApiClientError && error.statusCode) {
        if (error.statusCode === 401 || error.statusCode === 403) {
          return false;
        }
      }
      return failureCount < 2;
    },
  });

  const createPaper = useCreatePaper();
  const updatePaper = useUpdatePaper();
  const deletePaper = useDeletePaper();

  return {
    papers: Array.isArray(data?.data) ? data.data : [],
    isLoading,
    isError,
    error: error instanceof ApiClientError ? error : (error ? new ApiClientError('UNKNOWN_ERROR', String(error)) : null),
    pagination: data?.pagination ?? null,
    refetch: () => {
      refetch();
    },
    createPaper: createPaper.mutateAsync,
    updatePaper: updatePaper.mutateAsync,
    deletePaper: deletePaper.mutateAsync,
  };
}

export interface UsePaperOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export interface UsePaperReturn {
  paper: Paper | null;
  isLoading: boolean;
  isError: boolean;
  error: ApiClientError | null;
  refetch: () => void;
}

/**
 * Hook to fetch a single paper with React Query and WebSocket updates
 */
export function usePaper(
  id: string | null,
  options: UsePaperOptions = {},
): UsePaperReturn {
  const { enabled = true, refetchInterval } = options;
  const queryClient = useQueryClient();

  // Set up WebSocket connection for real-time updates for this specific paper
  const { isConnected, subscribe, unsubscribe } = useWebSocket({
    channels: id ? [`papers:${id}`] : [],
    enabled: enabled && !!id,
    onMessage: useCallback(
      (message: WebSocketMessage) => {
        // Invalidate paper query when receiving updates for this paper
        if (
          message.type === 'paper_processed' &&
          message.data &&
          typeof message.data === 'object' &&
          'paper_id' in message.data &&
          message.data.paper_id === id
        ) {
          queryClient.invalidateQueries({ queryKey: ['papers', id] });
        }
      },
      [queryClient, id],
    ),
  });

  // Subscribe to paper-specific channel when connected
  useEffect(() => {
    if (isConnected && id) {
      subscribe(`papers:${id}`);
      return () => {
        unsubscribe(`papers:${id}`);
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
    queryKey: ['papers', id],
    queryFn: async () => {
      if (!id) {
        throw new Error('Paper ID is required');
      }
      const response = await getPaperApi(id);
      return response.data;
    },
    enabled: enabled && !!id,
    refetchInterval,
    staleTime: 30 * 1000, // 30 seconds
  });

  return {
    paper: data ?? null,
    isLoading,
    isError,
    error: error instanceof ApiClientError ? error : null,
    refetch: () => {
      refetch();
    },
  };
}

/**
 * Hook to create a new paper
 */
export function useCreatePaper() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: CreatePaperRequest) => {
      return await createPaperApi(request);
    },
    onSuccess: () => {
      // Invalidate papers list to refetch
      queryClient.invalidateQueries({ queryKey: ['papers'] });
    },
  });
}

/**
 * Hook to update a paper
 */
export function useUpdatePaper() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      id,
      request,
    }: {
      id: string;
      request: UpdatePaperRequest;
    }) => {
      return await updatePaperApi(id, request);
    },
    onSuccess: (data, variables) => {
      // Invalidate both the specific paper and the papers list
      queryClient.invalidateQueries({ queryKey: ['papers', variables.id] });
      queryClient.invalidateQueries({ queryKey: ['papers'] });
    },
  });
}

/**
 * Hook to delete a paper
 */
export function useDeletePaper() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (id: string) => {
      await deletePaperApi(id);
    },
    onSuccess: (_, id) => {
      // Invalidate both the specific paper and the papers list
      queryClient.invalidateQueries({ queryKey: ['papers', id] });
      queryClient.invalidateQueries({ queryKey: ['papers'] });
    },
  });
}

export interface UsePaperStatusOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export function usePaperStatus(
  id: string | null,
  options: UsePaperStatusOptions = {},
) {
  const { enabled = true, refetchInterval } = options;

  const {
    data,
    isLoading,
    isError,
    error,
    refetch,
  } = useQuery({
    queryKey: ['papers', id, 'status'],
    queryFn: async () => {
      if (!id) {
        throw new Error('Paper ID is required');
      }
      const response = await getPaperStatus(id);
      return response.data;
    },
    enabled: enabled && !!id,
    refetchInterval,
    staleTime: 10 * 1000,
  });

  return {
    status: data ?? null,
    isLoading,
    isError,
    error: error instanceof ApiClientError ? error : null,
    refetch,
  };
}

export interface UsePaperSegmentsOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export function usePaperSegments(
  id: string | null,
  options: UsePaperSegmentsOptions = {},
) {
  const { enabled = true, refetchInterval } = options;

  const {
    data,
    isLoading,
    isError,
    error,
    refetch,
  } = useQuery({
    queryKey: ['papers', id, 'segments'],
    queryFn: async () => {
      if (!id) {
        throw new Error('Paper ID is required');
      }
      const response = await getPaperSegments(id);
      return response.data;
    },
    enabled: enabled && !!id,
    refetchInterval,
    staleTime: 30 * 1000,
  });

  return {
    segments: (data ?? []) as PaperSegment[],
    isLoading,
    isError,
    error: error instanceof ApiClientError ? error : null,
    refetch,
  };
}
