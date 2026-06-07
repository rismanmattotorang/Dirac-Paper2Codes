/**
 * Helper utilities for presenting API errors to the user
 */

import { ApiClientError } from './errors';

export interface FormattedApiError {
  title: string;
  description: string;
  status?: number;
  code: string;
  requestId?: string;
}

const extractRequestId = (details: unknown): string | undefined => {
  if (!details || typeof details !== 'object') {
    return undefined;
  }

  const payload = details as Record<string, unknown>;
  if (typeof payload.requestId === 'string') {
    return payload.requestId;
  }
  if (typeof payload.request_id === 'string') {
    return payload.request_id;
  }

  if (payload.meta && typeof payload.meta === 'object') {
    const meta = payload.meta as Record<string, unknown>;
    if (typeof meta.request_id === 'string') {
      return meta.request_id;
    }
  }
  return undefined;
};

const sanitizeMessage = (message: unknown): string | undefined => {
  if (!message) return undefined;
  if (typeof message === 'string') return message.trim();
  try {
    return JSON.stringify(message);
  } catch {
    return undefined;
  }
};

export function formatApiError(error: ApiClientError): FormattedApiError {
  const status = error.statusCode;
  const code = error.code || 'UNKNOWN_ERROR';
  const baseMessage = sanitizeMessage(error.message) || 'An unexpected error occurred';
  const requestId =
    error.requestId ??
    extractRequestId(error.details) ??
    (typeof (error.details as any)?.meta === 'object'
      ? (error.details as any)?.meta?.request_id
      : undefined);

  let title = 'Request Failed';
  let description = baseMessage;

  switch (code) {
    case 'NETWORK_ERROR': {
      const endpoint =
        typeof error.details === 'object' && error.details
          ? (error.details as Record<string, unknown>).endpoint
          : undefined;
      title = 'Cannot Reach Paper2Codes Core';
      description =
        typeof endpoint === 'string'
          ? `Unable to connect to the backend while calling ${endpoint}. Please verify that the Paper2Codes Core service is running and reachable.`
          : `Unable to connect to the backend service. Please verify that the Paper2Codes Core service is running and reachable.`;
      break;
    }
    case 'TIMEOUT': {
      const timeoutMs =
        typeof error.details === 'object' && error.details
          ? (error.details as Record<string, unknown>).timeoutMs
          : undefined;
      title = 'Request Timed Out';
      if (typeof timeoutMs === 'number' && Number.isFinite(timeoutMs)) {
        description = `The request took longer than ${timeoutMs}ms and was aborted. Please try again.`;
      } else {
        description = 'The request took too long to complete and was aborted. Please try again.';
      }
      break;
    }
    case 'STORAGE_NOT_ENABLED': {
      title = 'Storage Service Unavailable';
      description =
        'Persistent storage is disabled in the Paper2Codes Core configuration. Enable storage in the core service to use this feature.';
      break;
    }
    case 'VALIDATION_ERROR': {
      title = 'Validation Error';
      description =
        baseMessage ||
        'One or more inputs were invalid. Please review the form and try again.';
      break;
    }
    default: {
      if (status === 404) {
        title = 'Resource Not Found';
        description =
          baseMessage ||
          'The requested resource could not be found. It may have been removed or never existed.';
      } else if (status === 401 || status === 403) {
        title = 'Authentication Required';
        description =
          baseMessage ||
          'You do not have permission to perform this action. Please sign in and try again.';
      } else if (status && status >= 500) {
        title = 'Paper2Codes Core Error';
        description =
          baseMessage ||
          'The backend encountered an error while processing the request. Please try again later.';
      }
    }
  }

  return {
    title,
    description,
    status,
    code,
    requestId,
  };
}
