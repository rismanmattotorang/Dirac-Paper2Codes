"use client"

import { useEffect, useState } from "react"
import { IconBrain } from "@/components/ui/icons"
import { getSelectedSkill } from "@/lib/skill-selection"
import { skillsApi } from "@/lib/api/skills"

/**
 * Shows which domain skill (and language) will guide generation for the next
 * uploaded paper, reading the persisted selection. Renders an unobtrusive hint
 * when no skill is selected. Resolves the skill's display name from the API,
 * falling back to the id if the lookup fails.
 */
export function SelectedSkillBanner({ className = "" }: { className?: string }) {
  const [name, setName] = useState<string | null>(null)
  const [language, setLanguage] = useState<string | null>(null)
  const [loaded, setLoaded] = useState(false)

  useEffect(() => {
    let cancelled = false
    const selection = getSelectedSkill()
    if (!selection?.skillId) {
      setLoaded(true)
      return
    }
    setLanguage(selection.language ?? null)
    skillsApi
      .get(selection.skillId)
      .then((skill) => {
        if (!cancelled) setName(skill?.name || selection.skillId)
      })
      .catch(() => {
        if (!cancelled) setName(selection.skillId)
      })
      .finally(() => {
        if (!cancelled) setLoaded(true)
      })
    return () => {
      cancelled = true
    }
  }, [])

  if (!loaded) return null

  if (!name) {
    return (
      <div
        data-testid="selected-skill-banner"
        className={`text-xs rounded-md px-3 py-2 bg-neutral-50 dark:bg-neutral-800/50 border border-neutral-200 dark:border-neutral-700 text-neutral-500 dark:text-neutral-400 ${className}`}
      >
        No domain skill selected — generation will auto-detect the domain. Pick one under{" "}
        <span className="font-medium text-neutral-700 dark:text-neutral-300">Domain Skills</span>.
      </div>
    )
  }

  return (
    <div
      data-testid="selected-skill-banner"
      className={`flex items-center gap-2 text-xs rounded-md px-3 py-2 bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 text-blue-700 dark:text-blue-300 ${className}`}
    >
      <IconBrain className="w-4 h-4 shrink-0" />
      <span>
        Domain skill <strong>{name}</strong>
        {language ? ` (${language})` : ""} will guide code generation.
      </span>
    </div>
  )
}
