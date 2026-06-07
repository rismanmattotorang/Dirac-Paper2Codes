"use client"

import { IconFileText, IconFilter, IconSearch, IconUpload, IconAlertCircle, IconLoader, IconPlus } from "@/components/ui/icons"
import { useState } from "react"
import { PaperAnalysisPanel } from "./paper-analysis-panel"
import { PaperUploadDialog } from "./paper-upload-dialog"
import { usePapers } from "@/lib/hooks/use-papers"
import type { Paper } from "@/lib/api/types"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Textarea } from "@/components/ui/textarea"
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu"

export function PapersPage() {
  const [selectedPaper, setSelectedPaper] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)
  const [isUploadDialogOpen, setIsUploadDialogOpen] = useState(false)
  const [newPaperTitle, setNewPaperTitle] = useState("")
  const [newPaperAbstract, setNewPaperAbstract] = useState("")
  
  const { papers, isLoading, error, createPaper, refetch } = usePapers({ enabled: true, page: 1, per_page: 50 })
  
  // Filter papers based on search query
  const filteredPapers = papers.filter((paper) =>
    paper.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
    paper.abstract_text.toLowerCase().includes(searchQuery.toLowerCase()) ||
    (paper.metadata.authors && paper.metadata.authors.some((author: string) =>
      author.toLowerCase().includes(searchQuery.toLowerCase())
    ))
  )
  
  const handleUploadComplete = () => {
    // Refresh papers list after upload
    refetch()
  }
  
  const handleCreatePaper = async () => {
    if (!newPaperTitle.trim()) return
    
    try {
      await createPaper({
        title: newPaperTitle,
        abstract_text: newPaperAbstract || "",
      })
      setIsCreateDialogOpen(false)
      setNewPaperTitle("")
      setNewPaperAbstract("")
    } catch (err) {
      console.error("Failed to create paper:", err)
    }
  }
  
  const getPaperStatus = (paper: Paper) => {
    const hasSegments = paper.segments && paper.segments.length > 0
    const hasAlgorithms = paper.algorithms && paper.algorithms.length > 0
    const hasEmbeddings = paper.segments && paper.segments.some(s => s.embedding && s.embedding.length > 0)
    
    if (hasSegments && hasAlgorithms && hasEmbeddings) {
      return { status: "analyzed", progress: 100 }
    } else if (hasSegments) {
      return { status: "processing", progress: 65 }
    } else {
      return { status: "queued", progress: 0 }
    }
  }

  if (error) {
    return (
      <div className="p-6 space-y-6">
        <div>
          <h1 className="text-3xl font-bold text-neutral-900 dark:text-white">Papers</h1>
          <p className="text-neutral-600 dark:text-neutral-400 mt-1">Manage and analyze your research papers</p>
        </div>
        <div className="card-base p-8">
          <div className="text-center">
            <IconAlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <h3 className="text-lg font-semibold text-neutral-900 dark:text-white mb-2">Failed to load papers</h3>
            <p className="text-sm text-neutral-600 dark:text-neutral-400">{error.message}</p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-neutral-900 dark:text-white">Papers</h1>
          <p className="text-neutral-600 dark:text-neutral-400 mt-1">Manage and analyze your research papers</p>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button className="flex items-center gap-2">
              <IconPlus className="w-4 h-4" />
              Add Paper
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={() => setIsUploadDialogOpen(true)}>
              <IconUpload className="w-4 h-4 mr-2" />
              Upload File
            </DropdownMenuItem>
            <DropdownMenuItem onClick={() => setIsCreateDialogOpen(true)}>
              <IconPlus className="w-4 h-4 mr-2" />
              Create Manually
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        
        <Dialog open={isCreateDialogOpen} onOpenChange={setIsCreateDialogOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create New Paper</DialogTitle>
              <DialogDescription>
                Add a new research paper to your collection
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="title">Title *</Label>
                <Input
                  id="title"
                  placeholder="Enter paper title"
                  value={newPaperTitle}
                  onChange={(e) => setNewPaperTitle(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="abstract">Abstract</Label>
                <Textarea
                  id="abstract"
                  placeholder="Enter paper abstract (optional)"
                  value={newPaperAbstract}
                  onChange={(e) => setNewPaperAbstract(e.target.value)}
                  rows={6}
                />
              </div>
              <div className="flex justify-end gap-3 pt-4">
                <Button variant="outline" onClick={() => setIsCreateDialogOpen(false)}>
                  Cancel
                </Button>
                <Button onClick={handleCreatePaper} disabled={!newPaperTitle.trim()}>
                  Create Paper
                </Button>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      </div>

      {selectedPaper ? (
        <PaperAnalysisPanel paperId={selectedPaper} onBack={() => setSelectedPaper(null)} />
      ) : (
        <div className="space-y-4">
          {/* Search and Filter */}
          <div className="flex flex-col sm:flex-row gap-3">
            <div className="flex-1 relative">
              <IconSearch className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-neutral-400" />
              <input
                type="text"
                placeholder="Search papers..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="input-base pl-10 w-full"
              />
            </div>
            <button className="btn-secondary flex items-center justify-center gap-2">
              <IconFilter className="w-4 h-4" />
              Filter
            </button>
          </div>

          {/* Loading State */}
          {isLoading && papers.length === 0 && (
            <div className="card-base p-12">
              <div className="text-center">
                <IconLoader className="w-12 h-12 text-blue-500 mx-auto mb-4 animate-spin" />
                <p className="text-sm text-neutral-600 dark:text-neutral-400">Loading papers...</p>
              </div>
            </div>
          )}

          {/* Empty State */}
          {!isLoading && filteredPapers.length === 0 && (
            <div className="card-base p-12">
              <div className="text-center">
                <IconFileText className="w-12 h-12 text-neutral-400 mx-auto mb-4" />
                <h3 className="text-lg font-semibold text-neutral-900 dark:text-white mb-2">No papers found</h3>
                <p className="text-sm text-neutral-600 dark:text-neutral-400 mb-6">
                  {searchQuery ? "Try adjusting your search query" : "Get started by adding your first paper"}
                </p>
                {!searchQuery && (
                  <Button onClick={() => setIsCreateDialogOpen(true)} className="flex items-center gap-2 mx-auto">
                    <IconPlus className="w-4 h-4" />
                    Add Your First Paper
                  </Button>
                )}
              </div>
            </div>
          )}

          {/* Papers List */}
          {filteredPapers.length > 0 && (
            <div className="card-base overflow-hidden">
              <div className="divide-y divide-neutral-200 dark:divide-neutral-700">
                {filteredPapers.map((paper) => {
                  const { status, progress } = getPaperStatus(paper)
                  const statusStyles = {
                    analyzed: "bg-green-50 dark:bg-green-950/30 text-green-700 dark:text-green-300",
                    processing: "bg-blue-50 dark:bg-blue-950/30 text-blue-700 dark:text-blue-300",
                    queued: "bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-300",
                  }

                  return (
                    <div
                      key={paper.id}
                      onClick={() => setSelectedPaper(paper.id)}
                      className="p-4 hover:bg-neutral-50 dark:hover:bg-neutral-800/50 cursor-pointer transition-colors"
                    >
                      <div className="flex items-start justify-between">
                        <div className="flex items-start gap-4 flex-1">
                          <div className="w-10 h-10 rounded-lg bg-blue-50 dark:bg-blue-950/30 flex items-center justify-center flex-shrink-0">
                            <IconFileText className="w-5 h-5 text-blue-600 dark:text-blue-400" />
                          </div>
                          <div className="flex-1 min-w-0">
                            <h3 className="font-semibold text-neutral-900 dark:text-white text-balance">
                              {paper.title}
                            </h3>
                            <div className="flex items-center gap-3 mt-1 text-sm text-neutral-600 dark:text-neutral-400 flex-wrap">
                              {paper.metadata.authors && paper.metadata.authors.length > 0 && (
                                <>
                                  <span>{paper.metadata.authors.slice(0, 3).join(", ")}{paper.metadata.authors.length > 3 ? " et al." : ""}</span>
                                  <span>•</span>
                                </>
                              )}
                              {paper.metadata.year && (
                                <span className="text-xs px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-700">
                                  {paper.metadata.year}
                                </span>
                              )}
                              {paper.metadata.keywords && paper.metadata.keywords.length > 0 && (
                                <>
                                  <span>•</span>
                                  <span className="text-xs">{paper.metadata.keywords.slice(0, 2).join(", ")}</span>
                                </>
                              )}
                            </div>
                            {status === "processing" && (
                              <div className="mt-3">
                                <div className="flex items-center justify-between mb-1">
                                  <span className="text-xs text-neutral-500 dark:text-neutral-400">Processing</span>
                                  <span className="text-xs font-semibold text-neutral-900 dark:text-white">
                                    {progress}%
                                  </span>
                                </div>
                                <div className="w-full bg-neutral-200 dark:bg-neutral-600 rounded-full h-1.5">
                                  <div
                                    className="bg-blue-500 h-full rounded-full transition-all duration-300"
                                    style={{ width: `${progress}%` }}
                                  ></div>
                                </div>
                              </div>
                            )}
                          </div>
                        </div>
                        <div className="text-right ml-4 flex-shrink-0 space-y-2">
                          <span
                            className={`inline-block px-2 py-1 rounded text-xs font-semibold capitalize ${statusStyles[status as keyof typeof statusStyles]}`}
                          >
                            {status}
                          </span>
                          <div className="text-xs text-neutral-500 dark:text-neutral-400">
                            <p>{paper.segments?.length || 0} segments</p>
                            {paper.algorithms && paper.algorithms.length > 0 && (
                              <p>{paper.algorithms.length} algorithms</p>
                            )}
                          </div>
                        </div>
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          )}
        </div>
      )}
      
      <PaperUploadDialog 
        open={isUploadDialogOpen} 
        onOpenChange={setIsUploadDialogOpen}
        onUploadComplete={handleUploadComplete}
      />
    </div>
  )
}
