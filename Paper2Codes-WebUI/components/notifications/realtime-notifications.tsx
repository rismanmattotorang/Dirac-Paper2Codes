/**
 * Real-time notification system component
 * Displays toast notifications for WebSocket events with customizable messages
 */

'use client';

import { useEffect, useRef } from 'react';
import { useWebSocket } from '@/lib/hooks/use-websocket';
import { toast } from 'sonner';
import type { WebSocketMessage } from '@/lib/hooks/use-websocket';

interface RealtimeNotificationsProps {
  enabled?: boolean;
  channels?: string[];
  /**
   * Custom notification message formatters
   */
  messageFormatters?: {
    taskUpdate?: (data: unknown) => { title: string; description: string; duration?: number };
    taskCreated?: (data: unknown) => { title: string; description: string; duration?: number };
    paperProcessed?: (data: unknown) => { title: string; description: string; duration?: number };
    generationProgress?: (data: unknown) => { title: string; description: string; duration?: number };
    verificationComplete?: (data: unknown) => { title: string; description: string; duration?: number };
    error?: (data: unknown) => { title: string; description: string; duration?: number };
  };
}

/**
 * Default message formatters
 */
const defaultFormatters = {
  taskUpdate: (data: unknown) => {
    const taskData = data as {
      task_id?: string;
      status?: unknown;
      progress?: number;
      message?: string;
    };

    const taskId = taskData.task_id?.slice(0, 8) || 'Unknown';
    const progress = taskData.progress ? `${Math.round(taskData.progress)}%` : '';
    const statusValue =
      typeof taskData.status === 'string'
        ? taskData.status
        : taskData.status
        ? JSON.stringify(taskData.status)
        : '';
    const normalizedStatus = statusValue.toLowerCase();

    if (normalizedStatus.includes('completed')) {
      return {
        title: 'Task Completed',
        description: taskData.message || `Task ${taskId} has been completed successfully${progress ? ` (${progress})` : ''}`,
        duration: 5000,
      };
    }
    if (normalizedStatus.includes('inprogress') || normalizedStatus.includes('in_progress') || normalizedStatus.includes('progress')) {
      return {
        title: 'Task In Progress',
        description: taskData.message || `Task ${taskId} is now running${progress ? ` (${progress})` : ''}`,
        duration: 3000,
      };
    }
    if (normalizedStatus.includes('failed')) {
      return {
        title: 'Task Failed',
        description:
          taskData.message ||
          (typeof taskData.status === 'object' && taskData.status !== null && 'Failed' in (taskData.status as Record<string, unknown>)
            ? `Task ${taskId} has failed: ${(taskData.status as { Failed: string }).Failed}`
            : `Task ${taskId} has failed`),
        duration: 7000,
      };
    }
    return {
      title: 'Task Updated',
      description: taskData.message || `Task ${taskId} status updated`,
      duration: 4000,
    };
  },

  taskCreated: (data: unknown) => {
    const taskData = data as {
      task_id?: string;
      description?: string;
      task_type?: string;
    };
    return {
      title: 'New Task Created',
      description: taskData.description || `A new ${taskData.task_type || 'task'} has been created`,
      duration: 4000,
    };
  },

  paperProcessed: (data: unknown) => {
    const paperData = data as {
      paper_id?: string;
      title?: string;
      status?: string;
      segments_count?: number;
      message?: string;
    };
    const paperId = paperData.paper_id?.slice(0, 8) || 'Unknown';
    const segmentsInfo = paperData.segments_count ? ` (${paperData.segments_count} segments)` : '';
    return {
      title: 'Paper Processed',
      description: paperData.message || paperData.title || `Paper ${paperId} has been processed${segmentsInfo}`,
      duration: 5000,
    };
  },

  generationProgress: (data: unknown) => {
    const genData = data as {
      repository_id?: string;
      paper_id?: string;
      progress?: number;
      current_module?: string;
      modules_completed?: number;
      modules_total?: number;
      message?: string;
    };
    const repoId = genData.repository_id?.slice(0, 8) || 'Unknown';
    const progress = genData.progress ? `${Math.round(genData.progress)}%` : '';
    const moduleInfo = genData.current_module
      ? ` - ${genData.current_module}`
      : genData.modules_completed && genData.modules_total
        ? ` (${genData.modules_completed}/${genData.modules_total} modules)`
        : '';

    if (genData.progress === 100) {
      return {
        title: 'Code Generation Complete',
        description: genData.message || `Repository ${repoId} generation completed successfully`,
        duration: 5000,
      };
    }
    if (genData.progress && genData.progress > 0) {
      return {
        title: 'Generation Progress',
        description: genData.message || `Generating ${repoId}${progress ? ` - ${progress}` : ''}${moduleInfo}`,
        duration: 3000,
      };
    }
    return {
      title: 'Generation Started',
      description: genData.message || `Code generation started for repository ${repoId}`,
      duration: 3000,
    };
  },

  verificationComplete: (data: unknown) => {
    const verifyData = data as {
      repository_id?: string;
      passed?: boolean;
      issues_count?: number;
      message?: string;
    };
    const repoId = verifyData.repository_id?.slice(0, 8) || 'Unknown';
    const issuesInfo = verifyData.issues_count !== undefined
      ? ` (${verifyData.issues_count} issue${verifyData.issues_count !== 1 ? 's' : ''})`
      : '';

    if (verifyData.passed) {
      return {
        title: 'Verification Passed',
        description: verifyData.message || `Repository ${repoId} verification passed${issuesInfo}`,
        duration: 5000,
      };
    }
    return {
      title: 'Verification Failed',
      description: verifyData.message || `Repository ${repoId} verification failed${issuesInfo}`,
      duration: 7000,
    };
  },

  error: (data: unknown) => {
    const errorData = data as {
      message?: string;
      code?: string;
      details?: unknown;
    };
    return {
      title: 'Error Occurred',
      description: errorData.message || `An error occurred${errorData.code ? ` (${errorData.code})` : ''}`,
      duration: 7000,
    };
  },
};

