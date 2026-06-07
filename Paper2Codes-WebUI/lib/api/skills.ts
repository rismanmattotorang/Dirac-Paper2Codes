import { apiClient } from './client'

/**
 * A domain-specialisation Skill (mirrors the backend `skills::Skill`).
 * Skills can be chosen, reused, and improved by users.
 */
export interface Skill {
  id: string
  name: string
  version: string
  description: string
  domain?: string
  keywords: string[]
  languages: string[]
  libraries: Record<string, string[]>
  instructions: string
  verification_hints: string[]
  builtin: boolean
}

export const skillsApi = {
  /** List all available skills (built-in + user). */
  list: async (): Promise<Skill[]> => {
    const response = await apiClient.get<Skill[]>('/api/skills')
    return Array.isArray(response.data) ? response.data : []
  },

  /** Fetch a single skill by id. */
  get: async (id: string): Promise<Skill> => {
    const response = await apiClient.get<Skill>(`/api/skills/${encodeURIComponent(id)}`)
    return response.data
  },

  /** Create or improve a user skill. */
  upsert: async (skill: Skill): Promise<Skill> => {
    const response = await apiClient.put<Skill>(
      `/api/skills/${encodeURIComponent(skill.id)}`,
      skill,
    )
    return response.data
  },
}

/** Recommended libraries for a skill in a given language (case-insensitive). */
export function librariesFor(skill: Skill, language: string): string[] {
  const key = Object.keys(skill.libraries).find(
    (k) => k.toLowerCase() === language.toLowerCase(),
  )
  return key ? skill.libraries[key] : []
}
