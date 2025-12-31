import { RefreshCw } from 'lucide-react';
import { IndexStatus as IndexStatusType } from '@/types';
import { cn } from '@/lib/utils';

interface IndexStatusProps {
  status: IndexStatusType;
  onRebuild: () => void;
}

export function IndexStatus({ status, onRebuild }: IndexStatusProps) {
  const statusConfig = {
    ready: {
      label: 'Ready',
      color: 'bg-emerald-500',
      textColor: 'text-emerald-500',
    },
    indexing: {
      label: 'Indexing...',
      color: 'bg-amber-500',
      textColor: 'text-amber-500',
    },
    error: {
      label: 'Error',
      color: 'bg-destructive',
      textColor: 'text-destructive',
    },
  };

  const config = statusConfig[status];

  return (
    <div className="flex items-center justify-between px-3 py-2 border-t border-sidebar-border">
      <div className="flex items-center gap-2">
        <span
          className={cn(
            'h-2 w-2 rounded-full',
            config.color,
            status === 'indexing' && 'animate-pulse'
          )}
        />
        <span className={cn('text-xs font-medium', config.textColor)}>
          {config.label}
        </span>
      </div>
      <button
        onClick={onRebuild}
        disabled={status === 'indexing'}
        className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors disabled:opacity-50"
      >
        <RefreshCw
          className={cn('h-3 w-3', status === 'indexing' && 'animate-spin')}
        />
        Rebuild
      </button>
    </div>
  );
}
