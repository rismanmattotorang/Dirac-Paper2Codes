"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import {
  IconKey,
  IconEye,
  IconEyeOff,
  IconTrash,
  IconStar,
  IconRefresh,
  IconCheckCircle,
  IconAlertCircle,
} from "@/components/ui/icons"
import { settingsApi, type ProviderStatus, type TestKeyResponse } from "@/lib/api/settings"
import { useToast } from "@/hooks/use-toast"

type TestState = { status: "idle" | "testing" | "ok" | "fail"; message?: string; latency?: number }

/**
 * Dedicated panel for managing per-provider LLM API keys.
 *
 * Shows the live status of every provider (configured / source / default),
 * never reveals stored keys (only a masked preview), and supports validating a
 * key against the live provider before or after saving.
 */
export function ApiKeysPanel() {
  const { toast } = useToast()
  const [providers, setProviders] = useState<ProviderStatus[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [loadError, setLoadError] = useState<string | null>(null)

  // Per-provider transient UI state, keyed by provider id.
  const [drafts, setDrafts] = useState<Record<string, string>>({})
  const [revealed, setRevealed] = useState<Record<string, boolean>>({})
  const [busy, setBusy] = useState<Record<string, boolean>>({})
  const [tests, setTests] = useState<Record<string, TestState>>({})

  const load = useCallback(async () => {
    setIsLoading(true)
    setLoadError(null)
    try {
      const data = await settingsApi.listProviders()
      setProviders(data)
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to load providers"
      setLoadError(message)
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const setProviderBusy = (id: string, value: boolean) =>
    setBusy((prev) => ({ ...prev, [id]: value }))

  const handleSave = async (provider: ProviderStatus) => {
    const key = (drafts[provider.id] ?? "").trim()
    if (!key) {
      toast({ title: "Missing key", description: "Enter an API key first.", variant: "destructive" })
      return
    }
    setProviderBusy(provider.id, true)
    try {
      const updated = await settingsApi.setProviderKey(provider.id, key)
      setProviders((prev) => prev.map((p) => (p.id === updated.id ? updated : p)))
      setDrafts((prev) => ({ ...prev, [provider.id]: "" }))
      setTests((prev) => ({ ...prev, [provider.id]: { status: "idle" } }))
      toast({ title: "Saved", description: `${provider.name} API key stored and active.` })
    } catch (error) {
      toast({
        title: "Could not save key",
        description: error instanceof Error ? error.message : `Failed to save ${provider.name} key`,
        variant: "destructive",
      })
    } finally {
      setProviderBusy(provider.id, false)
    }
  }

  const handleTest = async (provider: ProviderStatus) => {
    const draft = (drafts[provider.id] ?? "").trim()
    setTests((prev) => ({ ...prev, [provider.id]: { status: "testing" } }))
    try {
      const result: TestKeyResponse = await settingsApi.testProviderKey(
        provider.id,
        draft.length > 0 ? draft : undefined,
      )
      setTests((prev) => ({
        ...prev,
        [provider.id]: {
          status: result.valid ? "ok" : "fail",
          message: result.message,
          latency: result.latency_ms,
        },
      }))
    } catch (error) {
      setTests((prev) => ({
        ...prev,
        [provider.id]: {
          status: "fail",
          message: error instanceof Error ? error.message : "Test failed",
        },
      }))
    }
  }

  const handleRemove = async (provider: ProviderStatus) => {
    setProviderBusy(provider.id, true)
    try {
      const updated = await settingsApi.deleteProviderKey(provider.id)
      setProviders((prev) => prev.map((p) => (p.id === updated.id ? updated : p)))
      setTests((prev) => ({ ...prev, [provider.id]: { status: "idle" } }))
      toast({ title: "Removed", description: `${provider.name} API key removed.` })
    } catch (error) {
      toast({
        title: "Could not remove key",
        description: error instanceof Error ? error.message : `Failed to remove ${provider.name} key`,
        variant: "destructive",
      })
    } finally {
      setProviderBusy(provider.id, false)
    }
  }

  const handleSetDefault = async (provider: ProviderStatus) => {
    setProviderBusy(provider.id, true)
    try {
      const updated = await settingsApi.setDefaultProvider(provider.id)
      setProviders(updated)
      toast({ title: "Default updated", description: `${provider.name} is now the default provider.` })
    } catch (error) {
      toast({
        title: "Could not set default",
        description: error instanceof Error ? error.message : "Failed to set default provider",
        variant: "destructive",
      })
    } finally {
      setProviderBusy(provider.id, false)
    }
  }

  const configuredCount = useMemo(
    () => providers.filter((p) => p.configured).length,
    [providers],
  )

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 py-10 justify-center text-neutral-600 dark:text-neutral-400">
        <IconRefresh className="w-5 h-5 animate-spin" />
        <span>Loading providers…</span>
      </div>
    )
  }

  if (loadError) {
    return (
      <div className="p-4 rounded-lg bg-red-50 dark:bg-red-950/30 border border-red-200 dark:border-red-800">
        <div className="flex items-center gap-2 text-red-900 dark:text-red-300">
          <IconAlertCircle className="w-4 h-4" />
          <p className="text-sm font-medium">{loadError}</p>
        </div>
        <button className="mt-3 btn-secondary text-sm" onClick={() => void load()}>
          Retry
        </button>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <IconKey className="w-5 h-5 text-blue-600 dark:text-blue-400" />
          <div>
            <h3 className="font-semibold text-neutral-900 dark:text-white">Provider API Keys</h3>
            <p className="text-xs text-neutral-500 dark:text-neutral-400">
              {configuredCount} of {providers.length} providers configured · keys are stored
              server-side and never displayed in full
            </p>
          </div>
        </div>
        <button
          className="btn-secondary text-sm flex items-center gap-2"
          onClick={() => void load()}
          aria-label="Refresh provider status"
        >
          <IconRefresh className="w-3.5 h-3.5" />
          Refresh
        </button>
      </div>

      <div className="space-y-3">
        {providers.map((provider) => (
          <ProviderCard
            key={provider.id}
            provider={provider}
            draft={drafts[provider.id] ?? ""}
            revealed={!!revealed[provider.id]}
            busy={!!busy[provider.id]}
            test={tests[provider.id] ?? { status: "idle" }}
            onDraftChange={(value) => setDrafts((prev) => ({ ...prev, [provider.id]: value }))}
            onToggleReveal={() =>
              setRevealed((prev) => ({ ...prev, [provider.id]: !prev[provider.id] }))
            }
            onSave={() => void handleSave(provider)}
            onTest={() => void handleTest(provider)}
            onRemove={() => void handleRemove(provider)}
            onSetDefault={() => void handleSetDefault(provider)}
          />
        ))}
      </div>
    </div>
  )
}

function ProviderCard({
  provider,
  draft,
  revealed,
  busy,
  test,
  onDraftChange,
  onToggleReveal,
  onSave,
  onTest,
  onRemove,
  onSetDefault,
}: {
  provider: ProviderStatus
  draft: string
  revealed: boolean
  busy: boolean
  test: TestState
  onDraftChange: (value: string) => void
  onToggleReveal: () => void
  onSave: () => void
  onTest: () => void
  onRemove: () => void
  onSetDefault: () => void
}) {
  const fromEnv = provider.key_source === "environment"
  const placeholder = provider.key_prefix_hint
    ? `${provider.key_prefix_hint}…`
    : "Enter API key"

  return (
    <div
      data-testid={`provider-${provider.id}`}
      className="rounded-lg border border-neutral-200 dark:border-neutral-700 bg-white dark:bg-neutral-800/50 p-4 space-y-3"
    >
      {/* Header row */}
      <div className="flex items-start justify-between gap-3 flex-wrap">
        <div className="min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="font-semibold text-neutral-900 dark:text-white">{provider.name}</span>
            {provider.is_default && (
              <span className="inline-flex items-center gap-1 text-xs font-medium px-2 py-0.5 rounded-full bg-blue-100 text-blue-700 dark:bg-blue-950/50 dark:text-blue-300">
                <IconStar className="w-3 h-3" />
                Default
              </span>
            )}
            <StatusBadge provider={provider} />
          </div>
          <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400 truncate">
            {provider.base_url}
            {provider.models.length > 0 && (
              <> · {provider.models.length} model{provider.models.length === 1 ? "" : "s"}</>
            )}
          </p>
          {provider.configured && provider.masked_key && (
            <p className="mt-1 font-mono text-xs text-neutral-600 dark:text-neutral-300">
              {provider.masked_key}
              {fromEnv && (
                <span className="ml-2 not-italic text-neutral-400">(from environment variable)</span>
              )}
            </p>
          )}
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {provider.configured && !provider.is_default && (
            <button
              className="btn-secondary text-xs flex items-center gap-1"
              onClick={onSetDefault}
              disabled={busy}
              title="Set as default provider"
            >
              <IconStar className="w-3.5 h-3.5" />
              Set default
            </button>
          )}
          {provider.configured && !fromEnv && (
            <button
              className="text-xs text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300 inline-flex items-center gap-1 px-2 py-1 rounded-md hover:bg-red-50 dark:hover:bg-red-950/30 disabled:opacity-50"
              onClick={onRemove}
              disabled={busy}
              title="Remove this key"
            >
              <IconTrash className="w-3.5 h-3.5" />
              Remove
            </button>
          )}
        </div>
      </div>

      {/* Editor row */}
      <div className="flex flex-col sm:flex-row gap-2">
        <div className="relative flex-1">
          <input
            type={revealed ? "text" : "password"}
            className="input-base w-full pr-10 font-mono"
            placeholder={fromEnv ? "Overrides the environment variable" : placeholder}
            value={draft}
            autoComplete="off"
            spellCheck={false}
            onChange={(e) => onDraftChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && draft.trim() && !busy) onSave()
            }}
          />
          <button
            type="button"
            className="absolute right-2 top-1/2 -translate-y-1/2 text-neutral-400 hover:text-neutral-600 dark:hover:text-neutral-200"
            onClick={onToggleReveal}
            aria-label={revealed ? "Hide key" : "Show key"}
            tabIndex={-1}
          >
            {revealed ? <IconEyeOff className="w-4 h-4" /> : <IconEye className="w-4 h-4" />}
          </button>
        </div>
        <div className="flex gap-2">
          <button
            className="btn-secondary px-3"
            onClick={onTest}
            disabled={test.status === "testing" || (!provider.configured && !draft.trim())}
            title="Validate the key against the provider"
          >
            {test.status === "testing" ? (
              <span className="inline-flex items-center gap-1">
                <IconRefresh className="w-3.5 h-3.5 animate-spin" /> Testing
              </span>
            ) : (
              "Test"
            )}
          </button>
          <button className="btn-primary px-4" onClick={onSave} disabled={busy || !draft.trim()}>
            {busy ? "Saving…" : provider.configured ? "Update" : "Save"}
          </button>
        </div>
      </div>

      {/* Test result */}
      {test.status === "ok" && (
        <div className="flex items-center gap-2 text-xs text-green-700 dark:text-green-400">
          <IconCheckCircle className="w-4 h-4" />
          <span>
            {test.message}
            {typeof test.latency === "number" && ` · ${test.latency}ms`}
          </span>
        </div>
      )}
      {test.status === "fail" && (
        <div className="flex items-center gap-2 text-xs text-red-600 dark:text-red-400">
          <IconAlertCircle className="w-4 h-4" />
          <span>{test.message}</span>
        </div>
      )}
    </div>
  )
}

function StatusBadge({ provider }: { provider: ProviderStatus }) {
  if (!provider.configured) {
    return (
      <span className="text-xs font-medium px-2 py-0.5 rounded-full bg-neutral-100 text-neutral-500 dark:bg-neutral-700/50 dark:text-neutral-400">
        Not configured
      </span>
    )
  }
  if (provider.key_source === "environment") {
    return (
      <span className="text-xs font-medium px-2 py-0.5 rounded-full bg-amber-100 text-amber-700 dark:bg-amber-950/50 dark:text-amber-300">
        From environment
      </span>
    )
  }
  return (
    <span className="inline-flex items-center gap-1 text-xs font-medium px-2 py-0.5 rounded-full bg-green-100 text-green-700 dark:bg-green-950/50 dark:text-green-300">
      <IconCheckCircle className="w-3 h-3" />
      Configured
    </span>
  )
}
