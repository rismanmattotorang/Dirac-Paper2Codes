"use client"

import { useCallback, useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { useToast } from "@/hooks/use-toast"
import { useAuth } from "@/lib/auth/auth-context"
import { tokensApi, type ApiTokenInfo, type CreatedToken } from "@/lib/api/tokens"

export function ApiTokensPanel() {
  const { status } = useAuth()
  const { toast } = useToast()
  const [tokens, setTokens] = useState<ApiTokenInfo[]>([])
  const [loading, setLoading] = useState(false)
  const [name, setName] = useState("")
  const [expiresInDays, setExpiresInDays] = useState("")
  const [creating, setCreating] = useState(false)
  // The plaintext is returned exactly once; surface it until the user dismisses.
  const [createdToken, setCreatedToken] = useState<CreatedToken | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      setTokens(await tokensApi.list())
    } catch {
      // Likely unauthenticated or API down; leave the list empty.
      setTokens([])
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (status === "authenticated") {
      void load()
    }
  }, [status, load])

  if (status !== "authenticated") {
    return (
      <div className="rounded-lg border border-neutral-200 dark:border-neutral-700 p-4">
        <h3 className="font-semibold text-neutral-900 dark:text-white">API Tokens</h3>
        <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400">
          Sign in to create personal API tokens for scripts and CI.
        </p>
      </div>
    )
  }

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim()) return
    setCreating(true)
    try {
      const days = expiresInDays ? Number(expiresInDays) : undefined
      const created = await tokensApi.create(name.trim(), Number.isFinite(days) ? days : undefined)
      setCreatedToken(created)
      setName("")
      setExpiresInDays("")
      await load()
      toast({ title: "Token created", description: "Copy it now — it won't be shown again." })
    } catch (err) {
      toast({
        title: "Failed to create token",
        description: err instanceof Error ? err.message : "Please try again.",
        variant: "destructive",
      })
    } finally {
      setCreating(false)
    }
  }

  const handleRevoke = async (id: string) => {
    try {
      await tokensApi.revoke(id)
      await load()
      toast({ title: "Token revoked" })
    } catch (err) {
      toast({
        title: "Failed to revoke token",
        description: err instanceof Error ? err.message : "Please try again.",
        variant: "destructive",
      })
    }
  }

  const copy = (value: string) => {
    void navigator.clipboard?.writeText(value)
    toast({ title: "Copied to clipboard" })
  }

  return (
    <div className="rounded-lg border border-neutral-200 dark:border-neutral-700 p-4 space-y-4" data-testid="api-tokens-panel">
      <div>
        <h3 className="font-semibold text-neutral-900 dark:text-white">API Tokens</h3>
        <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400">
          Long-lived bearer tokens for programmatic access. Use them as
          <code className="mx-1 rounded bg-neutral-100 dark:bg-neutral-800 px-1">Authorization: Bearer p2c_…</code>.
        </p>
      </div>

      {createdToken && (
        <Alert data-testid="created-token">
          <AlertDescription>
            <p className="font-medium">New token “{createdToken.name}” — copy it now:</p>
            <div className="mt-2 flex items-center gap-2">
              <code className="flex-1 truncate rounded bg-neutral-100 dark:bg-neutral-800 px-2 py-1 text-xs">
                {createdToken.token}
              </code>
              <Button type="button" size="sm" variant="outline" onClick={() => copy(createdToken.token)}>
                Copy
              </Button>
              <Button type="button" size="sm" variant="ghost" onClick={() => setCreatedToken(null)}>
                Dismiss
              </Button>
            </div>
          </AlertDescription>
        </Alert>
      )}

      <form onSubmit={handleCreate} className="flex flex-col sm:flex-row sm:items-end gap-3">
        <div className="flex-1 space-y-1">
          <Label htmlFor="token-name">Name</Label>
          <Input
            id="token-name"
            placeholder="e.g. CI pipeline"
            value={name}
            onChange={(e) => setName(e.target.value)}
            data-testid="token-name-input"
          />
        </div>
        <div className="w-full sm:w-40 space-y-1">
          <Label htmlFor="token-expiry">Expires (days)</Label>
          <Input
            id="token-expiry"
            type="number"
            min="1"
            placeholder="never"
            value={expiresInDays}
            onChange={(e) => setExpiresInDays(e.target.value)}
          />
        </div>
        <Button type="submit" disabled={creating || !name.trim()} data-testid="create-token-button">
          {creating ? "Creating…" : "Create token"}
        </Button>
      </form>

      <div className="space-y-2">
        {loading && <p className="text-sm text-neutral-500">Loading…</p>}
        {!loading && tokens.length === 0 && (
          <p className="text-sm text-neutral-500 dark:text-neutral-400">No tokens yet.</p>
        )}
        {tokens.map((t) => (
          <div
            key={t.id}
            className="flex items-center justify-between rounded-lg bg-neutral-100 dark:bg-neutral-700/50 p-3"
          >
            <div className="min-w-0">
              <p className="font-medium text-neutral-900 dark:text-white truncate">{t.name}</p>
              <p className="text-xs text-neutral-500 dark:text-neutral-400">
                <code>{t.prefix}…</code>
                {t.expires_at ? ` · expires ${new Date(t.expires_at).toLocaleDateString()}` : " · no expiry"}
                {t.last_used_at ? ` · last used ${new Date(t.last_used_at).toLocaleDateString()}` : " · never used"}
              </p>
            </div>
            <button
              className="text-xs text-red-600 dark:text-red-400 hover:underline"
              onClick={() => handleRevoke(t.id)}
            >
              Revoke
            </button>
          </div>
        ))}
      </div>
    </div>
  )
}
