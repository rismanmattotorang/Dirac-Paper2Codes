/**
 * Analytics API client
 */

import { apiClient } from './client';
import { API_BASE_URL } from '@/lib/config';
import { ApiClientError } from './errors';
import type { ApiResponse } from './types';

export interface AnalyticsOverview {
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
}

export interface PerformanceMetrics {
  api_response_times: ResponseTimeMetrics;
  database_query_times: ResponseTimeMetrics;
  llm_request_times: ResponseTimeMetrics;
  task_execution_times: TaskExecutionMetrics;
  cache_hit_rate: number;
  throughput: ThroughputMetrics;
}

export interface ResponseTimeMetrics {
  p50_ms: number;
  p95_ms: number;
  p99_ms: number;
  average_ms: number;
  min_ms: number;
  max_ms: number;
}

export interface TaskExecutionMetrics {
  planning_avg_seconds: number;
  analysis_avg_seconds: number;
  coding_avg_seconds: number;
  verification_avg_seconds: number;
  total_avg_seconds: number;
}

export interface ThroughputMetrics {
  requests_per_second: number;
  tasks_per_hour: number;
  papers_processed_per_day: number;
}

export interface UsageStatistics {
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
}

export interface AgentPerformanceMetrics {
  planning_agent: AgentMetrics;
  analysis_agent: AgentMetrics;
  coding_agent: AgentMetrics;
  verification_agent: AgentMetrics;
}

export interface AgentMetrics {
  total_executions: number;
  successful_executions: number;
  failed_executions: number;
  average_execution_time_seconds: number;
  average_tokens_used: number;
  average_cost_usd: number;
  success_rate: number;
}

/**
 * Get analytics overview
 */
export async function getAnalyticsOverview(): Promise<
  ApiResponse<AnalyticsOverview>
> {
  return apiClient.getAnalyticsOverview();
}

/**
 * Get performance metrics
 */
export async function getPerformanceMetrics(
  range?: string
): Promise<ApiResponse<PerformanceMetrics>> {
  return apiClient.getPerformanceMetrics(range);
}

/**
 * Get usage statistics
 */
export async function getUsageStatistics(
  period?: string
): Promise<ApiResponse<UsageStatistics>> {
  return apiClient.getUsageStatistics(period);
}

/**
 * Get agent performance metrics
 */
export async function getAgentPerformance(
  range?: string
): Promise<ApiResponse<AgentPerformanceMetrics>> {
  return apiClient.getAgentPerformance(range);
}

/**
 * Get time series data for charts
 */
export async function getTimeSeriesData(
  metric: 'tasks' | 'papers' | 'requests' | 'llm_usage',
  range: '24h' | '7d' | '30d' | '90d' = '7d'
): Promise<
  ApiResponse<
    Array<{
      timestamp: string;
      value: number;
      metadata?: Record<string, unknown>;
    }>
  >
> {
  return apiClient.get('/api/analytics/timeseries', {
    params: { metric, range },
  });
}

/**
 * Get domain distribution
 */
export async function getDomainDistribution(): Promise<
  ApiResponse<
    Array<{
      domain: string;
      count: number;
      percentage: number;
    }>
  >
> {
  return apiClient.get('/api/analytics/domains');
}

/**
 * Get LLM usage breakdown
 */
export async function getLLMUsageBreakdown(): Promise<
  ApiResponse<
    Array<{
      model: string;
      requests: number;
      tokens: number;
      cost_usd: number;
      percentage: number;
    }>
  >
> {
  return apiClient.get('/api/analytics/llm-usage');
}

/**
 * Export analytics report
 */
export async function exportAnalyticsReport(
  format: 'csv' | 'json' | 'pdf',
  range?: string
): Promise<Blob> {
  if (typeof window === 'undefined') {
    throw new ApiClientError(
      'UNSUPPORTED_ENVIRONMENT',
      'Analytics export is only supported in the browser',
    );
  }

  const url = new URL('/api/analytics/export', API_BASE_URL);
  url.searchParams.append('format', format);
  if (range) {
    url.searchParams.append('range', range);
  }

  const token = localStorage.getItem('auth_token');
  const headers: Record<string, string> = {};
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const response = await fetch(url.toString(), {
    headers,
    credentials: 'include',
  });
  if (!response.ok) {
    const errorText = await response.text().catch(() => '');
    throw new ApiClientError(
      'EXPORT_FAILED',
      'Failed to export analytics report',
      response.status,
      { endpoint: url.pathname, responseBody: errorText || undefined },
    );
  }

  return response.blob();
}
