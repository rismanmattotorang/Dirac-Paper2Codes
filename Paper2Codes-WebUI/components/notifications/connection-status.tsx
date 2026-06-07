/**
 * Connection status indicator component
 * Shows WebSocket connection state
 */

'use client';

import { useWebSocket } from '@/lib/hooks/use-websocket';
import { IconWifi, IconWifiOff, IconLoader2 } from '@/components/ui/icons';
import { cn } from '@/lib/utils';

interface ConnectionStatusProps {
  className?: string;
  showLabel?: boolean;
}

export function ConnectionStatus({
  className,
  showLabel = true,
}: ConnectionStatusProps) {
  const { isConnected, isConnecting, connectionState } = useWebSocket({
    enabled: true,
  });

  const getStatusConfig = () => {
    if (isConnected) {
      return {
        icon: IconWifi,
        label: 'Connected',
        color: 'text-green-600 dark:text-green-400',
        bgColor: 'bg-green-500/10',
      };
    }
    if (isConnecting || connectionState === 'connecting') {
      return {
        icon: IconLoader2,
        label: 'Connecting...',
        color: 'text-amber-600 dark:text-amber-400',
        bgColor: 'bg-amber-500/10',
      };
    }
    return {
      icon: IconWifiOff,
      label: 'Disconnected',
      color: 'text-red-600 dark:text-red-400',
      bgColor: 'bg-red-500/10',
    };
  };

  const config = getStatusConfig();
  const Icon = config.icon;

  return (
    <div
      className={cn(
        'flex items-center gap-2 px-3 py-1.5 rounded-lg',
        config.bgColor,
        className,
      )}
    >
      <Icon
        className={cn(
          'w-4 h-4',
          config.color,
          isConnecting && 'animate-spin',
        )}
      />
      {showLabel && (
        <span className={cn('text-xs font-medium', config.color)}>
          {config.label}
        </span>
      )}
    </div>
  );
}

