/**
 * Repository and code module API client
 */

import { apiClient } from './client';
import type { ApiResponse, PaginatedResponse, Repository, CodeModule } from './types';
import { API_BASE_URL } from '@/lib/config';
import { ApiClientError } from './errors';

export interface ModuleContent {
  id: string;
  repository_id?: string | null;
  file_path: string;
  content: string;
  language: string;
  ast?: string;
  dependencies: string[];
  tests: Array<{
    id: string;
    name: string;
    code: string;
    expected_output?: string;
  }>;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface ModuleUpdate {
  content: string;
  description?: string;
}

export interface DependencyGraph {
  nodes: Array<{
    id: string;
    name: string;
    file_path: string;
    type: 'module' | 'package';
  }>;
  edges: Array<{
    from: string;
    to: string;
    type: 'imports' | 'uses';
  }>;
}

/**
 * Get list of repositories
 */
export async function getRepositories(
  page: number = 1,
  per_page: number = 20
): Promise<PaginatedResponse<Repository>> {
  return apiClient.listRepositories({ page, per_page });
}

/**
 * Get repository by ID
 */
export async function getRepository(id: string): Promise<ApiResponse<Repository>> {
  return apiClient.getRepository(id);
}

/**
 * Get repository modules
 */
export async function getRepositoryModules(
  repositoryId: string
): Promise<ApiResponse<CodeModule[]>> {
  return apiClient.get(`/api/repositories/${repositoryId}/modules`);
}

/**
 * Get module content
 */
export async function getModuleContent(
  moduleId: string
): Promise<ApiResponse<ModuleContent>> {
  return apiClient.get(`/api/modules/${moduleId}`);
}

/**
 * Update module content
 */
export async function updateModuleContent(
  moduleId: string,
  update: ModuleUpdate
): Promise<ApiResponse<ModuleContent>> {
  return apiClient.put(`/api/modules/${moduleId}`, update);
}

/**
 * Download repository as ZIP
 */
export async function downloadRepository(repositoryId: string): Promise<Blob> {
  if (typeof window === 'undefined') {
    throw new ApiClientError(
      'UNSUPPORTED_ENVIRONMENT',
      'Repository downloads are only supported in the browser',
    );
  }

  const url = new URL(`/api/repositories/${repositoryId}/download`, API_BASE_URL);
  const headers: Record<string, string> = {};
  const token = localStorage.getItem('auth_token');

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
      'DOWNLOAD_FAILED',
      'Failed to download repository archive',
      response.status,
      { endpoint: url.pathname, responseBody: errorText || undefined },
    );
  }

  return response.blob();
}

/**
 * Download module file
 */
export async function downloadModule(moduleId: string): Promise<Blob> {
  if (typeof window === 'undefined') {
    throw new ApiClientError(
      'UNSUPPORTED_ENVIRONMENT',
      'Module downloads are only supported in the browser',
    );
  }

  const url = new URL(`/api/modules/${moduleId}/download`, API_BASE_URL);
  const headers: Record<string, string> = {};
  const token = localStorage.getItem('auth_token');

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
      'DOWNLOAD_FAILED',
      'Failed to download module source',
      response.status,
      { endpoint: url.pathname, responseBody: errorText || undefined },
    );
  }

  return response.blob();
}

/**
 * Get dependency graph for repository
 */
export async function getDependencyGraph(
  repositoryId: string
): Promise<ApiResponse<DependencyGraph>> {
  return apiClient.get(`/api/repositories/${repositoryId}/graph`);
}

/**
 * Search modules by query
 */
export async function searchModules(
  query: string,
  repositoryId?: string
): Promise<
  ApiResponse<
    Array<{
      id: string;
      file_path: string;
      language: string;
      score: number;
      snippet: string;
    }>
  >
> {
  return apiClient.get('/api/modules/search', {
    params: {
      q: query,
      repository_id: repositoryId,
    },
  });
}

/**
 * Get module verification results
 */
export async function getModuleVerification(
  moduleId: string
): Promise<
  ApiResponse<{
    module_id: string;
    status: 'passed' | 'failed' | 'warning' | 'pending';
    tests_passed: number;
    tests_failed: number;
    tests_total: number;
    coverage: number;
    issues: Array<{
      severity: 'error' | 'warning' | 'info';
      message: string;
      line: number;
      column?: number;
    }>;
    executed_at: string;
  }>
> {
  return apiClient.get(`/api/modules/${moduleId}/verification`);
}
