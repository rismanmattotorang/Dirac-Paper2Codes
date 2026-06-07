/**
 * React hook for API calls with loading and error states
 */

import { useState, useEffect, useCallback } from 'react';
import { apiClient, type RequestOptions } from '@/lib/api/client';
import { ApiClientError } from '@/lib/api/errors';

export interface UseApiReturn<T> {
  data: T | null;
  loading: boolean;
  error: ApiClientError | null;
  refetch: () => Promise<void>;
}

export function useApi<T>(
  endpoint: string | null,
  options?: RequestOptions,
): UseApiReturn<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<ApiClientError | null>(null);

  const fetchData = useCallback(async () => {
    if (!endpoint) {
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const response = await apiClient.get<T>(endpoint, options);
      setData(response.data);
    } catch (err) {
      const apiError =
        err instanceof ApiClientError
          ? err
          : new ApiClientError(
              'UNKNOWN_ERROR',
              err instanceof Error ? err.message : 'Unknown error occurred',
            );
      setError(apiError);
    } finally {
      setLoading(false);
    }
  }, [endpoint, options]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  return { data, loading, error, refetch: fetchData };
}
