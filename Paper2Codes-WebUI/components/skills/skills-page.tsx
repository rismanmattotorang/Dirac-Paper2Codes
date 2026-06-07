"use client"

import { useEffect, useMemo, useState } from "react"
import {
  IconBrain,
  IconCheckCircle,
  IconRefresh,
  IconAlertCircle,
  IconCode,
} from "@/components/ui/icons"
import { skillsApi, librariesFor, type Skill } from "@/lib/api/skills"
import { getSelectedSkill, setSelectedSkill } from "@/lib/skill-selection"
import { useToast } from "@/hooks/use-toast"

/**
 * Domain Skills page: browse the catalog of computational-domain skills, inspect
 * a skill's expertise primer and recommended libraries, and select a skill +
 * language to drive the next paper's code generation.
 */
export function SkillsPage() {
  const { toast } = useToast()
  const [skills, setSkills] = useState<Skill[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [activeId, setActiveId] = useState<string | null>(null)
  const [language, setLanguage] = useState<string>("python")

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      setIsLoading(true)
      setError(null)
      try {
        const data = await skillsApi.list()
        if (cancelled) return
        setSkills(data)
        const saved = getSelectedSkill()
        const initial = saved?.skillId && data.some((s) => s.id === saved.skillId)
          ? saved.skillId
          : data[0]?.id ?? null
        setActiveId(initial)
        if (saved?.language) setLanguage(saved.language)
      } catch (e) {
        if (!cancelled) setError(e instanceof Error ? e.message : "Failed to load skills")
      } finally {
        if (!cancelled) setIsLoading(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [])

  const active = useMemo(() => skills.find((s) => s.id === activeId) ?? null, [skills, activeId])

  // Keep the language valid for the active skill.
  useEffect(() => {
    if (active && active.languages.length > 0 && !active.languages.includes(language)) {
      setLanguage(active.languages[0])
    }
  }, [active]) // eslint-disable-line react-hooks/exhaustive-deps

  const handleUseSkill = () => {
    if (!active) return
    setSelectedSkill({ skillId: active.id, language })
    toast({
      title: "Skill selected",
      description: `${active.name} (${language}) will guide your next paper's generation.`,
    })
  }

  if (isLoading) {
    return (
      <div className="p-6 flex items-center gap-2 justify-center text-neutral-600 dark:text-neutral-400">
        <IconRefresh className="w-5 h-5 animate-spin" />
        <span>Loading domain skills…</span>
      </div>
    )
  }

  if (error) {
    return (
      <div className="p-6">
        <div className="p-4 rounded-lg bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800 flex items-center gap-2">
          <IconAlertCircle className="w-4 h-4 text-red-600 dark:text-red-400" />
          <p className="text-sm font-medium text-red-900 dark:text-red-300">{error}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-neutral-900 dark:text-white flex items-center gap-2">
          <IconBrain className="w-7 h-7 text-blue-600 dark:text-blue-400" />
          Domain Skills
        </h1>
        <p className="text-neutral-600 dark:text-neutral-400 mt-1">
          Choose a computational-domain skill to specialise retrieval, code generation, and
          verification for your paper.
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Skill list */}
        <div className="lg:col-span-1 space-y-2">
          {skills.map((skill) => {
            const isActive = skill.id === activeId
            return (
              <button
                key={skill.id}
                onClick={() => setActiveId(skill.id)}
                className={`w-full text-left p-4 rounded-lg border transition-colors ${
                  isActive
                    ? "border-blue-500 bg-blue-50 dark:bg-blue-950/30"
                    : "border-neutral-200 dark:border-neutral-700 hover:border-neutral-300 dark:hover:border-neutral-600"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-semibold text-neutral-900 dark:text-white">{skill.name}</span>
                  {!skill.builtin && (
                    <span className="text-xs px-2 py-0.5 rounded-full bg-purple-100 text-purple-700 dark:bg-purple-950/50 dark:text-purple-300">
                      Custom
                    </span>
                  )}
                </div>
                <p className="text-xs text-neutral-500 dark:text-neutral-400 mt-1 line-clamp-2">
                  {skill.description}
                </p>
              </button>
            )
          })}
        </div>

        {/* Skill detail */}
        <div className="lg:col-span-2">
          {active ? (
            <div className="card-base p-6 space-y-5">
              <div className="flex items-start justify-between gap-3 flex-wrap">
                <div>
                  <h2 className="text-xl font-bold text-neutral-900 dark:text-white">{active.name}</h2>
                  <p className="text-xs text-neutral-500 dark:text-neutral-400">
                    {active.domain ?? "Domain"} · v{active.version}
                    {active.builtin ? " · built-in" : " · custom"}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <label className="text-sm text-neutral-600 dark:text-neutral-400">Language</label>
                  <select
                    className="input-base text-sm"
                    value={language}
                    onChange={(e) => setLanguage(e.target.value)}
                  >
                    {(active.languages.length > 0 ? active.languages : ["python"]).map((l) => (
                      <option key={l} value={l}>
                        {l}
                      </option>
                    ))}
                  </select>
                  <button className="btn-primary text-sm flex items-center gap-1" onClick={handleUseSkill}>
                    <IconCheckCircle className="w-4 h-4" />
                    Use skill
                  </button>
                </div>
              </div>

              <p className="text-sm text-neutral-700 dark:text-neutral-300">{active.instructions}</p>

              <div>
                <h3 className="text-sm font-semibold text-neutral-900 dark:text-white mb-2 flex items-center gap-1">
                  <IconCode className="w-4 h-4" /> Recommended {language} libraries
                </h3>
                <div className="flex flex-wrap gap-2">
                  {librariesFor(active, language).length > 0 ? (
                    librariesFor(active, language).map((lib) => (
                      <span
                        key={lib}
                        className="text-xs font-mono px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-700/50 text-neutral-700 dark:text-neutral-300"
                      >
                        {lib}
                      </span>
                    ))
                  ) : (
                    <span className="text-xs text-neutral-500">No specific libraries listed.</span>
                  )}
                </div>
              </div>

              {active.verification_hints.length > 0 && (
                <div>
                  <h3 className="text-sm font-semibold text-neutral-900 dark:text-white mb-2">
                    Verification properties
                  </h3>
                  <ul className="list-disc list-inside space-y-1 text-sm text-neutral-700 dark:text-neutral-300">
                    {active.verification_hints.map((h, i) => (
                      <li key={i}>{h}</li>
                    ))}
                  </ul>
                </div>
              )}

              {active.keywords.length > 0 && (
                <div>
                  <h3 className="text-sm font-semibold text-neutral-900 dark:text-white mb-2">
                    Trigger keywords
                  </h3>
                  <div className="flex flex-wrap gap-1.5">
                    {active.keywords.map((kw) => (
                      <span
                        key={kw}
                        className="text-xs px-2 py-0.5 rounded-full bg-blue-50 dark:bg-blue-950/40 text-blue-700 dark:text-blue-300"
                      >
                        {kw}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="card-base p-6 text-sm text-neutral-500">Select a skill to view details.</div>
          )}
        </div>
      </div>
    </div>
  )
}
