"use client"

import { CodeBrowser } from "./CodeBrowser"
import { GenerationJobsPanel } from "./generation-jobs-panel"
import { Button } from "@/components/ui/button"
import { IconDownload, IconRefresh } from "@/components/ui/icons"
import { toast } from "sonner"
import { downloadRepository } from "@/lib/api/repositories"
import { useQuery } from "@tanstack/react-query"
import { getRepositories } from "@/lib/api/repositories"

export function CodeGenerationPage() {
  const { data: reposData } = useQuery({
    queryKey: ["repositories"],
    queryFn: () => getRepositories(1, 100),
  })

  const selectedRepo = reposData?.data?.[0]

  const handleDownloadRepo = async () => {
    if (!selectedRepo) {
      toast.error("No repository available")
      return
    }

    try {
      const blob = await downloadRepository(selectedRepo.id)
      const url = URL.createObjectURL(blob)
      const a = document.createElement("a")
      a.href = url
      a.download = `repository-${selectedRepo.id}.zip`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      toast.success("Repository downloaded successfully")
    } catch (error) {
      toast.error("Failed to download repository")
      console.error(error)
    }
  }

  return (
    <div className="p-6 space-y-6 h-full flex flex-col">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-neutral-900 dark:text-white">
            Generated Code
          </h1>
          <p className="text-neutral-600 dark:text-neutral-400 mt-1">
            Browse, edit, and download your generated implementation
          </p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => window.location.reload()}
            title="Refresh"
          >
            <IconRefresh className="w-4 h-4 mr-2" />
            Refresh
          </Button>
          {selectedRepo && (
            <Button variant="default" size="sm" onClick={handleDownloadRepo}>
              <IconDownload className="w-4 h-4 mr-2" />
              Download Repository
            </Button>
          )}
        </div>
      </div>

      <GenerationJobsPanel />

      <div className="flex-1">
        <CodeBrowser />
      </div>
    </div>
  )
}
