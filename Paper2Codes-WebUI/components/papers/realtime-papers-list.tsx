/**
 * Real-time papers list component with WebSocket integration
 * Displays live paper processing updates
 */

'use client';

import { usePapers } from '@/lib/hooks/use-papers';
import { useWebSocket } from '@/lib/hooks/use-websocket';
import {
  IconFileText,
  IconCheckCircle,
  IconAlertCircle,
  IconClock,
  IconLoader,
  IconWifi,
  IconWifiOff,
} from '@/components/ui/icons';
import { formatDistanceToNow } from 'date-fns';
import type { Paper } from '@/lib/api/types';

interface RealtimePapersListProps {
  maxPapers?: number;
  showConnectionStatus?: boolean;
}

export function RealtimePapersList({
  maxPapers = 20,
  showConnectionStatus = true,
}: RealtimePapersListProps) {
  const { papers, isLoading, error } = usePapers({ enabled: true });
  const { isConnected, connectionState } = useWebSocket({
    channels: ['papers'],
    enabled: true,
  });

  // Get papers sorted by updated_at (most recent first) and limit
  const displayedPapers = papers
    .sort((a, b) => {
      // Sort by creation date, most recent first
      const aTime = new Date(a.metadata?.year || 0).getTime();
      const bTime = new Date(b.metadata?.year || 0).getTime();
      return bTime - aTime;
    })
    .slice(0, maxPapers);

  const getStatusConfig = (paper: Paper) => {
    // Determine status based on paper state
    const hasSegments = paper.segments && paper.segments.length > 0;
    const hasAlgorithms = paper.algorithms && paper.algorithms.length > 0;

    if (hasSegments && hasAlgorithms) {
      return {
        icon: IconCheckCircle,
        color: 'text-green-600 dark:text-green-400',
        bgColor: 'bg-green-500/10',
        borderColor: 'border-green-200/50 dark:border-green-800/50',
        label: 'Processed',
      };
    }
    if (hasSegments) {
      return {
        icon: IconLoader,
        color: 'text-blue-600 dark:text-blue-400',
        bgColor: 'bg-blue-500/10',
        borderColor: 'border-blue-200/50 dark:border-blue-800/50',
        label: 'Processing',
        pulse: true,
      };
    }
    return {
      icon: IconClock,
      color: 'text-amber-600 dark:text-amber-400',
      bgColor: 'bg-amber-500/10',
      borderColor: 'border-amber-200/50 dark:border-amber-800/50',
      label: 'Pending',
    };
  };

  if (isLoading && papers.length === 0) {
    return (
      <div className="rounded-xl border border-border bg-card shadow-sm p-6 lg:p-8">
        <div className="flex items-center justify-center py-12">
          <div className="text-center">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
            <p className="text-sm text-muted-foreground">Loading papers...</p>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="rounded-xl border border-border bg-card shadow-sm p-6 lg:p-8">
        <div className="flex items-center justify-center py-12">
          <div className="text-center">
            <IconAlertCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <p className="text-sm text-red-600 dark:text-red-400">
              Failed to load papers: {error.message}
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-xl border border-border bg-card shadow-sm hover:shadow-md transition-shadow duration-300">
      <div className="p-6 lg:p-8">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center">
              <IconFileText className="w-5 h-5 text-primary" />
            </div>
            <div>
              <h2 className="text-xl font-bold tracking-tight text-foreground">
                Papers
              </h2>
              <p className="text-xs text-muted-foreground mt-0.5">
                Real-time paper processing
              </p>
            </div>
          </div>
          {showConnectionStatus && (
            <div className="flex items-center gap-2">
              {isConnected ? (
                <>
                  <IconWifi className="w-4 h-4 text-green-500" />
                  <span className="text-xs text-muted-foreground">Connected</span>
                </>
              ) : (
                <>
                  <IconWifiOff className="w-4 h-4 text-amber-500" />
                  <span className="text-xs text-muted-foreground">
                    {connectionState === 'connecting' ? 'Connecting...' : 'Disconnected'}
                  </span>
                </>
              )}
            </div>
          )}
        </div>

        {displayedPapers.length === 0 ? (
          <div className="text-center py-12">
            <IconFileText className="w-12 h-12 text-muted-foreground/50 mx-auto mb-4" />
            <p className="text-sm text-muted-foreground">No papers found</p>
          </div>
        ) : (
          <div className="space-y-3">
            {displayedPapers.map((paper) => {
              const config = getStatusConfig(paper);
              const StatusIcon = config.icon;
              const segmentsCount = paper.segments?.length || 0;
              const algorithmsCount = paper.algorithms?.length || 0;

              return (
                <div
                  key={paper.id}
                  className={`group relative overflow-hidden rounded-xl border ${config.borderColor} bg-gradient-to-br from-muted/30 to-transparent p-4 transition-all duration-300 hover:shadow-md hover:scale-[1.01]`}
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex items-start gap-3 flex-1 min-w-0">
                      <div
                        className={`w-10 h-10 rounded-lg ${config.bgColor} flex items-center justify-center flex-shrink-0 ${
                          config.pulse ? 'animate-pulse' : ''
                        }`}
                      >
                        <StatusIcon className={`w-5 h-5 ${config.color}`} />
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="font-semibold text-foreground text-sm leading-tight mb-1 truncate">
                          {paper.title}
                        </p>
                        <div className="flex items-center gap-2 mt-1 flex-wrap">
                          {paper.metadata?.authors && (
                            <>
                              <span className="text-xs text-muted-foreground">
                                {paper.metadata.authors.slice(0, 2).join(', ')}
                                {paper.metadata.authors.length > 2 && ' et al.'}
                              </span>
                              <span className="text-xs text-muted-foreground">•</span>
                            </>
                          )}
                          {paper.metadata?.year && (
                            <span className="text-xs text-muted-foreground">
                              {paper.metadata.year}
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-4 mt-2">
                          <div className="flex items-center gap-1">
                            <span className="text-xs text-muted-foreground">
                              {segmentsCount} segments
                            </span>
                          </div>
                          {algorithmsCount > 0 && (
                            <div className="flex items-center gap-1">
                              <span className="text-xs text-muted-foreground">
                                {algorithmsCount} algorithms
                              </span>
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-start gap-2 flex-shrink-0">
                      <span
                        className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold ${config.color} ${config.bgColor} border ${config.borderColor}`}
                      >
                        {config.label}
                      </span>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

