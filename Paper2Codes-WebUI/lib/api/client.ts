/**
 * API client for Paper2Codes backend
 */

import { API_BASE_URL, API_TIMEOUT_MS } from '@/lib/config';
import type {
  ApiResponse,
  PaginatedResponse,
  ApiError,
  HealthResponse,
  Paper,
  Repository,
  Task,
  CreatePaperRequest,
  UpdatePaperRequest,
  PaginationQuery,
} from './types';
import { ApiClientError, isRetryableError, isNonRetryableError } from './errors';

export interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  body?: unknown;
  headers?: Record<string, string>;
  signal?: AbortSignal;
  retries?: number;
  params?: Record<string, string | number | boolean | undefined>;
  timeout?: number;
}

export class ApiClient {
  private baseURL: string;
  private defaultHeaders: Record<string, string>;
  private requestTimeoutMs: number;

  constructor(baseURL: string = API_BASE_URL, timeoutMs: number = API_TIMEOUT_MS) {
    // Ensure baseURL doesn't end with a slash
    this.baseURL = baseURL.replace(/\/$/, '');
    this.defaultHeaders = {
      'Content-Type': 'application/json',
    };
    this.requestTimeoutMs = Number.isFinite(timeoutMs) && timeoutMs >= 0 ? timeoutMs : API_TIMEOUT_MS;
    
    // Log API base URL in development
    if (typeof window !== 'undefined' && process.env.NODE_ENV === 'development') {
      console.log('API Client initialized with base URL:', this.baseURL);
    }
  }

