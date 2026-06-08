"use client"

import { useQuery, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { jobsApi, type JobInfo, type JobStatus } from "@/lib/api/jobs"

const STATUS_STYLES: Record<JobStatus, string> = {
  pending: "bg-neutral-200 text-neutral-700 dark:bg-neutral-700 dark:text-neutral-200",
  running: "bg-blue-100 text-blue-800 dark:bg-blue-950/40 dark:text-blue-300",
  completed: "bg-green-100 text-green-800 dark:bg-green-950/40 dark:text-green-300",
  failed: "bg-red-100 text-red-800 dark:bg-red-950/40 dark:text-red-300",
  cancelled: "bg-amber-100 text-amber-800 dark:bg-amber-950/40 dark:text-amber-300",
}

function StatusBadge({ status }: { status: JobStatus }) {
  return (
    <span className={`inline-block rounded-full px-2 py-0.5 text-xs font-medium ${STATUS_STYLES[status]}`}>
      {status}
    </span>
  )
}

/**
 * Live view of durable generation jobs. Now that `POST /api/papers/:id/process`
 * enqueues a job, this binds the "Generated Code" page to actual runs: status,
 * progress, retry count, and errors, with the ability to cancel in-flight work.
 * Polls while any job is still pending/running.
 */
export function GenerationJobsPanel({ paperId }: { paperId?: string }) {
  const queryClient = useQueryClient()
  const queryKey = ["generation-jobs", paperId ?? "all"]

  const { data: jobs = [], isLoading } = useQuery<JobInfo[]>({
    queryKey,
    queryFn: () => jobsApi.list(paperId ? { paperId } : undefined),
    // Poll while work is outstanding; otherwise back off.
    refetchInterval: (query) => {
      const data = query.state.data as JobInfo[] | undefined
      const active = data?.some((j) => j.status === "pending" || j.status === "running")
      return active ? 2000 : false
    },
  })

  const handleCancel = async (id: string) => {
    try {
      await jobsApi.cancel(id)
      toast.success("Job cancelled")
      void queryClient.invalidateQueries({ queryKey })
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to cancel job")
    }
  }

  return (
    <div className="rounded-lg border border-neutral-200 dark:border-neutral-700 p-4" data-testid="generation-jobs-panel">
      <h2 className="font-semibold text-neutral-900 dark:text-white">Generation runs</h2>
      <p className="mt-1 text-sm text-neutral-500 dark:text-neutral-400">
        Durable code-generation jobs and their live status.
      </p>

      <div className="mt-4 space-y-2">
        {isLoading && <p className="text-sm text-neutral-500">Loading…</p>}
        {!isLoading && jobs.length === 0 && (
          <p className="text-sm text-neutral-500 dark:text-neutral-400">
            No generation runs yet. Upload a paper and start processing to see jobs here.
          </p>
        )}
        {jobs.map((job) => (
          <div
            key={job.id}
            className="flex items-center justify-between gap-3 rounded-lg bg-neutral-100 dark:bg-neutral-700/50 p-3"
            data-testid="generation-job"
          >
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <StatusBadge status={job.status} />
                <span className="truncate text-sm font-medium text-neutral-900 dark:text-white">
                  {job.paper_id ? `Paper ${job.paper_id}` : job.kind}
                </span>
              </div>
              <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
                {Math.round(job.progress * 100)}% · attempt {job.attempts}/{job.max_attempts}
                {job.last_error ? ` · ${job.last_error}` : ""}
              </p>
            </div>
            {(job.status === "pending" || job.status === "running") && (
              <Button variant="outline" size="sm" onClick={() => handleCancel(job.id)}>
                Cancel
              </Button>
            )}
          </div>
        ))}
      </div>
    </div>
  )
}
