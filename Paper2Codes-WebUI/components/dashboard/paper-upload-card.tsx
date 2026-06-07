"use client"

import type React from "react"
import { useState, useCallback } from "react"
import { IconUpload, IconFileText, IconLoader, IconFile, IconAlertCircle, IconCheckCircle } from "@/components/ui/icons"
import { useRecentPapers } from "@/lib/hooks/use-dashboard"
import { uploadPaper, processPaper } from "@/lib/api/papers"
import { getSelectedSkill } from "@/lib/skill-selection"
import type { Paper } from "@/lib/api/types"

const getPaperStatusLabel = (paper: Paper): string => {
  const hasSegments = Array.isArray(paper.segments) && paper.segments.length > 0
  const hasAlgorithms = Array.isArray(paper.algorithms) && paper.algorithms.length > 0
  const hasEmbeddings =
    Array.isArray(paper.segments) &&
    paper.segments.some((segment) => Array.isArray(segment.embedding) && segment.embedding.length > 0)

  if (hasSegments && hasAlgorithms && hasEmbeddings) {
    return "Analyzed"
  }
  if (hasSegments) {
    return "Processing"
  }
  return "Queued"
}

export function PaperUploadCard() {
  const [isLoading, setIsLoading] = useState(false)
  const [isDragActive, setIsDragActive] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)
  const [uploadProgress, setUploadProgress] = useState(0)
  
  const { papers, loading: papersLoading } = useRecentPapers(2)

  const handleDrag = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    if (e.type === "dragenter" || e.type === "dragover") {
      setIsDragActive(true)
    } else if (e.type === "dragleave") {
      setIsDragActive(false)
    }
  }

  const uploadFile = useCallback(async (file: File) => {
    if (!file.type.includes('pdf')) {
      setError('Please upload a PDF file')
      return
    }

    if (file.size > 50 * 1024 * 1024) {
      setError('File size must be less than 50MB')
      return
    }

    setIsLoading(true)
    setError(null)
    setSuccess(null)
    setUploadProgress(0)

    try {
      // Upload file using API client
      const response = await uploadPaper(file, {
        name: file.name,
        description: `Uploaded ${new Date().toLocaleString()}`,
        onProgress: (progress) => {
          setUploadProgress(progress)
        },
      })

      if (!response.success) {
        throw new Error(response?.meta?.request_id ? `Upload failed (request ${response.meta.request_id})` : 'Upload failed')
      }

      const uploadedPaper = response.data
      setSuccess(`Successfully uploaded ${uploadedPaper.title || file.name}`)
      setUploadProgress(100)

      const selectedSkill = getSelectedSkill()
      processPaper(uploadedPaper.id, {
        skillId: selectedSkill?.skillId,
        language: selectedSkill?.language,
      }).catch((err) => {
        console.warn('Failed to trigger paper processing:', err)
      })
      
      // Clear success message after 5 seconds
      setTimeout(() => setSuccess(null), 5000)
    } catch (err) {
      console.error('Upload failed:', err)
      setError(err instanceof Error ? err.message : 'Upload failed')
    } finally {
      setIsLoading(false)
      setTimeout(() => setUploadProgress(0), 1000)
    }
  }, [])

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragActive(false)
    
    const file = e.dataTransfer.files?.[0]
    if (file) {
      uploadFile(file)
    }
  }

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      uploadFile(file)
    }
  }

  return (
    <div className="group relative overflow-hidden rounded-xl border border-border bg-card shadow-sm hover:shadow-lg transition-all duration-300">
      <div className="p-6 lg:p-8">
        {/* Header */}
        <div className="mb-6">
          <h2 className="text-xl font-bold tracking-tight text-foreground mb-1">Upload Research Paper</h2>
          <p className="text-sm text-muted-foreground">Upload PDF files to begin the code generation process</p>
        </div>

        {/* Upload Area */}
        <div
          onDragEnter={handleDrag}
          onDragLeave={handleDrag}
          onDragOver={handleDrag}
          onDrop={handleDrop}
          className={`relative rounded-xl border-2 border-dashed transition-all duration-300 ${
            isDragActive
              ? "border-primary bg-primary/5 scale-[1.02] shadow-lg ring-2 ring-primary/20"
              : "border-muted-foreground/25 hover:border-primary/50 hover:bg-muted/30 focus-within:border-primary/50 focus-within:ring-2 focus-within:ring-primary/20"
          }`}
          role="region"
          aria-label="File upload area"
        >
          <div className="px-6 py-16 flex flex-col items-center justify-center">
            {isLoading ? (
              <div className="flex flex-col items-center gap-4">
                <div className="relative">
                  <div className="w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center">
                    <IconLoader className="w-8 h-8 text-primary animate-spin" />
                  </div>
                  <div className="absolute inset-0 rounded-2xl bg-primary/20 animate-ping" />
                </div>
                <div className="text-center w-full max-w-xs">
                  <p className="font-semibold text-foreground">Uploading paper...</p>
                  <p className="text-sm text-muted-foreground mt-1">{Math.round(uploadProgress)}%</p>
                  <div className="mt-3 w-full bg-muted rounded-full h-2">
                    <div 
                      className="bg-primary h-2 rounded-full transition-all duration-300"
                      style={{ width: `${uploadProgress}%` }}
                    />
                  </div>
                </div>
              </div>
            ) : success ? (
              <div className="flex flex-col items-center gap-4">
                <div className="w-16 h-16 rounded-2xl bg-green-500/10 flex items-center justify-center">
                  <IconCheckCircle className="w-8 h-8 text-green-600 dark:text-green-400" />
                </div>
                <div className="text-center">
                  <p className="font-semibold text-foreground">Upload successful!</p>
                  <p className="text-sm text-muted-foreground mt-1">{success}</p>
                </div>
              </div>
            ) : error ? (
              <div className="flex flex-col items-center gap-4">
                <div className="w-16 h-16 rounded-2xl bg-red-500/10 flex items-center justify-center">
                  <IconAlertCircle className="w-8 h-8 text-red-600 dark:text-red-400" />
                </div>
                <div className="text-center">
                  <p className="font-semibold text-foreground">Upload failed</p>
                  <p className="text-sm text-muted-foreground mt-1">{error}</p>
                </div>
              </div>
            ) : (
              <>
                <div
                  className={`w-16 h-16 rounded-2xl bg-gradient-to-br from-primary/10 to-primary/20 flex items-center justify-center mb-4 transition-transform duration-300 ${
                    isDragActive ? "scale-110" : "group-hover:scale-105"
                  }`}
                >
                  <IconUpload className="w-8 h-8 text-primary" />
                </div>
                <div className="text-center space-y-2">
                  <p className="font-semibold text-foreground text-lg">
                    {isDragActive ? "Drop your PDF here" : "Drag and drop your PDF here"}
                  </p>
                  <p className="text-sm text-muted-foreground">or click to browse files</p>
                  <p className="text-xs text-muted-foreground/70 mt-2">Supports PDF files up to 50MB</p>
                </div>
              </>
            )}

          <input
            type="file"
            accept=".pdf"
            className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
            onChange={handleFileSelect}
            aria-label="Upload PDF file"
            disabled={isLoading}
          />
          </div>
        </div>

        {/* Recent Papers */}
        <div className="mt-8">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-semibold text-foreground">Recent Papers</h3>
            <button
              className="text-xs text-muted-foreground hover:text-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 rounded px-1"
              aria-label="View all recent papers"
              type="button"
            >
              View all
            </button>
          </div>
          
          {papersLoading ? (
            <div className="space-y-2">
              {[1, 2].map((i) => (
                <div key={i} className="p-3 rounded-lg bg-muted/50 animate-pulse">
                  <div className="h-4 bg-muted rounded w-3/4 mb-2" />
                  <div className="h-3 bg-muted rounded w-1/2" />
                </div>
              ))}
            </div>
          ) : papers.length === 0 ? (
            <div className="text-center py-8 text-sm text-muted-foreground">
              No recent papers
            </div>
          ) : (
            <div className="space-y-2">
              {papers.map((paper) => {
                const statusLabel = getPaperStatusLabel(paper)
                const segmentsCount = paper.segments?.length ?? 0
                const algorithmsCount = paper.algorithms?.length ?? 0
                return (
                  <div
                    key={paper.id}
                    className="group/item flex items-center gap-3 p-3 rounded-lg bg-muted/50 hover:bg-muted border border-transparent hover:border-border transition-all duration-200 cursor-pointer"
                  >
                    <div className="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0 group-hover/item:bg-primary/20 transition-colors">
                      <IconFile className="w-5 h-5 text-primary" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium text-foreground truncate group-hover/item:text-primary transition-colors">
                        {paper.title}
                      </p>
                      <p className="text-xs text-muted-foreground flex items-center gap-2 mt-0.5">
                        <span>{statusLabel}</span>
                        <span aria-hidden="true">•</span>
                        <span>{segmentsCount} segments</span>
                        {algorithmsCount > 0 && (
                          <>
                            <span aria-hidden="true">•</span>
                            <span>{algorithmsCount} algorithms</span>
                          </>
                        )}
                      </p>
                    </div>
                    <IconFileText className="w-4 h-4 text-muted-foreground opacity-0 group-hover/item:opacity-100 transition-opacity" />
                  </div>
                )
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
