"use client"

import { useState, useCallback } from "react"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { IconUpload, IconFile, IconX, IconLoader, IconCheckCircle } from "@/components/ui/icons"
import { uploadPaper, processPaper } from "@/lib/api/papers"
import { getSelectedSkill } from "@/lib/skill-selection"
import { SelectedSkillBanner } from "@/components/skills/selected-skill-banner"

interface PaperUploadDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onUploadComplete?: () => void
}

export function PaperUploadDialog({ open, onOpenChange, onUploadComplete }: PaperUploadDialogProps) {
  const [file, setFile] = useState<File | null>(null)
  const [uploading, setUploading] = useState(false)
  const [uploadProgress, setUploadProgress] = useState(0)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState(false)
  const [dragActive, setDragActive] = useState(false)

  const handleDrag = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    if (e.type === "dragenter" || e.type === "dragover") {
      setDragActive(true)
    } else if (e.type === "dragleave") {
      setDragActive(false)
    }
  }, [])

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    e.stopPropagation()
    setDragActive(false)
    
    if (e.dataTransfer.files && e.dataTransfer.files[0]) {
      const droppedFile = e.dataTransfer.files[0]
      if (validateFile(droppedFile)) {
        setFile(droppedFile)
        setError(null)
      }
    }
  }, [])

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      const selectedFile = e.target.files[0]
      if (validateFile(selectedFile)) {
        setFile(selectedFile)
        setError(null)
      }
    }
  }

  const validateFile = (file: File): boolean => {
    const validTypes = ["application/pdf", "text/plain"]
    const maxSize = 50 * 1024 * 1024 // 50MB

    if (!validTypes.includes(file.type)) {
      setError("Invalid file type. Please upload a PDF or text file.")
      return false
    }

    if (file.size > maxSize) {
      setError("File too large. Maximum size is 50MB.")
      return false
    }

    return true
  }

  const handleUpload = async () => {
    if (!file) return

    setUploading(true)
    setError(null)
    setUploadProgress(0)

    try {
      const response = await uploadPaper(file, {
        name: file.name,
        description: `Paper uploaded: ${file.name}`,
        onProgress: (progress) => {
          setUploadProgress(Math.round(progress))
        },
      })

      if (!response.success) {
        throw new Error(response?.meta?.request_id ? `Upload failed (request ${response.meta.request_id})` : "Upload failed")
      }

      setSuccess(true)
      setUploadProgress(100)

      // Start backend processing asynchronously (best effort), forwarding the
      // user's selected domain skill so it drives generation.
      const paperId = response.data.id
      const selectedSkill = getSelectedSkill()
      processPaper(paperId, {
        skillId: selectedSkill?.skillId,
        language: selectedSkill?.language,
      }).catch((err) => {
        console.warn("Failed to start paper processing:", err)
      })

      // Reset after a delay
      setTimeout(() => {
        setFile(null)
        setSuccess(false)
        setUploadProgress(0)
        onOpenChange(false)
        onUploadComplete?.()
      }, 1500)
    } catch (err) {
      console.error("Upload failed:", err)
      setError(err instanceof Error ? err.message : "Upload failed. Please try again.")
    } finally {
      setUploading(false)
    }
  }

  const handleClose = () => {
    if (!uploading) {
      setFile(null)
      setError(null)
      setSuccess(false)
      setUploadProgress(0)
      onOpenChange(false)
    }
  }

  const removeFile = () => {
    if (!uploading) {
      setFile(null)
      setError(null)
    }
  }

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Upload Paper</DialogTitle>
          <DialogDescription>
            Upload a research paper (PDF or text file) to analyze
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <SelectedSkillBanner />
          {!file ? (
            <div
              onDragEnter={handleDrag}
              onDragLeave={handleDrag}
              onDragOver={handleDrag}
              onDrop={handleDrop}
              className={`relative border-2 border-dashed rounded-lg p-8 text-center transition-colors ${
                dragActive
                  ? "border-blue-500 bg-blue-50 dark:bg-blue-950/20"
                  : "border-neutral-300 dark:border-neutral-700 hover:border-neutral-400 dark:hover:border-neutral-600"
              }`}
            >
              <input
                type="file"
                id="file-upload"
                accept=".pdf,.txt"
                onChange={handleFileChange}
                className="hidden"
                disabled={uploading}
              />
              <label htmlFor="file-upload" className="cursor-pointer">
                <IconUpload className="w-12 h-12 text-neutral-400 mx-auto mb-4" />
                <p className="text-sm font-medium text-neutral-900 dark:text-white mb-1">
                  Drop your file here, or click to browse
                </p>
                <p className="text-xs text-neutral-500 dark:text-neutral-400">
                  PDF or TXT files up to 50MB
                </p>
              </label>
            </div>
          ) : (
            <div className="border border-neutral-200 dark:border-neutral-700 rounded-lg p-4">
              <div className="flex items-start gap-3">
                <IconFile className="w-10 h-10 text-blue-500 flex-shrink-0" />
                <div className="flex-1 min-w-0">
                  <p className="font-medium text-neutral-900 dark:text-white truncate">{file.name}</p>
                  <p className="text-xs text-neutral-500 dark:text-neutral-400 mt-0.5">
                    {(file.size / 1024 / 1024).toFixed(2)} MB
                  </p>
                  {uploading && (
                    <div className="mt-3 space-y-2">
                      <div className="flex items-center justify-between text-xs">
                        <span className="text-neutral-600 dark:text-neutral-400">
                          {success ? "Complete" : "Uploading..."}
                        </span>
                        <span className="font-semibold text-neutral-900 dark:text-white">
                          {uploadProgress}%
                        </span>
                      </div>
                      <div className="w-full h-2 bg-neutral-200 dark:bg-neutral-700 rounded-full overflow-hidden">
                        <div
                          className="h-full bg-gradient-to-r from-blue-500 to-purple-500 rounded-full transition-all duration-300"
                          style={{ width: `${uploadProgress}%` }}
                        />
                      </div>
                    </div>
                  )}
                </div>
                {!uploading && !success && (
                  <button
                    onClick={removeFile}
                    className="p-1 hover:bg-neutral-100 dark:hover:bg-neutral-800 rounded transition-colors"
                  >
                    <IconX className="w-4 h-4 text-neutral-500" />
                  </button>
                )}
                {success && (
                  <IconCheckCircle className="w-5 h-5 text-green-500 flex-shrink-0" />
                )}
              </div>
            </div>
          )}

          {error && (
            <div className="bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-800 rounded-lg p-3">
              <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
            </div>
          )}

          <div className="flex justify-end gap-3 pt-4">
            <Button variant="outline" onClick={handleClose} disabled={uploading}>
              Cancel
            </Button>
            <Button
              onClick={handleUpload}
              disabled={!file || uploading || success}
              className="flex items-center gap-2"
            >
              {uploading ? (
                <>
                  <IconLoader className="w-4 h-4 animate-spin" />
                  Uploading...
                </>
              ) : success ? (
                <>
                  <IconCheckCircle className="w-4 h-4" />
                  Uploaded
                </>
              ) : (
                <>
                  <IconUpload className="w-4 h-4" />
                  Upload
                </>
              )}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
