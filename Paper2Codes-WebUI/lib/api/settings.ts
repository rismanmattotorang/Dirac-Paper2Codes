import { apiClient } from './client'
import { ApiClientError } from './errors'

// Types for settings
export interface SettingsResponse {
  general: GeneralSettings
  security: SecuritySettings
  notifications: NotificationSettings
  llm: LLMSettings
  database: DatabaseSettings
  team: TeamSettings
}

export interface GeneralSettings {
  organization_name: string
  default_domain: string
  dark_mode: boolean
  theme: string
}

export interface SecuritySettings {
  two_factor_enabled: boolean
  active_sessions: SessionInfo[]
  password_min_length: number
  jwt_expiration: number
  enable_csrf: boolean
}

export interface SessionInfo {
  id: string
  device: string
  ip: string
  last_active: string
  created_at: string
}

export interface NotificationSettings {
  paper_processing_complete: boolean
  code_generation_errors: boolean
  task_queue_updates: boolean
  weekly_report: boolean
}

export interface LLMSettings {
  primary_provider: string
  api_key_configured: boolean
  temperature: number
  model_preferences: ModelPreferences
  timeout_seconds: number
  max_retries: number
}

export interface ModelPreferences {
  planning: string
  analysis: string
  coding: string
  verification: string
}

export interface DatabaseSettings {
  connected: boolean
  connection_string: string
  namespace: string
  database: string
  max_connections: number
  backup_schedule: string
  last_sync?: string
  schema_valid: boolean
}

export interface TeamSettings {
  members: TeamMember[]
}

export interface TeamMember {
  id: string
  name: string
  email: string
  role: string
  status: string
}

// Update requests
export interface UpdateSettingsRequest {
  general?: Partial<GeneralSettings>
  security?: SecuritySettingsUpdate
  notifications?: Partial<NotificationSettings>
  llm?: LLMSettingsUpdate
  database?: DatabaseSettingsUpdate
}

export interface SecuritySettingsUpdate {
  password_min_length?: number
  jwt_expiration?: number
  enable_csrf?: boolean
}

export interface LLMSettingsUpdate {
  primary_provider?: string
  api_key?: string
  temperature?: number
  model_preferences?: Partial<ModelPreferences>
  timeout_seconds?: number
  max_retries?: number
}

export interface DatabaseSettingsUpdate {
  connection_string?: string
  namespace?: string
  database?: string
  max_connections?: number
  backup_schedule?: string
}

export interface Enable2FAResponse {
  qr_code: string
  secret: string
  backup_codes: string[]
}

export interface DatabaseConnectionTestResponse {
  success: boolean
  message: string
  latency_ms?: number
  schema_valid?: boolean
}

export interface AddTeamMemberRequest {
  name: string
  email: string
  role: string
}

export interface UpdateTeamMemberRoleRequest {
  role: string
}

// Settings API functions
const isSettingsResponse = (value: unknown): value is SettingsResponse => {
  if (!value || typeof value !== 'object') {
    return false
  }

  const candidate = value as Partial<SettingsResponse>
  return (
    typeof candidate.general === 'object' &&
    typeof candidate.security === 'object' &&
    typeof candidate.notifications === 'object' &&
    typeof candidate.llm === 'object' &&
    typeof candidate.database === 'object' &&
    typeof candidate.team === 'object'
  )
}

export const settingsApi = {
  // Get all settings
  getSettings: async (): Promise<SettingsResponse> => {
    try {
      const response = await apiClient.get<SettingsResponse>('/api/settings')
      if (response?.data) {
        return response.data
      }

      if (isSettingsResponse(response)) {
        return response
      }

      throw new ApiClientError(
        'INVALID_SETTINGS_RESPONSE',
        'Settings response did not match expected structure',
        undefined,
        { response }
      )
    } catch (error) {
      // Return default settings on error
      console.error('Failed to load settings:', error)
      throw error
    }
  },

  // Update settings
  updateSettings: async (request: UpdateSettingsRequest): Promise<SettingsResponse> => {
    const response = await apiClient.put<SettingsResponse>('/api/settings', request)
    return response.data
  },

  // Security operations
  revokeSession: async (sessionId: string): Promise<void> => {
    await apiClient.delete(`/api/settings/security/sessions/${sessionId}`)
  },

  enable2FA: async (): Promise<Enable2FAResponse> => {
    const response = await apiClient.post<Enable2FAResponse>('/api/settings/security/2fa/enable')
    return response.data
  },

  disable2FA: async (): Promise<void> => {
    await apiClient.post('/api/settings/security/2fa/disable')
  },

  // Database operations
  testDatabaseConnection: async (): Promise<DatabaseConnectionTestResponse> => {
    const response = await apiClient.post<DatabaseConnectionTestResponse>('/api/settings/database/test', undefined)
    // The API client normalizes responses, so the data should be in response.data
    // But handle both cases: wrapped response and direct response
    if (response.data) {
      return response.data
    }
    // If response is the data directly (backend returns it unwrapped)
    return response as unknown as DatabaseConnectionTestResponse
  },

  // Team operations
  addTeamMember: async (request: AddTeamMemberRequest): Promise<TeamMember> => {
    const response = await apiClient.post<TeamMember>('/api/settings/team/members', request)
    return response.data
  },

  removeTeamMember: async (memberId: string): Promise<void> => {
    await apiClient.delete(`/api/settings/team/members/${memberId}`)
  },

  updateTeamMemberRole: async (
    memberId: string,
    request: UpdateTeamMemberRoleRequest
  ): Promise<TeamMember> => {
    const response = await apiClient.put<TeamMember>(`/api/settings/team/members/${memberId}/role`, request)
    return response.data
  },
}
