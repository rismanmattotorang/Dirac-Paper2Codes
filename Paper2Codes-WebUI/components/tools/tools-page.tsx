"use client"

import { useCallback } from "react"
import { useQuery } from "@tanstack/react-query"
import { getTools } from "@/lib/api/tools"
import type { ToolInfo } from "@/lib/api/types"
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { IconToolbox, IconLoader, IconAlertCircle } from "@/components/ui/icons"
import { toast } from "sonner"

function copyPrompt(text: string, label: string) {
  if (typeof navigator === "undefined" || !navigator.clipboard) {
    toast.error("Clipboard not available")
    return
  }

  navigator.clipboard
    .writeText(text)
    .then(() => toast.success(`${label} copied to clipboard`))
    .catch(() => toast.error("Failed to copy prompt"))
}

function PromptPanel({
  title,
  content,
  collapsibleId,
}: {
  title: string
  content: string
  collapsibleId: string
}) {
  const handleCopy = useCallback(
    (event: React.MouseEvent<HTMLButtonElement>) => {
      event.preventDefault()
      event.stopPropagation()
      copyPrompt(content, title)
    },
    [content, title],
  )

  return (
    <details className="group rounded-lg border border-border/60 bg-muted/40 p-4 transition-all">
      <summary
        className="flex items-center justify-between cursor-pointer text-sm font-semibold text-foreground"
        aria-controls={collapsibleId}
      >
        <span>{title}</span>
        <Button variant="ghost" size="sm" onClick={handleCopy}>
          Copy
        </Button>
      </summary>
      <pre
        id={collapsibleId}
        className="mt-3 max-h-72 overflow-auto rounded-md bg-background/95 p-3 text-xs leading-relaxed text-muted-foreground whitespace-pre-wrap shadow-inner"
      >
        {content}
      </pre>
    </details>
  )
}

export function ToolsPage() {
  const {
    data: tools = [],
    isLoading,
    isError,
    error,
    refetch,
  } = useQuery<ToolInfo[], Error>({
    queryKey: ["tool-metadata"],
    queryFn: async () => {
      const response = await getTools()
      return response.data ?? []
    },
    staleTime: 5 * 60 * 1000,
  })

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between flex-wrap gap-4">
        <div>
          <h1 className="text-3xl font-bold text-neutral-900 dark:text-neutral-100">Tools & Prompts</h1>
          <p className="text-neutral-600 dark:text-neutral-400 mt-1 max-w-2xl">
            Inspect available MCP-inspired tools, their model requirements, and the system prompts powering Paper2Codes.
          </p>
        </div>
        <Badge variant="outline" className="px-3 py-1 text-xs font-semibold uppercase tracking-wide">
          {tools.length} Tools Available
        </Badge>
      </div>

      {isLoading ? (
        <div className="grid gap-4 md:grid-cols-2">
          {[...Array(4)].map((_, idx) => (
            <div
              key={idx}
              className="h-48 animate-pulse rounded-xl border border-border/70 bg-muted/40"
              role="status"
              aria-label="Loading tool card"
            />
          ))}
        </div>
      ) : isError ? (
        <div className="card-base p-10 text-center space-y-4">
          <IconAlertCircle className="w-10 h-10 text-destructive mx-auto" aria-hidden="true" />
          <div className="space-y-1">
            <h2 className="text-lg font-semibold text-foreground">Failed to load tool metadata</h2>
            <p className="text-sm text-muted-foreground">{error.message || "An unexpected error occurred."}</p>
          </div>
          <Button onClick={() => refetch()} className="mx-auto">
            Retry
          </Button>
        </div>
      ) : tools.length === 0 ? (
        <div className="card-base p-10 text-center space-y-4">
          <IconLoader className="w-10 h-10 text-primary mx-auto animate-spin" aria-hidden="true" />
          <div className="space-y-1">
            <h2 className="text-lg font-semibold text-foreground">No tools registered</h2>
            <p className="text-sm text-muted-foreground">
              Configure the backend tool catalog to see prompts and model requirements here.
            </p>
          </div>
        </div>
      ) : (
        <div className="grid gap-5 lg:grid-cols-2">
          {tools.map((tool) => (
            <Card key={tool.name} className="border-border/70">
              <CardHeader>
                <CardTitle className="flex items-center gap-3 text-lg">
                  <span className="inline-flex h-9 w-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
                    <IconToolbox className="w-5 h-5" aria-hidden="true" />
                  </span>
                  {tool.name}
                </CardTitle>
                <CardDescription>{tool.description}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-5">
                <div className="flex flex-wrap gap-2">
                  <Badge variant={tool.requires_model ? "destructive" : "secondary"}>
                    {tool.requires_model ? "Requires model access" : "No model required"}
                  </Badge>
                  <Badge variant="outline">Category: {tool.model_category.replace("_", " ")}</Badge>
                  <Badge variant="outline">
                    Temperature: {tool.temperature_label} ({tool.temperature_value.toFixed(1)})
                  </Badge>
                </div>

                {tool.primary_prompt && (
                  <PromptPanel
                    title="Primary Prompt"
                    content={tool.primary_prompt.content}
                    collapsibleId={`${tool.name}-primary`}
                  />
                )}

                {tool.supplemental_prompts.length > 0 && (
                  <div className="space-y-3">
                    <h3 className="text-sm font-semibold text-foreground">Supplemental Prompts</h3>
                    <div className="space-y-3">
                      {tool.supplemental_prompts.map((prompt, index) => (
                        <PromptPanel
                          key={`${tool.name}-supplemental-${prompt.name}-${index}`}
                          title={prompt.name || `Supplemental Prompt ${index + 1}`}
                          content={prompt.content}
                          collapsibleId={`${tool.name}-supplemental-${index}`}
                        />
                      ))}
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  )
}

