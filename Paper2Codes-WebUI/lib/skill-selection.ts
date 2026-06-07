/**
 * Client-side persistence of the user's chosen domain skill + target language.
 * The paper-upload / generation flow reads this so a selected skill drives the
 * pipeline for the next paper.
 */

const KEY = 'p2c_selected_skill'

export interface SkillSelection {
  skillId: string
  language: string
}

export function getSelectedSkill(): SkillSelection | null {
  if (typeof window === 'undefined') return null
  try {
    const raw = window.localStorage.getItem(KEY)
    return raw ? (JSON.parse(raw) as SkillSelection) : null
  } catch {
    return null
  }
}

export function setSelectedSkill(selection: SkillSelection): void {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(KEY, JSON.stringify(selection))
}

export function clearSelectedSkill(): void {
  if (typeof window === 'undefined') return
  window.localStorage.removeItem(KEY)
}
