/**
 * API error handling utilities
 */

import type { ApiError } from './types';

export class ApiClientError extends Error {
  constructor(
    public code: string,
    message: string,
    public statusCode?: number,
    public details?: unknown,
    public requestId?: string,
  ) {
    super(message);
    this.name = 'ApiClientError';
    if (!this.requestId) {
      this.requestId = ApiClientError.extractRequestId(details);
    }
  }

  private static extractRequestId(details: unknown): string | undefined {
    if (!details || typeof details !== 'object') {
      return undefined;
    }

    const detailObj = details as Record<string, unknown>;
    if (typeof detailObj.requestId === 'string') {
      return detailObj.requestId;
    }
    if (typeof detailObj.request_id === 'string') {
      return detailObj.request_id;
    }
    if (detailObj.meta && typeof detailObj.meta === 'object') {
      const meta = detailObj.meta as Record<string, unknown>;
      if (typeof meta.request_id === 'string') {
        return meta.request_id;
      }
    }
    return undefined;
  }

  static fromResponse(
    error: ApiError,
    statusCode: number,
    requestId?: string,
  ): ApiClientError {
    // Handle different error response formats
    if (error?.error) {
      // Extract the actual error message, removing misleading prefixes
      let message = error.error.message || 'Request failed';
      
      // Clean up config error messages that are actually validation errors
      if (message.includes('Invalid configuration:') && message.includes('multipart')) {
        // Extract the actual error after "Invalid configuration:"
        const match = message.match(/Invalid configuration: (.+)/);
        if (match && match[1]) {
          message = match[1];
        }
      }
      
      let mergedDetails: Record<string, unknown> | undefined;
      if (error.error.details && typeof error.error.details === 'object') {
        mergedDetails = { ...(error.error.details as Record<string, unknown>) };
      } else if (error.error.details !== undefined) {
        mergedDetails = { detail: error.error.details };
      }

      if (error.meta) {
        mergedDetails = {
          ...(mergedDetails ?? {}),
          meta: error.meta,
          request_id: error.meta.request_id,
        };
      }

      return new ApiClientError(
        error.error.code || `HTTP_${statusCode}`,
        message,
        statusCode,
        mergedDetails,
        requestId || error.meta?.request_id,
      );
    }
    
    // Fallback for unexpected error formats
    const errorMessage = 
      (error as any)?.message || 
      (error as any)?.error?.message ||
      `Request failed with status ${statusCode}`;
    
    const fallbackDetails =
      (error as any)?.meta || (error as any)?.details
        ? {
            meta: (error as any)?.meta,
            details: (error as any)?.details,
          }
        : error;

    return new ApiClientError(
      (error as any)?.code || `HTTP_${statusCode}`,
      errorMessage,
      statusCode,
      fallbackDetails,
      requestId || (fallbackDetails as any)?.meta?.request_id,
    );
  }

  static fromNetworkError(error: Error): ApiClientError {
    return new ApiClientError(
      'NETWORK_ERROR',
      `Network error: ${error.message}`,
      undefined,
      { originalError: error.message },
    );
  }

  static fromParseError(error: Error): ApiClientError {
    return new ApiClientError(
      'PARSE_ERROR',
      `Failed to parse response: ${error.message}`,
      undefined,
      { originalError: error.message },
    );
  }
}

/**
 * Check if an error is retryable
 */
export function isRetryableError(error: ApiClientError): boolean {
  // Retry on network errors and 5xx server errors
  if (!error.statusCode) {
    return true; // Network errors are retryable
  }

  // Retry on server errors (500-599)
  if (error.statusCode >= 500 && error.statusCode < 600) {
    return true;
  }

  // Retry on rate limiting (429)
  if (error.statusCode === 429) {
    return true;
  }

  // Don't retry on client errors (400-499) except 429
  return false;
}

/**
 * Check if an error is non-retryable
 */
export function isNonRetryableError(error: unknown): boolean {
  if (!(error instanceof ApiClientError)) {
    return true;
  }

  return !isRetryableError(error);
}
