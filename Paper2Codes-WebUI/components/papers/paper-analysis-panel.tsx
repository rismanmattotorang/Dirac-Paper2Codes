"use client"

import { IconChevronLeft, IconZap, IconBarChart, IconClock, IconLoader, IconAlertCircle } from "@/components/ui/icons"
import { usePaper, usePaperStatus, usePaperSegments } from "@/lib/hooks/use-papers"
import type { PaperSegment, Paper } from "@/lib/api/types"
import { useMemo } from "react"

interface PaperAnalysisPanelProps {
  paperId: string
  onBack: () => void
}

const statusStyles: Record<
  string,
  { label: string; badge: string }
> = {
  pending: { label: "Pending", badge: "bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-200" },
  parsing: { label: "Parsing", badge: "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-200" },
  segmenting: { label: "Segmenting", badge: "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-200" },
  extracting: { label: "Extracting", badge: "bg-purple-100 text-purple-800 dark:bg-purple-900/40 dark:text-purple-200" },
  classifying: { label: "Classifying", badge: "bg-indigo-100 text-indigo-800 dark:bg-indigo-900/40 dark:text-indigo-200" },
  embedding: { label: "Embedding", badge: "bg-sky-100 text-sky-800 dark:bg-sky-900/40 dark:text-sky-200" },
  completed: { label: "Completed", badge: "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-200" },
  failed: { label: "Failed", badge: "bg-red-100 text-red-800 dark:bg-red-900/40 dark:text-red-200" },
}

const formatSegmentType = (segment: PaperSegment) => {
  if (typeof segment.segment_type === "string") {
    return segment.segment_type
  }
  if (segment.segment_type && "Other" in segment.segment_type) {
    return segment.segment_type.Other
  }
  return "Segment"
}

const deriveFallbackStatus = (paper: Paper | null): "completed" | "processing" | "pending" => {
  if (!paper) return "pending"
  if (paper.segments?.length && paper.algorithms?.length) {
    return "completed"
  }
  if (paper.segments?.length) {
    return "processing"
  }
  return "pending"
}

