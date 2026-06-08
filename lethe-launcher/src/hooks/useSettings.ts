import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface Settings {
  skipValidation: boolean;
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings>({ skipValidation: false });
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    invoke<Record<string, string>>('get_settings')
      .then((config) => {
        setSettings({
          skipValidation: (config['DisableAutoUpdate'] || 'false').toLowerCase() === 'true',
        });
      })
      .catch(console.error);
  }, []);

  const toggleValidation = useCallback(async () => {
    const newValue = !settings.skipValidation;
    await invoke('set_setting', { key: 'DisableAutoUpdate', value: String(newValue) });
    setSettings((prev) => ({ ...prev, skipValidation: newValue }));
  }, [settings.skipValidation]);

  return { settings, isOpen, setIsOpen, toggleValidation };
}
