import { X, Monitor, Moon, Sun } from 'lucide-react';
import { Settings } from '@/types';
import { useTheme } from '@/contexts/ThemeContext';
import { Button } from '@/components/ui/button';
import { Slider } from '@/components/ui/slider';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/utils';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  settings: Settings;
  onUpdateSettings: (updates: Partial<Settings>) => void;
}

export function SettingsModal({
  isOpen,
  onClose,
  settings,
  onUpdateSettings,
}: SettingsModalProps) {
  const { theme, setTheme } = useTheme();

  if (!isOpen) return null;

  const themeOptions = [
    { value: 'light' as const, label: 'Light', icon: Sun },
    { value: 'dark' as const, label: 'Dark', icon: Moon },
    { value: 'system' as const, label: 'System', icon: Monitor },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div
        className="absolute inset-0 bg-background/80 backdrop-blur-sm"
        onClick={onClose}
      />
      <div className="relative z-10 w-full max-w-md rounded-xl border bg-card p-6 shadow-xl">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-lg font-semibold">Settings</h2>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="h-8 w-8"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        <div className="space-y-6">
          {/* Appearance */}
          <div className="space-y-3">
            <Label className="text-sm font-medium">Appearance</Label>
            <div className="flex gap-2">
              {themeOptions.map((option) => (
                <button
                  key={option.value}
                  onClick={() => setTheme(option.value)}
                  className={cn(
                    'flex flex-1 flex-col items-center gap-2 rounded-lg border p-3 transition-colors',
                    theme === option.value
                      ? 'border-primary bg-primary/10'
                      : 'border-border hover:bg-muted'
                  )}
                >
                  <option.icon className="h-5 w-5" />
                  <span className="text-xs font-medium">{option.label}</span>
                </button>
              ))}
            </div>
          </div>

          <Separator />

          {/* Model Settings */}
          <div className="space-y-4">
            <Label className="text-sm font-medium">Model Settings</Label>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-xs text-muted-foreground">
                  Temperature
                </Label>
                <span className="text-xs font-mono">
                  {settings.temperature.toFixed(2)}
                </span>
              </div>
              <Slider
                value={[settings.temperature]}
                onValueChange={([value]) =>
                  onUpdateSettings({ temperature: value })
                }
                min={0}
                max={1}
                step={0.01}
                className="w-full"
              />
            </div>

            <div className="space-y-2">
              <Label className="text-xs text-muted-foreground">
                Max Tokens
              </Label>
              <Input
                type="number"
                value={settings.maxTokens}
                onChange={(e) =>
                  onUpdateSettings({ maxTokens: parseInt(e.target.value) || 0 })
                }
                min={256}
                max={8192}
                className="h-9"
              />
            </div>
          </div>

          <Separator />

          {/* RAG Settings */}
          <div className="space-y-4">
            <Label className="text-sm font-medium">RAG Settings</Label>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-xs text-muted-foreground">
                  Chunk Size
                </Label>
                <span className="text-xs font-mono">{settings.chunkSize}</span>
              </div>
              <Slider
                value={[settings.chunkSize]}
                onValueChange={([value]) =>
                  onUpdateSettings({ chunkSize: value })
                }
                min={128}
                max={2048}
                step={64}
                className="w-full"
              />
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label className="text-xs text-muted-foreground">
                  Top-K Results
                </Label>
                <span className="text-xs font-mono">{settings.topK}</span>
              </div>
              <Slider
                value={[settings.topK]}
                onValueChange={([value]) => onUpdateSettings({ topK: value })}
                min={1}
                max={20}
                step={1}
                className="w-full"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
