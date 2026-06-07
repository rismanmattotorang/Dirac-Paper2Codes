/**
 * Real-time repositories list component with WebSocket integration
 * Displays live code generation and verification updates
 */

'use client';

import { useRepositories } from '@/lib/hooks/use-repositories';
import { useWebSocket } from '@/lib/hooks/use-websocket';
import {
  IconCode,
  IconCheckCircle,
  IconAlertCircle,
  IconClock,
  IconLoader,
  IconWifi,
  IconWifiOff,
  IconGitBranch,
} from '@/components/ui/icons';
import { formatDistanceToNow } from 'date-fns';
import type { Repository } from '@/lib/api/types';

interface RealtimeRepositoriesListProps {
  maxRepositories?: number;
  showConnectionStatus?: boolean;
}

export function RealtimeRepositoriesList({
  maxRepositories = 20,
  showConnectionStatus = true,
}: RealtimeRepositoriesListProps) {
  const { repositories, isLoading, error } = useRepositories({ enabled: true });
  const { isConnected, connectionState } = useWebSocket({
    channels: ['repositories'],
    enabled: true,
  });

  // Get repositories sorted by updated_at (most recent first) and limit
  const displayedRepositories = repositories
    .sort((a, b) => {
      const aTime = new Date(a.metadata?.updated_at || 0).getTime();
      const bTime = new Date(b.metadata?.updated_at || 0).getTime();
      return bTime - aTime;
    })
    .slice(0, maxRepositories);

  const getStatusConfig = (repository: Repository) => {
    // Determine status based on repository state
    const modulesCount = repository.modules?.length || 0;
    const hasModules = modulesCount > 0;

    if (hasModules) {
      return {
        icon: IconCheckCircle,
        color: 'text-green-600 dark:text-green-400',
        bgColor: 'bg-green-500/10',
        borderColor: 'border-green-200/50 dark:border-green-800/50',
        label: 'Generated',
      };
    }
    return {
      icon: IconLoader,
      color: 'text-blue-600 dark:text-blue-400',
      bgColor: 'bg-blue-500/10',
      borderColor: 'border-blue-200/50 dark:border-blue-800/50',
      label: 'Generating',
      pulse: true,
    };
  };

  if (isLoading && repositories.length === 0) {
    return (
      <div className="rounded-xl border border-border bg-card shadow-sm p-6 lg:p-8">
        <div className="flex items-center justify-center py-12">
          <div className="text-center">
            <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary mb-4"></div>
            <p className="text-sm text-muted-foreground">Loading repositories...</p>
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
              Failed to load repositories: {error.message}
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
              <IconGitBranch className="w-5 h-5 text-primary" />
            </div>
            <div>
              <h2 className="text-xl font-bold tracking-tight text-foreground">
                Repositories
              </h2>
              <p className="text-xs text-muted-foreground mt-0.5">
                Real-time code generation
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

        {displayedRepositories.length === 0 ? (
          <div className="text-center py-12">
            <IconCode className="w-12 h-12 text-muted-foreground/50 mx-auto mb-4" />
            <p className="text-sm text-muted-foreground">No repositories found</p>
          </div>
        ) : (
          <div className="space-y-3">
            {displayedRepositories.map((repository) => {
              const config = getStatusConfig(repository);
              const StatusIcon = config.icon;
              const modulesCount = repository.modules?.length || 0;
              const filesCount = repository.structure?.files?.length || 0;

              return (
                <div
                  key={repository.id}
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
                          {repository.root_path || `Repository ${repository.id.slice(0, 8)}`}
                        </p>
                        <div className="flex items-center gap-2 mt-1 flex-wrap">
                          {repository.metadata?.created_at && (
                            <span className="text-xs text-muted-foreground">
                              Created{' '}
                              {formatDistanceToNow(
                                new Date(repository.metadata.created_at),
                                { addSuffix: true },
                              )}
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-4 mt-2">
                          <div className="flex items-center gap-1">
                            <IconCode className="w-3 h-3 text-muted-foreground" />
                            <span className="text-xs text-muted-foreground">
                              {modulesCount} modules
                            </span>
                          </div>
                          {filesCount > 0 && (
                            <div className="flex items-center gap-1">
                              <span className="text-xs text-muted-foreground">
                                {filesCount} files
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