export function PaperAnalysisPanel({ paperId, onBack }: PaperAnalysisPanelProps) {
  const {
    paper,
    isLoading: isPaperLoading,
    isError: isPaperError,
    error: paperError,
  } = usePaper(paperId, { enabled: !!paperId })
  const {
    status,
    isLoading: isStatusLoading,
    error: statusError,
  } = usePaperStatus(paperId, { enabled: !!paperId, refetchInterval: 5_000 })
  const {
    segments,
    isLoading: isSegmentsLoading,
    error: segmentsError,
  } = usePaperSegments(paperId, { enabled: !!paperId, refetchInterval: 30_000 })

  const combinedSegments = useMemo(() => {
    if (segments && segments.length > 0) {
      return segments
    }
    return paper?.segments ?? []
  }, [segments, paper])

  const algorithms = paper?.algorithms ?? []
  const equations = paper?.equations ?? []
  const references = paper?.references ?? []

  const normalizedStatus = status?.status ?? deriveFallbackStatus(paper)
  const statusInfo =
    statusStyles[normalizedStatus] ?? statusStyles[normalizedStatus === "processing" ? "segmenting" : "pending"]

  if (isPaperLoading) {
    return (
      <div className="flex items-center justify-center h-[60vh]">
        <div className="flex flex-col items-center gap-3 text-muted-foreground">
          <IconLoader className="w-6 h-6 animate-spin" />
          <p>Loading paper details…</p>
        </div>
      </div>
    )
  }

  if (isPaperError || !paper) {
    return (
      <div className="space-y-4">
        <button
          onClick={onBack}
          className="flex items-center gap-2 text-blue-600 dark:text-blue-400 hover:text-blue-700 font-semibold"
        >
          <IconChevronLeft className="w-4 h-4" />
          Back to Papers
        </button>
        <div className="card-base p-6 flex items-center gap-3 text-red-600 dark:text-red-400">
          <IconAlertCircle className="w-5 h-5" />
          <div>
            <p className="font-semibold">Failed to load paper</p>
            <p className="text-sm text-muted-foreground">
              {paperError?.message || "Please retry or select another paper."}
            </p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <button
        onClick={onBack}
        className="flex items-center gap-2 text-blue-600 dark:text-blue-400 hover:text-blue-700 font-semibold"
      >
        <IconChevronLeft className="w-4 h-4" />
        Back to Papers
      </button>

      <div className="space-y-3">
        <div className="flex flex-wrap items-start gap-2">
          <h1 className="text-3xl font-bold text-neutral-900 dark:text-white flex-1">{paper.title}</h1>
          <span className={`inline-flex items-center px-3 py-1 rounded-full text-xs font-semibold ${statusInfo.badge}`}>
            {statusInfo.label}
            {isStatusLoading && <IconLoader className="w-3 h-3 ml-2 animate-spin" />}
          </span>
        </div>
        <div className="text-sm text-muted-foreground flex flex-wrap gap-2">
          {paper.metadata?.authors?.length ? (
            <span>{paper.metadata.authors.join(", ")}</span>
          ) : (
            <span>Unknown authors</span>
          )}
          {paper.metadata?.year && (
            <>
              <span>•</span>
              <span>{paper.metadata.year}</span>
            </>
          )}
          {paper.metadata?.venue && (
            <>
              <span>•</span>
              <span>{paper.metadata.venue}</span>
            </>
          )}
        </div>
        {(statusError || segmentsError) && (
          <p className="text-xs text-muted-foreground">
            {statusError?.message ?? segmentsError?.message ?? null}
          </p>
        )}
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        {[
          { icon: IconZap, label: "Segments Extracted", value: combinedSegments.length },
          { icon: IconBarChart, label: "Algorithms Identified", value: algorithms.length },
          { icon: IconClock, label: "References", value: references.length },
        ].map(({ icon: Icon, label, value }) => (
          <div key={label} className="card-base p-4 flex items-center gap-3">
            <Icon className="w-5 h-5 text-blue-600 dark:text-blue-400 flex-shrink-0" />
            <div>
              <p className="text-xs text-muted-foreground">{label}</p>
              <p className="text-lg font-semibold text-foreground">{value}</p>
            </div>
          </div>
        ))}
      </div>

      <section className="card-base p-6 space-y-4">
        <header>
          <h2 className="text-lg font-semibold text-foreground">Abstract</h2>
        </header>
        <p className="text-sm leading-relaxed text-muted-foreground whitespace-pre-line">
          {paper.abstract_text || "No abstract provided for this paper."}
        </p>
      </section>

      <section className="card-base p-6 space-y-4">
        <header className="flex items-center justify-between">
          <h2 className="text-lg font-semibold text-foreground">Segments</h2>
          {isSegmentsLoading && <IconLoader className="w-4 h-4 text-muted-foreground animate-spin" />}
        </header>
        {combinedSegments.length === 0 ? (
          <p className="text-sm text-muted-foreground">No segments available yet. Processing may still be running.</p>
        ) : (
          <div className="space-y-3">
            {combinedSegments.slice(0, 6).map((segment) => (
              <article
                key={segment.id}
                className="rounded-lg border border-border/60 bg-muted/20 px-4 py-3 space-y-2"
              >
                <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground uppercase tracking-wide">
                  <span>{formatSegmentType(segment)}</span>
                  <span>
                    Lines {segment.line_range[0]} – {segment.line_range[1]}
                  </span>
                </div>
                <p className="text-sm text-foreground leading-relaxed">{segment.content}</p>
              </article>
            ))}
            {combinedSegments.length > 6 && (
              <p className="text-xs text-muted-foreground">
                Showing 6 of {combinedSegments.length} segments. Additional segments are available through the API.
              </p>
            )}
          </div>
        )}
      </section>

      <section className="card-base p-6 space-y-4">
        <header>
          <h2 className="text-lg font-semibold text-foreground">Algorithms</h2>
        </header>
        {algorithms.length === 0 ? (
          <p className="text-sm text-muted-foreground">No algorithms extracted yet.</p>
        ) : (
          <div className="space-y-4">
            {algorithms.slice(0, 4).map((algorithm) => (
              <article key={algorithm.id} className="rounded-lg border border-border/60 bg-muted/10 p-4 space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <h3 className="text-sm font-semibold text-foreground">{algorithm.name}</h3>
                  <span className="text-xs text-muted-foreground">
                    Lines {algorithm.line_range[0]} – {algorithm.line_range[1]}
                  </span>
                </div>
                <pre className="text-xs whitespace-pre-wrap bg-background/60 rounded-md p-3 border border-border/50 overflow-auto">
                  {algorithm.pseudocode}
                </pre>
                {algorithm.description && (
                  <p className="text-xs text-muted-foreground leading-relaxed">{algorithm.description}</p>
                )}
              </article>
            ))}
            {algorithms.length > 4 && (
              <p className="text-xs text-muted-foreground">
                Showing 4 of {algorithms.length} algorithms. View the full repository for the complete set.
              </p>
            )}
          </div>
        )}
      </section>
    </div>
  )
}
