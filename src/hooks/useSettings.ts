import { useState, useEffect, useCallback } from 'react';
import { Settings } from '@/types';

const defaultSettings: Settings = {
  temperature: 0.7,
  maxTokens: 2048,
  chunkSize: 512,
  topK: 5,
  theme: 'system',
};

const STORAGE_KEY = 'knowledge-assistant-settings';

export function useSettings() {
  const [settings, setSettingsState] = useState<Settings>(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try {
        return { ...defaultSettings, ...JSON.parse(stored) };
      } catch {
        return defaultSettings;
      }
    }
    return defaultSettings;
  });

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  }, [settings]);

  const updateSettings = useCallback((updates: Partial<Settings>) => {
    setSettingsState(prev => ({ ...prev, ...updates }));
  }, []);

  const resetSettings = useCallback(() => {
    setSettingsState(defaultSettings);
  }, []);

  return {
    settings,
    updateSettings,
    resetSettings,
  };
}
