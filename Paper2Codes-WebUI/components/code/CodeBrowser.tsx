"use client"

import { useState, useEffect, type ReactNode } from "react"
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query"
import {
  IconFileText,
  IconCheckCircle,
  IconAlertCircle,
  IconFolder,
  IconChevronRight,
  IconChevronDown,
  IconSearch,
  IconLoader,
  IconRefresh,
} from "@/components/ui/icons"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { toast } from "sonner"
import {
  getRepositories,
  getRepositoryModules,
  getModuleContent,
  updateModuleContent,
  getModuleVerification,
  type ModuleContent,
} from "@/lib/api/repositories"
import { CodeEditor } from "./CodeEditor"

interface FileNode {
  id: string
  name: string
  path: string
  type: "file" | "directory"
  language?: string
  children?: FileNode[]
  moduleId?: string
}

export function CodeBrowser() {
  const [selectedFile, setSelectedFile] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set())
  const queryClient = useQueryClient()

  // Fetch repositories
  const { data: reposData, isLoading: reposLoading } = useQuery({
    queryKey: ["repositories"],
    queryFn: () => getRepositories(1, 100),
  })

  const repositories = reposData?.data || []
  const selectedRepo = repositories[0] // For now, use the first repository

  // Fetch modules for selected repository
  const { data: modulesData, isLoading: modulesLoading } = useQuery({
    queryKey: ["modules", selectedRepo?.id],
    queryFn: () => getRepositoryModules(selectedRepo!.id),
    enabled: !!selectedRepo,
  })

  const modules = modulesData?.data || []

  // Fetch selected module content
  const { data: moduleContentData, isLoading: contentLoading } = useQuery({
    queryKey: ["module-content", selectedFile],
    queryFn: () => getModuleContent(selectedFile!),
    enabled: !!selectedFile,
  })

  const moduleContent = moduleContentData?.data

  // Fetch module verification
  const { data: verificationData } = useQuery({
    queryKey: ["module-verification", selectedFile],
    queryFn: () => getModuleVerification(selectedFile!),
    enabled: !!selectedFile,
    refetchInterval: 5000, // Refresh every 5 seconds
  })

  const verification = verificationData?.data

  // Update module mutation
  const updateMutation = useMutation({
    mutationFn: ({ moduleId, content }: { moduleId: string; content: string }) =>
      updateModuleContent(moduleId, { content }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["module-content", selectedFile] })
      toast.success("Module updated successfully")
    },
    onError: () => {
      toast.error("Failed to update module")
    },
  })

  // Build file tree from modules
  const buildFileTree = (): FileNode[] => {
    const tree: FileNode[] = []
    const dirMap = new Map<string, FileNode>()

    modules.forEach((module) => {
      const parts = module.file_path.split("/")
      let currentPath = ""

      parts.forEach((part, index) => {
        const isFile = index === parts.length - 1
        currentPath = currentPath ? `${currentPath}/${part}` : part

        if (isFile) {
          // Add file
          const parentPath = parts.slice(0, -1).join("/")
          const parent = parentPath ? dirMap.get(parentPath) : null

          const fileNode: FileNode = {
            id: module.id,
            name: part,
            path: currentPath,
            type: "file",
            language: getLanguageFromPath(part),
            moduleId: module.id,
          }

          if (parent) {
            parent.children = parent.children || []
            parent.children.push(fileNode)
          } else {
            tree.push(fileNode)
          }
        } else {
          // Add directory
          if (!dirMap.has(currentPath)) {
            const parentPath = parts.slice(0, index).join("/")
            const parent = parentPath ? dirMap.get(parentPath) : null

            const dirNode: FileNode = {
              id: currentPath,
              name: part,
              path: currentPath,
              type: "directory",
              children: [],
            }

            dirMap.set(currentPath, dirNode)

            if (parent) {
              parent.children = parent.children || []
              parent.children.push(dirNode)
            } else {
              tree.push(dirNode)
            }
          }
        }
      })
    })

    return tree
  }

  const getLanguageFromPath = (path: string): string => {
    const ext = path.split(".").pop()?.toLowerCase()
    const langMap: Record<string, string> = {
      py: "python",
      js: "javascript",
      ts: "typescript",
      jsx: "javascript",
      tsx: "typescript",
      rs: "rust",
      go: "go",
      java: "java",
      cpp: "cpp",
      c: "c",
      cs: "csharp",
      rb: "ruby",
      php: "php",
      swift: "swift",
      kt: "kotlin",
    }
    return langMap[ext || ""] || "plaintext"
  }

  const toggleDirectory = (path: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })
  }

  const renderFileTree = (nodes: FileNode[], level = 0): ReactNode => {
    const filteredNodes = searchQuery
      ? nodes.filter(
          (node) =>
            node.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
            (node.children && hasMatchingChildren(node, searchQuery))
        )
      : nodes

    return (
      <div>
        {filteredNodes.map((node) => {
          const isExpanded = expandedDirs.has(node.path)
          const isSelected = node.type === "file" && node.id === selectedFile

          return (
            <div key={node.id}>
              <button
                onClick={() => {
                  if (node.type === "directory") {
                    toggleDirectory(node.path)
                  } else {
                    setSelectedFile(node.id)
                  }
                }}
                className={`w-full text-left px-2 py-1.5 rounded transition-colors flex items-center gap-2 ${
                  isSelected
                    ? "bg-blue-50 dark:bg-blue-950/30 text-blue-600 dark:text-blue-400"
                    : "hover:bg-neutral-100 dark:hover:bg-neutral-700/50 text-neutral-700 dark:text-neutral-300"
                }`}
                style={{ paddingLeft: `${level * 16 + 8}px` }}
              >
                {node.type === "directory" ? (
                  <>
                    {isExpanded ? (
                      <IconChevronDown className="w-4 h-4 flex-shrink-0" />
                    ) : (
                      <IconChevronRight className="w-4 h-4 flex-shrink-0" />
                    )}
                    <IconFolder className="w-4 h-4 flex-shrink-0 text-blue-500" />
                  </>
                ) : (
                  <>
                    <span className="w-4" />
                    <IconFileText className="w-4 h-4 flex-shrink-0 text-neutral-400" />
                  </>
                )}
                <span className="text-sm truncate">{node.name}</span>
              </button>

              {node.type === "directory" && isExpanded && node.children && (
                <div>{renderFileTree(node.children, level + 1)}</div>
              )}
            </div>
          )
        })}
      </div>
    )
  }

  const hasMatchingChildren = (node: FileNode, query: string): boolean => {
    if (!node.children) return false
    return node.children.some(
      (child) =>
        child.name.toLowerCase().includes(query.toLowerCase()) ||
        (child.children && hasMatchingChildren(child, query))
    )
  }

  const handleSave = async (code: string) => {
    if (!selectedFile) return
    await updateMutation.mutateAsync({ moduleId: selectedFile, content: code })
  }

  const fileTree = buildFileTree()

  if (reposLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <IconLoader className="w-8 h-8 animate-spin text-neutral-400" />
      </div>
    )
  }

  if (!selectedRepo) {
    return (
      <div className="p-6 text-center text-neutral-500 dark:text-neutral-400">
        No repositories found. Generate code from a paper first.
      </div>
    )
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-4 gap-6 h-[calc(100vh-12rem)]">
      {/* File Browser Sidebar */}
      <div className="lg:col-span-1">
        <div className="card-base p-4 h-full flex flex-col">
          <div className="mb-3">
            <h3 className="font-semibold text-neutral-900 dark:text-white mb-2">
              File Browser
            </h3>
            <div className="relative">
              <IconSearch className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-neutral-400" />
              <Input
                type="text"
                placeholder="Search files..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-9"
              />
            </div>
          </div>

          <div className="flex-1 overflow-auto">
            {modulesLoading ? (
              <div className="flex items-center justify-center h-32">
                <IconLoader className="w-6 h-6 animate-spin text-neutral-400" />
              </div>
            ) : (
              renderFileTree(fileTree)
            )}
          </div>

          {verification && (
            <div className="mt-3 pt-3 border-t border-neutral-200 dark:border-neutral-700">
              <div className="text-xs space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-neutral-500 dark:text-neutral-400">Tests</span>
                  <span className="font-medium text-neutral-900 dark:text-white">
                    {verification.tests_passed}/{verification.tests_total}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-neutral-500 dark:text-neutral-400">Coverage</span>
                  <span className="font-medium text-neutral-900 dark:text-white">
                    {verification.coverage}%
                  </span>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Code Editor */}
      <div className="lg:col-span-3">
        {selectedFile && moduleContent ? (
          <div className="h-full flex flex-col">
            <CodeEditor
              code={moduleContent.content}
              language={moduleContent.language}
              filePath={moduleContent.file_path}
              moduleId={moduleContent.id}
              readOnly={false}
              onSave={handleSave}
            />

            {/* Verification Status */}
            {verification && verification.issues.length > 0 && (
              <div className="mt-4 card-base p-4">
                <h4 className="font-semibold text-neutral-900 dark:text-white mb-3">
                  Issues ({verification.issues.length})
                </h4>
                <div className="space-y-2 max-h-48 overflow-auto">
                  {verification.issues.map((issue, idx) => (
                    <div
                      key={idx}
                      className="flex items-start gap-2 text-sm p-2 rounded bg-neutral-50 dark:bg-neutral-800"
                    >
                      <IconAlertCircle
                        className={`w-4 h-4 flex-shrink-0 mt-0.5 ${
                          issue.severity === "error"
                            ? "text-red-500"
                            : issue.severity === "warning"
                              ? "text-amber-500"
                              : "text-blue-500"
                        }`}
                      />
                      <div className="flex-1">
                        <p className="text-neutral-900 dark:text-white">{issue.message}</p>
                        <p className="text-xs text-neutral-500 dark:text-neutral-400 mt-1">
                          Line {issue.line}
                          {issue.column && `, Column ${issue.column}`}
                        </p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : contentLoading ? (
          <div className="flex items-center justify-center h-full card-base">
            <IconLoader className="w-8 h-8 animate-spin text-neutral-400" />
          </div>
        ) : (
          <div className="flex items-center justify-center h-full card-base">
            <div className="text-center text-neutral-500 dark:text-neutral-400">
              <IconFileText className="w-12 h-12 mx-auto mb-3 opacity-50" />
              <p>Select a file to view its contents</p>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
