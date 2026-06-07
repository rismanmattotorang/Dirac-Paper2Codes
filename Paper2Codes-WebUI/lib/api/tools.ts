/**
 * Tools API client
 */

import { apiClient } from "./client"
import type { ApiResponse, ToolInfo } from "./types"

/**
 * Retrieve available tool metadata from the backend.
 */
export async function getTools(): Promise<ApiResponse<ToolInfo[]>> {
  return apiClient.get("/api/tools")
}

