/**
 * Paper-related API helpers
 */

import { apiClient } from './client';
import type {
  ApiResponse,
  PaginatedResponse,
  Paper,
  PaperSegment,
  PaginationQuery,
  ProcessingStatusResponse,
  SearchResult,
} from './types';

export interface UploadPaperOptions {
  name?: string;
  description?: string;
  tags?: string[];
  onProgress?: (progress: number) => void;
}

/**
 * List papers with pagination support
 */
export async function listPapers(
  pagination?: PaginationQuery,
): Promise<PaginatedResponse<Paper>> {
  return apiClient.listPapers(pagination);
}

/**
 * Retrieve a single paper by id
 */
export async function getPaper(id: string): Promise<ApiResponse<Paper>> {
  return apiClient.getPaper(id);
}

/**
 * Create a paper manually
 */
export async function createPaper(
  request: Parameters<typeof apiClient.createPaper>[0],
): Promise<ApiResponse<Paper>> {
  return apiClient.createPaper(request);
}

/**
 * Update paper metadata
 */
export async function updatePaper(
  id: string,
  request: Parameters<typeof apiClient.updatePaper>[1],
): Promise<ApiResponse<Paper>> {
  return apiClient.updatePaper(id, request);
}

/**
 * Delete a paper
 */
export async function deletePaper(id: string): Promise<void> {
  return apiClient.deletePaper(id);
}

/**
 * Upload a paper (PDF or text). Returns the stored paper entity.
 */
export async function uploadPaper(
  file: File,
  options: UploadPaperOptions = {},
): Promise<ApiResponse<Paper>> {
  const formData = new FormData();
  formData.append('file', file);

  if (options.name) {
    formData.append('name', options.name);
  }
  if (options.description) {
    formData.append('description', options.description);
  }
  if (options.tags && options.tags.length > 0) {
    formData.append('tags', options.tags.join(','));
  }

  return apiClient.uploadPaperFile(formData, options.onProgress);
}

/**
 * Begin processing a paper (segmentation, extraction, embeddings) and, when a
 * domain skill is selected and LLM keys are configured, skill-guided code
 * generation. The selected skill + target language are forwarded to the backend.
 */
export async function processPaper(
  paperId: string,
  options?: { skillId?: string; language?: string },
): Promise<ApiResponse<ProcessingStatusResponse>> {
  const params = new URLSearchParams();
  if (options?.skillId) params.set('skill_id', options.skillId);
  if (options?.language) params.set('language', options.language);
  const query = params.toString();
  const path = `/api/papers/${paperId}/process${query ? `?${query}` : ''}`;
  return apiClient.post<ProcessingStatusResponse>(path);
}

/**
 * Get processing status for a paper.
 */
export async function getPaperStatus(
  paperId: string,
): Promise<ApiResponse<ProcessingStatusResponse>> {
  return apiClient.get<ProcessingStatusResponse>(`/api/papers/${paperId}/status`);
}

/**
 * Retrieve the segments for a paper.
 */
export async function getPaperSegments(
  paperId: string,
): Promise<ApiResponse<PaperSegment[]>> {
  return apiClient.get<PaperSegment[]>(`/api/papers/${paperId}/segments`);
}

/**
 * Perform semantic search across papers.
 */
export async function searchPapers(
  request: {
    query: string;
    k?: number;
    paper_id?: string;
    segment_type?: string;
  },
): Promise<ApiResponse<SearchResult[]>> {
  return apiClient.post<SearchResult[]>('/api/papers/search', request);
}