export function RealtimeNotifications({
  enabled = true,
  channels = ['tasks', 'papers', 'repositories'],
  messageFormatters = {},
}: RealtimeNotificationsProps) {
  const { messages, isConnected } = useWebSocket({
    channels,
    enabled,
  });

  const processedMessagesRef = useRef<Set<string>>(new Set());
  const formatters = { ...defaultFormatters, ...messageFormatters };

  useEffect(() => {
    if (!enabled || !isConnected) {
      return;
    }

    messages.forEach((message: WebSocketMessage) => {
      // Create unique message ID to prevent duplicate notifications
      const messageId = `${message.type}-${message.timestamp}-${JSON.stringify(message.data)}`;
      
      if (processedMessagesRef.current.has(messageId)) {
        return;
      }
      processedMessagesRef.current.add(messageId);

      // Clean up old message IDs to prevent memory issues
      if (processedMessagesRef.current.size > 100) {
        const firstId = processedMessagesRef.current.values().next().value as string | undefined;
        if (firstId) {
          processedMessagesRef.current.delete(firstId);
        }
      }

      // Handle different message types with custom formatters
      switch (message.type) {
      case 'task_update': {
        const formatter = formatters.taskUpdate || defaultFormatters.taskUpdate;
        const { title, description, duration } = formatter(message.data);
        const taskData = message.data as { status?: unknown };
        const statusValue =
          typeof taskData.status === 'string'
            ? taskData.status
            : taskData.status
            ? JSON.stringify(taskData.status)
            : '';
        const normalizedStatus = statusValue.toLowerCase();

        if (normalizedStatus.includes('failed')) {
          toast.error(title, { description, duration });
        } else if (normalizedStatus.includes('completed')) {
          toast.success(title, { description, duration });
        } else {
          toast.info(title, { description, duration });
        }
        break;
      }

        case 'task_created': {
          const formatter = formatters.taskCreated || defaultFormatters.taskCreated;
          const { title, description, duration } = formatter(message.data);
          toast.info(title, { description, duration });
          break;
        }

        case 'paper_processed': {
          const formatter = formatters.paperProcessed || defaultFormatters.paperProcessed;
          const { title, description, duration } = formatter(message.data);
          toast.success(title, { description, duration });
          break;
        }

        case 'generation_progress': {
          const formatter = formatters.generationProgress || defaultFormatters.generationProgress;
          const { title, description, duration } = formatter(message.data);
          const genData = message.data as { progress?: number };
          
          if (genData.progress === 100) {
            toast.success(title, { description, duration });
          } else if (genData.progress && genData.progress > 0) {
            toast.info(title, { description, duration });
          } else {
            toast.info(title, { description, duration });
          }
          break;
        }

        case 'verification_complete': {
          const formatter = formatters.verificationComplete || defaultFormatters.verificationComplete;
          const { title, description, duration } = formatter(message.data);
          const verifyData = message.data as { passed?: boolean };
          
          if (verifyData.passed) {
            toast.success(title, { description, duration });
          } else {
            toast.warning(title, { description, duration });
          }
          break;
        }

        case 'error': {
          const formatter = formatters.error || defaultFormatters.error;
          const { title, description, duration } = formatter(message.data);
          toast.error(title, { description, duration });
          break;
        }

        default:
          // Unknown message type - log but don't show notification
          console.debug('Unknown WebSocket message type:', message.type);
      }
    });
  }, [messages, isConnected, enabled, formatters]);

  // This component doesn't render anything visible
  return null;
}
