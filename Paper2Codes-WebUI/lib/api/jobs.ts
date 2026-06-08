/**
 * Durable generation jobs — read-only inspection + cancel.
 */

import { apiClient } from './client';

export type JobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface JobInfo {
  id: string;
  kind: string;
  paper_id: string | null;
  status: JobStatus;
  progress: number;
  attempts: number;
  max_attempts: number;
  last_error: string | null;
  created_at: string;
  updated_at: string;
}

export const jobsApi = {
  async list(filters?: { paperId?: string; status?: JobStatus }): Promise<JobInfo[]> {
    const params: Record<string, string> = {};
    if (filters?.paperId) params.paper_id = filters.paperId;
    if (filters?.status) params.status = filters.status;
    const res = await apiClient.get<JobInfo[]>('/api/jobs', { params });
    return (res.data as JobInfo[]) ?? [];
  },

  async get(id: string): Promise<JobInfo> {
    const res = await apiClient.get<JobInfo>(`/api/jobs/${id}`);
    return res.data as JobInfo;
  },

  async cancel(id: string): Promise<JobInfo> {
    const res = await apiClient.post<JobInfo>(`/api/jobs/${id}/cancel`, {});
    return res.data as JobInfo;
  },
};
