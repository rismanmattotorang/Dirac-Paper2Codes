"use client"

import { useState, useRef, useEffect } from "react"
import Editor, { Monaco } from "@monaco-editor/react"
import { useTheme } from "next-themes"
import {
  IconDownload,
  IconCopy,
  IconSave,
  IconCheck,
  IconAlertCircle,
  IconLoader,
} from "@/components/ui/icons"
import { Button } from "@/components/ui/button"
import { toast } from "sonner"

export interface CodeEditorProps {
  code: string
  language: string
  filePath: string
  moduleId: string
  readOnly?: boolean
  onSave?: (code: string) => Promise<void>
}

export function CodeEditor({
  code: initialCode,
  language,
  filePath,
  moduleId,
  readOnly = false,
  onSave,
}: CodeEditorProps) {
  const [code, setCode] = useState(initialCode)
  const [isModified, setIsModified] = useState(false)
  const [isSaving, setIsSaving] = useState(false)
  const [copied, setCopied] = useState(false)
  const editorRef = useRef<any>(null)
  const { theme } = useTheme()

  useEffect(() => {
    setCode(initialCode)
    setIsModified(false)
  }, [initialCode])

  const handleEditorDidMount = (editor: any, monaco: Monaco) => {
    editorRef.current = editor

    // Configure Monaco editor
    monaco.editor.defineTheme("custom-dark", {
      base: "vs-dark",
      inherit: true,
      rules: [],
      colors: {
        "editor.background": "#0a0a0a",
      },
    })

    monaco.editor.defineTheme("custom-light", {
      base: "vs",
      inherit: true,
      rules: [],
      colors: {
        "editor.background": "#ffffff",
      },
    })

    // Set the theme
    if (theme === "dark") {
      monaco.editor.setTheme("custom-dark")
    } else {
      monaco.editor.setTheme("custom-light")
    }
  }

  useEffect(() => {
    if (editorRef.current) {
      const monaco = editorRef.current.getModel()?.getLanguageId()
      if (monaco) {
        editorRef.current.updateOptions({
          theme: theme === "dark" ? "custom-dark" : "custom-light",
        })
      }
    }
  }, [theme])

  const handleChange = (value: string | undefined) => {
    if (value !== undefined) {
      setCode(value)
      setIsModified(value !== initialCode)
    }
  }

  const handleSave = async () => {
    if (!onSave || !isModified) return

    setIsSaving(true)
    try {
      await onSave(code)
      setIsModified(false)
      toast.success("Code saved successfully")
    } catch (error) {
      toast.error("Failed to save code")
      console.error("Save error:", error)
    } finally {
      setIsSaving(false)
    }
  }

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(code)
      setCopied(true)
      toast.success("Code copied to clipboard")
      setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      toast.error("Failed to copy code")
    }
  }

  const handleDownload = () => {
    try {
      const blob = new Blob([code], { type: "text/plain" })
      const url = URL.createObjectURL(blob)
      const a = document.createElement("a")
      a.href = url
      a.download = filePath.split("/").pop() || "code.txt"
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      toast.success("File downloaded")
    } catch (error) {
      toast.error("Failed to download file")
    }
  }

  const handleFormat = () => {
    if (editorRef.current) {
      editorRef.current.getAction("editor.action.formatDocument")?.run()
      toast.success("Code formatted")
    }
  }

  return (
    <div className="flex flex-col h-full border border-neutral-200 dark:border-neutral-700 rounded-lg overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center justify-between bg-neutral-50 dark:bg-neutral-800 border-b border-neutral-200 dark:border-neutral-700 px-4 py-2">
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-neutral-900 dark:text-white">{filePath}</span>
          {isModified && (
            <span className="text-xs px-2 py-1 rounded bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300">
              Modified
            </span>
          )}
          {readOnly && (
            <span className="text-xs px-2 py-1 rounded bg-neutral-200 dark:bg-neutral-700 text-neutral-600 dark:text-neutral-400">
              Read-only
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleFormat}
            title="Format code"
            disabled={readOnly}
          >
            <svg
              className="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 6h16M4 12h16M4 18h7"
              />
            </svg>
          </Button>

          <Button variant="ghost" size="sm" onClick={handleCopy} title="Copy code">
            {copied ? <IconCheck className="w-4 h-4" /> : <IconCopy className="w-4 h-4" />}
          </Button>

          <Button variant="ghost" size="sm" onClick={handleDownload} title="Download file">
            <IconDownload className="w-4 h-4" />
          </Button>

          {!readOnly && onSave && (
            <Button
              variant="default"
              size="sm"
              onClick={handleSave}
              disabled={!isModified || isSaving}
              title="Save changes"
            >
              {isSaving ? (
                <>
                  <IconLoader className="w-4 h-4 mr-2 animate-spin" />
                  Saving...
                </>
              ) : (
                <>
                  <IconSave className="w-4 h-4 mr-2" />
                  Save
                </>
              )}
            </Button>
          )}
        </div>
      </div>

      {/* Editor */}
      <div className="flex-1 overflow-hidden">
        <Editor
          height="100%"
          language={language}
          value={code}
          onChange={handleChange}
          onMount={handleEditorDidMount}
          theme={theme === "dark" ? "custom-dark" : "custom-light"}
          options={{
            readOnly,
            minimap: { enabled: true },
            fontSize: 14,
            lineNumbers: "on",
            renderWhitespace: "selection",
            scrollBeyondLastLine: false,
            automaticLayout: true,
            tabSize: 2,
            wordWrap: "on",
            formatOnPaste: true,
            formatOnType: true,
          }}
        />
      </div>
    </div>
  )
}

