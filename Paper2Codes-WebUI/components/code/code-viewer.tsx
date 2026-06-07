"use client"

import { IconCopy, IconDownload, IconCheckCircle, IconAlertCircle, IconBarChart } from "@/components/ui/icons"

interface CodeViewerProps {
  selectedFile: string | null
  files: Array<{
    id: string
    name: string
    status: string
    lines: number
    testsPassed: number
    coverage: number
  }>
}

export function CodeViewer({ selectedFile, files }: CodeViewerProps) {
  const file = files.find((f) => f.id === selectedFile)
  if (!file) return null

  return (
    <div className="card-base overflow-hidden flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-neutral-200 dark:border-neutral-700 p-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <h3 className="font-semibold text-neutral-900 dark:text-white">{file.name}</h3>
          <span className="text-xs px-2 py-1 rounded bg-neutral-100 dark:bg-neutral-700 text-neutral-600 dark:text-neutral-400">
            {file.lines} lines
          </span>
          {file.status === "verified" ? (
            <span className="inline-flex items-center gap-1 text-xs px-2 py-1 rounded bg-green-50 dark:bg-green-950/30 text-green-700 dark:text-green-300">
              <IconCheckCircle className="w-3 h-3" />
              Verified
            </span>
          ) : (
            <span className="inline-flex items-center gap-1 text-xs px-2 py-1 rounded bg-amber-50 dark:bg-amber-950/30 text-amber-700 dark:text-amber-300">
              <IconAlertCircle className="w-3 h-3" />
              Warnings
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <button className="p-2 hover:bg-neutral-100 dark:hover:bg-neutral-700 rounded-lg transition-colors">
            <IconCopy className="w-4 h-4 text-neutral-600 dark:text-neutral-400" />
          </button>
          <button className="p-2 hover:bg-neutral-100 dark:hover:bg-neutral-700 rounded-lg transition-colors">
            <IconDownload className="w-4 h-4 text-neutral-600 dark:text-neutral-400" />
          </button>
        </div>
      </div>

      {/* Code Preview */}
      <div className="flex-1 overflow-auto font-mono text-sm bg-neutral-900 dark:bg-black p-4">
        <div className="space-y-1 text-neutral-300">
          {[
            "import torch",
            "import torch.nn as nn",
            "",
            "class Attention(nn.Module):",
            "    def __init__(self, dim, heads=8):",
            "        super().__init__()",
            "        self.heads = heads",
            "        self.dim = dim",
            "",
            "    def forward(self, x):",
            "        # Multi-head attention",
            "        q, k, v = ...",
            "        return output",
          ].map((line, idx) => (
            <div key={idx} className="flex items-baseline gap-4">
              <span className="text-neutral-600 w-8 text-right select-none">{idx + 1}</span>
              <span>{line}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Stats Footer */}
      <div className="border-t border-neutral-200 dark:border-neutral-700 p-4 grid grid-cols-3 gap-4">
        {[
          { icon: IconCheckCircle, label: "Tests Passed", value: `${file.testsPassed}/15` },
          { icon: IconBarChart, label: "Code Coverage", value: `${file.coverage}%` },
          { icon: IconAlertCircle, label: "Issues", value: file.status === "verified" ? "0" : "2" },
        ].map((stat, idx) => {
          const Icon = stat.icon
          return (
            <div key={idx} className="flex items-center gap-2">
              <Icon className="w-4 h-4 text-neutral-500 dark:text-neutral-400" />
              <div>
                <p className="text-xs text-neutral-500 dark:text-neutral-400">{stat.label}</p>
                <p className="text-sm font-semibold text-neutral-900 dark:text-white">{stat.value}</p>
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
