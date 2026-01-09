import { Settings } from 'lucide-react';
import { ModelStatus } from '@/types';
import { Button } from '@/components/ui/button';
import { ThemeToggle } from './ThemeToggle';
import { cn } from '@/lib/utils';

interface ChatHeaderProps {
  modelStatus: ModelStatus;
  onOpenSettings: () => void;
}

export function ChatHeader({ modelStatus, onOpenSettings }: ChatHeaderProps) {
  const statusConfig = {
    not_loaded: { label: 'Model Not Loaded', color: 'bg-gray-400' },
    loading: { label: 'Loading Model...', color: 'bg-amber-500' },
    ready: { label: 'Model Ready', color: 'bg-emerald-500' },
    error: { label: 'Model Error', color: 'bg-destructive' },
  };

  const config = statusConfig[modelStatus];

  return (
    <header className="flex items-center justify-between border-b px-4 py-3 bg-background">
      <h1 className="text-lg font-semibold">Knowledge Assistant</h1>

      <div className="flex items-center gap-2">
        <span
          className={cn(
            'h-2 w-2 rounded-full',
            config.color,
            modelStatus === 'loading' && 'animate-pulse'
          )}
        />
        <span className="text-sm text-muted-foreground">{config.label}</span>
      </div>

      <div className="flex items-center gap-1">
        <ThemeToggle />
        <Button
          variant="ghost"
          size="icon"
          onClick={onOpenSettings}
          className="h-9 w-9 text-muted-foreground hover:text-foreground"
        >
          <Settings className="h-5 w-5" />
          <span className="sr-only">Settings</span>
        </Button>
      </div>
    </header>
  );
}
