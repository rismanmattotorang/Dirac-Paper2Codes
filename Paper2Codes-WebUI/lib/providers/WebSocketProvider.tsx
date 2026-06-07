"use client"

import { createContext, useContext, useEffect, useState, useCallback } from "react"
import { useWebSocket, type WebSocketMessage } from "@/lib/hooks/use-websocket"
import { useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { config, WS_BASE_URL } from "@/lib/config"

interface WebSocketContextValue {
  isConnected: boolean
  lastMessage: WebSocketMessage | null
  sendMessage: (message: unknown) => void
}

const WebSocketContext = createContext<WebSocketContextValue | undefined>(undefined)

export function useWebSocketContext() {
  const context = useContext(WebSocketContext)
  if (!context) {
    throw new Error("useWebSocketContext must be used within WebSocketProvider")
  }
  return context
}

interface WebSocketProviderProps {
  children: React.ReactNode
  url?: string
}

export function WebSocketProvider({ children, url }: WebSocketProviderProps) {
  const queryClient = useQueryClient()
  const [connectionNotified, setConnectionNotified] = useState(false)

  // Get WebSocket URL from config or prop
  const wsUrl = url || WS_BASE_URL

  const realTimeEnabled = config.features.realTime
  const realtimeConfig = config.realtime

  const handleMessage = (message: WebSocketMessage) => {
    console.log("WebSocket message received:", message)

    // Handle different message types - match backend message types
    switch (message.type) {
      case "task_update":
        // Invalidate tasks queries
        queryClient.invalidateQueries({ queryKey: ["tasks"] })
        break

      case "paper_processed":
      case "paper_update":
        // Invalidate papers queries
        queryClient.invalidateQueries({ queryKey: ["papers"] })
        break

      case "generation_progress":
      case "repository_update":
        // Invalidate repositories queries
        queryClient.invalidateQueries({ queryKey: ["repositories"] })
        break

      case "module_update":
        // Invalidate modules queries
        queryClient.invalidateQueries({ queryKey: ["modules"] })
        queryClient.invalidateQueries({ queryKey: ["module-content"] })
        break

      case "verification_complete":
        // Invalidate verification queries
        queryClient.invalidateQueries({ queryKey: ["module-verification"] })
        toast.success("Code verification completed")
        break

      case "analytics_update":
        // Invalidate analytics queries
        queryClient.invalidateQueries({ queryKey: ["analytics-overview"] })
        queryClient.invalidateQueries({ queryKey: ["usage-statistics"] })
        queryClient.invalidateQueries({ queryKey: ["agent-performance"] })
        break

      case "error":
        const errorData = message.data as { message?: string; code?: string } | undefined
        const errorMessage = errorData?.message || errorData?.code || "Unknown error"
        toast.error(`Error: ${errorMessage}`)
        break

      case "heartbeat":
        // Ignore heartbeat messages
        break

      case "subscribed":
      case "unsubscribed":
        // Ignore subscription confirmations
        break

      default:
        console.log("Unhandled message type:", message.type)
    }
  }

  const {
    isConnected,
    connectionState,
    messages,
    send,
    lastError,
  } = useWebSocket({
    url: wsUrl,
    onMessage: handleMessage,
    autoReconnect: realTimeEnabled && realtimeConfig.autoReconnect,
    reconnectInterval: realtimeConfig.reconnectIntervalMs,
    maxReconnectAttempts: realtimeConfig.maxReconnectAttempts,
    enabled: realTimeEnabled,
  })

  useEffect(() => {
    if (connectionState === "connected" && !connectionNotified) {
      console.log("WebSocket: Real-time updates connected")
      setConnectionNotified(true)
    } else if (connectionState === "disconnected" && connectionNotified) {
      console.log("WebSocket: Real-time updates disconnected")
      setConnectionNotified(false)
    }
  }, [connectionState, connectionNotified])

  useEffect(() => {
    if (lastError && process.env.NODE_ENV === "development") {
      console.warn("WebSocket connection error - will retry automatically")
    }
  }, [lastError])

  const lastMessage = messages.length > 0 ? messages[messages.length - 1] : null
  const sendMessage = useCallback(
    (message: unknown) => {
      send(message)
    },
    [send],
  )

  return (
    <WebSocketContext.Provider value={{ isConnected, lastMessage, sendMessage }}>
      {children}
    </WebSocketContext.Provider>
  )
}
