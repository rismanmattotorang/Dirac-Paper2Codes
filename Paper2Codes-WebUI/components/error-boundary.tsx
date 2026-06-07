"use client"

import React from "react"
import { IconAlertCircle } from "@/components/ui/icons"
import { Button } from "@/components/ui/button"

interface ErrorBoundaryState {
  hasError: boolean
  error: Error | null
  errorInfo: React.ErrorInfo | null
}

interface ErrorBoundaryProps {
  children: React.ReactNode
  fallback?: React.ComponentType<{ error: Error; resetError: () => void }>
}

export class ErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
    }
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      hasError: true,
      error,
    }
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("ErrorBoundary caught an error:", error, errorInfo)
    this.setState({
      error,
      errorInfo,
    })
  }

  resetError = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
    })
  }

  render() {
    if (this.state.hasError && this.state.error) {
      if (this.props.fallback) {
        const Fallback = this.props.fallback
        return <Fallback error={this.state.error} resetError={this.resetError} />
      }

      return (
        <div className="min-h-screen flex items-center justify-center bg-background p-4">
          <div className="max-w-md w-full space-y-4">
            <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-6">
              <div className="flex items-start gap-4">
                <IconAlertCircle className="w-6 h-6 text-destructive flex-shrink-0 mt-0.5" />
                <div className="flex-1 space-y-2">
                  <h2 className="text-lg font-semibold text-foreground">
                    Something went wrong
                  </h2>
                  <p className="text-sm text-muted-foreground">
                    {this.state.error.message || "An unexpected error occurred"}
                  </p>
                  {process.env.NODE_ENV === "development" && this.state.errorInfo && (
                    <details className="mt-4">
                      <summary className="text-xs text-muted-foreground cursor-pointer">
                        Error details
                      </summary>
                      <pre className="mt-2 text-xs bg-muted p-2 rounded overflow-auto max-h-40">
                        {this.state.error.stack}
                        {"\n\n"}
                        {this.state.errorInfo.componentStack}
                      </pre>
                    </details>
                  )}
                </div>
              </div>
            </div>
            <div className="flex gap-2">
              <Button onClick={this.resetError} variant="outline" className="flex-1">
                Try again
              </Button>
              <Button
                onClick={() => window.location.reload()}
                variant="default"
                className="flex-1"
              >
                Reload page
              </Button>
            </div>
          </div>
        </div>
      )
    }

    return this.props.children
  }
}