  /**
   * Make an API request with retry logic
   */
  async request<T>(
    endpoint: string,
    options: RequestOptions = {},
  ): Promise<ApiResponse<T>> {
    const {
      method = 'GET',
      body,
      headers = {},
      signal,
      retries = 3,
      params,
    } = options;

    // Build URL with query parameters
    const url = new URL(endpoint, this.baseURL);
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined && value !== null) {
          url.searchParams.append(key, String(value));
        }
      });
    }

    const requestHeaders = {
      ...this.defaultHeaders,
      ...headers,
      ...this.getAuthHeaders(),
    };

    let lastError: ApiClientError | null = null;

    // Retry loop
    for (let attempt = 0; attempt <= retries; attempt++) {
      const controller = new AbortController();
      const timeoutMs = options.timeout ?? this.requestTimeoutMs;
      let timeoutId: ReturnType<typeof setTimeout> | undefined;
      let didTimeout = false;

      const abortHandler = () => controller.abort();
      if (signal) {
        if (signal.aborted) {
          controller.abort();
        } else {
          signal.addEventListener('abort', abortHandler);
        }
      }

      if (timeoutMs > 0) {
        timeoutId = setTimeout(() => {
          didTimeout = true;
          controller.abort();
        }, timeoutMs);
      }

      try {
        const response = await fetch(url.toString(), {
          method,
          headers: requestHeaders,
          body: body ? JSON.stringify(body) : undefined,
          signal: controller.signal,
        });

        const contentType = response.headers.get('content-type');
        const isJson = contentType?.includes('application/json');

        let rawBody = '';
        try {
          rawBody = await response.text();
        } catch {
          rawBody = '';
        }

        let data: unknown;

        if (isJson) {
          try {
            data = rawBody ? JSON.parse(rawBody) : {};
          } catch (parseError) {
            data = {
              error: {
                code: 'PARSE_ERROR',
                message:
                  rawBody || response.statusText || 'Failed to parse response',
                details: {
                  status: response.status,
                  statusText: response.statusText,
                  parseError:
                    parseError instanceof Error
                      ? parseError.message
                      : 'Unknown parse error',
                },
              },
              meta: {
                request_id: response.headers.get('x-request-id') || '',
              },
            };
          }
        } else {
          data = {
            error: {
              code: `HTTP_${response.status}`,
              message:
                rawBody || response.statusText || 'Request failed',
              details: {
                status: response.status,
                statusText: response.statusText,
              },
            },
            meta: {
              request_id: response.headers.get('x-request-id') || '',
            },
          };
        }

        const responseRequestId =
          (typeof data === 'object' &&
            data !== null &&
            'meta' in data &&
            (data as any).meta?.request_id) ||
          response.headers.get('x-request-id') ||
          undefined;

        if (!response.ok) {
          const errorData = data as ApiError;
          let error: ApiClientError;

          if (errorData?.error) {
            error = ApiClientError.fromResponse(
              errorData,
              response.status,
              responseRequestId,
            );
          } else {
            error = new ApiClientError(
              `HTTP_${response.status}`,
              (data as any)?.message ||
                response.statusText ||
                'Request failed',
              response.status,
              {
                data,
                statusText: response.statusText,
              },
              responseRequestId,
            );
          }

          lastError = error;

          if (response.status === 401) {
            throw error;
          }

          if (
            isNonRetryableError(error) ||
            attempt === retries ||
            !isRetryableError(error)
          ) {
            throw error;
          }

          await this.delay(Math.pow(2, attempt) * 1000);
          continue;
        }

        if (!data || typeof data !== 'object') {
          throw new ApiClientError(
            'INVALID_RESPONSE',
            'Response is not a valid object',
            response.status,
            { data },
            responseRequestId,
          );
        }

        if ('error' in data && (data as any).success === false) {
          const errorData = data as any;
          throw new ApiClientError(
            errorData.error?.code || 'API_ERROR',
            errorData.error?.message || 'API request failed',
            response.status,
            errorData,
            responseRequestId,
          );
        }

        const normalizedData = data as any;
        const meta = normalizedData.meta ?? {
          timestamp: new Date().toISOString(),
          request_id: responseRequestId || '',
        };

        if (normalizedData.data !== undefined && normalizedData.success !== undefined) {
          return {
            ...normalizedData,
            meta: {
              ...meta,
              request_id: meta.request_id || responseRequestId || '',
            },
          } as ApiResponse<T>;
        } else if (normalizedData.data !== undefined) {
          return {
            success: true,
            data: normalizedData.data as T,
            meta: {
              ...meta,
              request_id: meta.request_id || responseRequestId || '',
            },
          } as ApiResponse<T>;
        } else {
          return {
            success: true,
            data: normalizedData as T,
            meta: {
              ...meta,
              request_id: meta.request_id || responseRequestId || '',
            },
          } as ApiResponse<T>;
        }
      } catch (error) {
        if (error instanceof ApiClientError) {
          lastError = error;
          if (attempt === retries || isNonRetryableError(error)) {
            throw error;
          }
          await this.delay(Math.pow(2, attempt) * 1000);
          continue;
        }

        if (error instanceof Error && error.name === 'AbortError') {
          const abortError = didTimeout
            ? new ApiClientError(
                'TIMEOUT',
                `Request to ${url.pathname} timed out after ${timeoutMs}ms`,
                408,
                { endpoint: url.pathname, baseUrl: this.baseURL, timeoutMs },
              )
            : new ApiClientError(
                'REQUEST_ABORTED',
                'Request was cancelled',
                undefined,
                { endpoint: url.pathname },
              );
          lastError = abortError;
          throw abortError;
        }

        if (error instanceof TypeError && error.message === 'Failed to fetch') {
          const networkError = new ApiClientError(
            'NETWORK_ERROR',
            `Cannot connect to backend at ${this.baseURL}. Please ensure the Paper2Codes Core service is running.`,
            0,
            { originalError: error.message, endpoint: url.pathname },
          );
          lastError = networkError;

          if (attempt === retries) {
            throw networkError;
          }
          await this.delay(Math.pow(2, attempt) * 1000);
          continue;
        }

        const apiError =
          error instanceof Error
            ? ApiClientError.fromNetworkError(error)
            : ApiClientError.fromNetworkError(
                new Error('Unknown error occurred'),
              );
        lastError = apiError;

        if (attempt === retries || isNonRetryableError(apiError)) {
          throw apiError;
        }

        await this.delay(Math.pow(2, attempt) * 1000);
      } finally {
        if (timeoutId) {
          clearTimeout(timeoutId);
        }
        if (signal) {
          signal.removeEventListener('abort', abortHandler);
        }
      }
    }

    throw lastError || new ApiClientError('UNKNOWN_ERROR', 'Request failed');
  }

  /**
   * GET request
   */
  async get<T>(endpoint: string, options?: RequestOptions): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, { ...options, method: 'GET' });
  }

  /**
   * POST request
   */
  async post<T>(
    endpoint: string,
    body?: unknown,
    options?: RequestOptions,
  ): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, { ...options, method: 'POST', body });
  }

  /**
   * PUT request
   */
  async put<T>(
    endpoint: string,
    body?: unknown,
    options?: RequestOptions,
  ): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, { ...options, method: 'PUT', body });
  }

  /**
   * DELETE request
   */
  async delete<T>(endpoint: string, options?: RequestOptions): Promise<ApiResponse<T>> {
    return this.request<T>(endpoint, { ...options, method: 'DELETE' });
  }

  /**
   * Get authentication headers
   */
  private getAuthHeaders(): Record<string, string> {
    // TODO: Implement authentication token retrieval
    const token = this.getAuthToken();
    return token ? { Authorization: `Bearer ${token}` } : {};
  }

  /**
   * Get authentication token
   */
  private getAuthToken(): string {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('auth_token') || '';
    }
    return '';
  }

  /**
   * Delay helper for retry logic
   */
  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  // Health check
  async health(): Promise<ApiResponse<HealthResponse>> {
    return this.get<HealthResponse>('/api/health');
  }

  // Papers endpoints
  async listPapers(
    pagination?: PaginationQuery,
  ): Promise<PaginatedResponse<Paper>> {
    const response = await this.get<Paper[]>('/api/papers', {
      params: pagination as Record<string, string | number | undefined>,
    });
    
    // Handle both direct array responses and paginated responses
    if (Array.isArray(response.data)) {
      const page = pagination?.page ?? 1;
      const perPage = pagination?.per_page ?? 10;
      const total = response.data.length;
      const totalPages = Math.max(1, Math.ceil(total / perPage));
      return {
        success: response.success ?? true,
        data: response.data,
        pagination: {
          page,
          per_page: perPage,
          total,
          total_pages: totalPages,
          has_more: page < totalPages,
          next_cursor: undefined,
          prev_cursor: undefined,
        },
        meta: response.meta ?? {
          timestamp: new Date().toISOString(),
          request_id: '',
        },
      };
    }
    
    return response as PaginatedResponse<Paper>;
  }

  async getPaper(id: string): Promise<ApiResponse<Paper>> {
    return this.get<Paper>(`/api/papers/${id}`);
  }

  async createPaper(
    request: CreatePaperRequest,
  ): Promise<ApiResponse<Paper>> {
    return this.post<Paper>('/api/papers', request);
  }

  async updatePaper(
    id: string,
    request: UpdatePaperRequest,
  ): Promise<ApiResponse<Paper>> {
    return this.put<Paper>(`/api/papers/${id}`, request);
  }

  async deletePaper(id: string): Promise<void> {
    await this.delete(`/api/papers/${id}`);
  }

  // Repositories endpoints
  async listRepositories(
    pagination?: PaginationQuery,
  ): Promise<PaginatedResponse<Repository>> {
    const response = await this.get<Repository[]>('/api/repositories', {
      params: pagination as Record<string, string | number | undefined>,
    });
    
    // Handle both direct array responses and paginated responses
    if (Array.isArray(response.data)) {
      const page = pagination?.page ?? 1;
      const perPage = pagination?.per_page ?? 10;
      const total = response.data.length;
      const totalPages = Math.max(1, Math.ceil(total / perPage));
      return {
        success: response.success ?? true,
        data: response.data,
        pagination: {
          page,
          per_page: perPage,
          total,
          total_pages: totalPages,
          has_more: page < totalPages,
          next_cursor: undefined,
          prev_cursor: undefined,
        },
        meta: response.meta ?? {
          timestamp: new Date().toISOString(),
          request_id: '',
        },
      };
    }
    
    return response as PaginatedResponse<Repository>;
  }

  async getRepository(id: string): Promise<ApiResponse<Repository>> {
    return this.get<Repository>(`/api/repositories/${id}`);
  }

  // Tasks endpoints
  async listTasks(pagination?: PaginationQuery): Promise<PaginatedResponse<Task>> {
    const response = await this.get<Task[]>('/api/tasks', {
      params: pagination as Record<string, string | number | undefined>,
    });
    
    // Handle both direct array responses and paginated responses
    if (Array.isArray(response.data)) {
      const page = pagination?.page ?? 1;
      const perPage = pagination?.per_page ?? 10;
      const total = response.data.length;
      const totalPages = Math.max(1, Math.ceil(total / perPage));
      return {
        success: response.success ?? true,
        data: response.data,
        pagination: {
          page,
          per_page: perPage,
          total,
          total_pages: totalPages,
          has_more: page < totalPages,
          next_cursor: undefined,
          prev_cursor: undefined,
        },
        meta: response.meta ?? {
          timestamp: new Date().toISOString(),
          request_id: '',
        },
      };
    }
    
    return response as PaginatedResponse<Task>;
  }

  async getTask(id: string): Promise<ApiResponse<Task>> {
    return this.get<Task>(`/api/tasks/${id}`);
  }

  async createTask(taskData: {
    task_type: string;
    description: string;
    paper_id?: string;
    module_id?: string;
  }): Promise<ApiResponse<Task>> {
    return this.post<Task>('/api/tasks', taskData);
  }

  async updateTaskStatus(
    taskId: string,
    status: string,
  ): Promise<ApiResponse<Task>> {
    return this.put<Task>(`/api/tasks/${taskId}/status`, { status });
  }

  async cancelTask(taskId: string): Promise<ApiResponse<Task>> {
    return this.post<Task>(`/api/tasks/${taskId}/cancel`, {});
  }

  private uploadMultipart<T>(
    endpoint: string,
    formData: FormData,
    onProgress?: (progress: number) => void,
  ): Promise<ApiResponse<T>> {
    const url = new URL(endpoint, this.baseURL);
    const token = this.getAuthToken();

    return new Promise((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.withCredentials = true;

      xhr.upload.addEventListener('progress', (event) => {
        if (event.lengthComputable && onProgress) {
          const progress = (event.loaded / event.total) * 100;
          onProgress(progress);
        }
      });

      const handleNetworkError = (code: string, message: string) => {
        reject(new ApiClientError(code, message, xhr.status || 0));
      };

      xhr.addEventListener('error', () => {
        handleNetworkError('UPLOAD_NETWORK_ERROR', 'Network error during upload');
      });

      xhr.addEventListener('abort', () => {
        handleNetworkError('UPLOAD_ABORTED', 'Upload was aborted');
      });

      xhr.addEventListener('load', () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          try {
            const parsed = xhr.responseText ? JSON.parse(xhr.responseText) : {};
            if (parsed && typeof parsed === 'object') {
              resolve(parsed as ApiResponse<T>);
              return;
            }
            reject(
              new ApiClientError(
                'UPLOAD_INVALID_RESPONSE',
                'Upload response was not a valid object',
                xhr.status,
                { response: xhr.responseText },
              ),
            );
          } catch (err) {
            reject(
              new ApiClientError(
                'UPLOAD_PARSE_ERROR',
                'Failed to parse upload response',
                xhr.status,
                {
                  response: xhr.responseText,
                  parseError:
                    err instanceof Error ? err.message : 'Unknown parse error',
                },
              ),
            );
          }
        } else {
          try {
            const errorPayload = xhr.responseText ? JSON.parse(xhr.responseText) : null;
            if (errorPayload) {
              reject(ApiClientError.fromResponse(errorPayload, xhr.status));
            } else {
              reject(
                new ApiClientError(
                  'UPLOAD_FAILED',
                  'Upload failed',
                  xhr.status,
                  { statusText: xhr.statusText },
                ),
              );
            }
          } catch {
            reject(
              new ApiClientError(
                'UPLOAD_FAILED',
                'Upload failed',
                xhr.status,
                { statusText: xhr.statusText },
              ),
            );
          }
        }
      });

      xhr.open('POST', url.toString());
      if (token) {
        xhr.setRequestHeader('Authorization', `Bearer ${token}`);
      }
      xhr.send(formData);
    });
  }

  // Phase 5: File management endpoints
  async uploadFile(
    file: File,
    metadata?: {
      name?: string;
      description?: string;
      tags?: string[];
    },
    onProgress?: (progress: number) => void,
  ): Promise<ApiResponse<{
    file_id: string;
    name: string;
    size: number;
    content_type: string;
    uploaded_at: string;
  }>> {
    const formData = new FormData();
    formData.append('file', file);

    if (metadata?.name) {
      formData.append('name', metadata.name);
    }
    if (metadata?.description) {
      formData.append('description', metadata.description);
    }
    if (metadata?.tags && metadata.tags.length > 0) {
      formData.append('tags', metadata.tags.join(','));
    }

    return this.uploadMultipart('/api/files/upload', formData, onProgress);
  }

  async uploadPaperFile(
    formData: FormData,
    onProgress?: (progress: number) => void,
  ): Promise<ApiResponse<Paper>> {
    return this.uploadMultipart('/api/papers/upload', formData, onProgress);
  }

  async downloadFile(fileId: string): Promise<Blob> {
    const url = new URL(`/api/files/${fileId}`, this.baseURL);
    const token = this.getAuthToken();
    
    const headers: Record<string, string> = {};
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(url.toString(), { headers });
    if (!response.ok) {
      throw new ApiClientError(
        'DOWNLOAD_FAILED',
        'Download failed',
        response.status,
        { statusText: response.statusText },
      );
    }
    
    return response.blob();
  }

  async getFileMetadata(fileId: string): Promise<ApiResponse<{
    id: string;
    name: string;
    size: number;
    content_type: string;
    description?: string;
    tags: string[];
    versions: Array<{
      version: number;
      size: number;
      created_at: string;
      checksum: string;
    }>;
    created_at: string;
    updated_at: string;
  }>> {
    return this.get(`/api/files/${fileId}/metadata`);
  }

  // Phase 5: Advanced search endpoints
  async search(request: {
    query: string;
    filters?: {
      paper_ids?: string[];
      domains?: string[];
      authors?: string[];
      date_from?: string;
      date_to?: string;
      status?: string[];
      tags?: string[];
    };
    sort?: 'relevance' | 'date_desc' | 'date_asc' | 'title';
    page?: number;
    per_page?: number;
  }): Promise<ApiResponse<{
    results: Array<{
      id: string;
      title: string;
      content: string;
      score: number;
      paper_id?: string;
      paper_title?: string;
      metadata: Record<string, unknown>;
    }>;
    facets: {
      domains: Array<{ value: string; count: number }>;
      authors: Array<{ value: string; count: number }>;
      years: Array<{ value: string; count: number }>;
      statuses: Array<{ value: string; count: number }>;
    };
    total: number;
  }>> {
    return this.post('/api/search', request);
  }

  async getSearchSuggestions(query: string): Promise<ApiResponse<Array<{
    text: string;
    count: number;
  }>>> {
    return this.get('/api/search/suggestions', {
      params: { q: query },
    });
  }

  async getSearchFacets(query?: string): Promise<ApiResponse<{
    domains: Array<{ value: string; count: number }>;
    authors: Array<{ value: string; count: number }>;
    years: Array<{ value: string; count: number }>;
    statuses: Array<{ value: string; count: number }>;
  }>> {
    return this.get('/api/search/facets', {
      params: query ? { query } : {},
    });
  }

  // Phase 5: Analytics endpoints
  async getAnalyticsOverview(): Promise<ApiResponse<{
    total_papers: number;
    total_repositories: number;
    total_tasks: number;
    active_tasks: number;
    completed_tasks: number;
    failed_tasks: number;
    total_modules: number;
    total_verifications: number;
    average_generation_time_seconds: number;
    average_verification_time_seconds: number;
    success_rate: number;
  }>> {
    return this.get('/api/analytics/overview');
  }

  async getPerformanceMetrics(range?: string): Promise<ApiResponse<{
    api_response_times: {
      p50_ms: number;
      p95_ms: number;
      p99_ms: number;
      average_ms: number;
      min_ms: number;
      max_ms: number;
    };
    database_query_times: {
      p50_ms: number;
      p95_ms: number;
      p99_ms: number;
      average_ms: number;
      min_ms: number;
      max_ms: number;
    };
    llm_request_times: {
      p50_ms: number;
      p95_ms: number;
      p99_ms: number;
      average_ms: number;
      min_ms: number;
      max_ms: number;
    };
    task_execution_times: {
      planning_avg_seconds: number;
      analysis_avg_seconds: number;
      coding_avg_seconds: number;
      verification_avg_seconds: number;
      total_avg_seconds: number;
    };
    cache_hit_rate: number;
    throughput: {
      requests_per_second: number;
      tasks_per_hour: number;
      papers_processed_per_day: number;
    };
  }>> {
    return this.get('/api/analytics/performance', {
      params: range ? { range } : {},
    });
  }

  async getUsageStatistics(period?: string): Promise<ApiResponse<{
    period: string;
    total_requests: number;
    total_tasks_created: number;
    total_papers_uploaded: number;
    total_repositories_generated: number;
    llm_requests: number;
    llm_tokens_used: number;
    storage_used_bytes: number;
    daily_breakdown: Array<{
      date: string;
      requests: number;
      tasks: number;
      papers: number;
      repositories: number;
    }>;
  }>> {
    return this.get('/api/analytics/usage', {
      params: period ? { period } : {},
    });
  }

  async getAgentPerformance(range?: string): Promise<ApiResponse<{
    planning_agent: {
      total_executions: number;
      successful_executions: number;
      failed_executions: number;
      average_execution_time_seconds: number;
      average_tokens_used: number;
      average_cost_usd: number;
      success_rate: number;
    };
    analysis_agent: {
      total_executions: number;
      successful_executions: number;
      failed_executions: number;
      average_execution_time_seconds: number;
      average_tokens_used: number;
      average_cost_usd: number;
      success_rate: number;
    };
    coding_agent: {
      total_executions: number;
      successful_executions: number;
      failed_executions: number;
      average_execution_time_seconds: number;
      average_tokens_used: number;
      average_cost_usd: number;
      success_rate: number;
    };
    verification_agent: {
      total_executions: number;
      successful_executions: number;
      failed_executions: number;
      average_execution_time_seconds: number;
      average_tokens_used: number;
      average_cost_usd: number;
      success_rate: number;
    };
  }>> {
    return this.get('/api/analytics/agents', {
      params: range ? { range } : {},
    });
  }
}

// Export singleton instance
export const apiClient = new ApiClient();
