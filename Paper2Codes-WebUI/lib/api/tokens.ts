/**
 * Personal API token management (programmatic access).
 */

import { apiClient } from './client';

export interface ApiTokenInfo {
  id: string;
  name: string;
  prefix: string;
  created_at: string;
  last_used_at: string | null;
  expires_at: string | null;
}

export interface CreatedToken extends ApiTokenInfo {
  /** Plaintext token — shown only once, on creation. */
  token: string;
}

export const tokensApi = {
  async list(): Promise<ApiTokenInfo[]> {
    const res = await apiClient.get<ApiTokenInfo[]>('/api/auth/tokens');
    return (res.data as ApiTokenInfo[]) ?? [];
  },

  async create(name: string, expiresInDays?: number): Promise<CreatedToken> {
    const res = await apiClient.post<CreatedToken>('/api/auth/tokens', {
      name,
      expires_in_days: expiresInDays,
    });
    return res.data as CreatedToken;
  },

  async revoke(id: string): Promise<void> {
    await apiClient.delete(`/api/auth/tokens/${id}`);
  },
};
