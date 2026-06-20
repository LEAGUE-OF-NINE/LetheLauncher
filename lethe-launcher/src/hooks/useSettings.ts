import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ModInfo } from '../types/launcher';

interface Settings {
  skipValidation: boolean;
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings>({ skipValidation: false });
  const [isOpen, setIsOpen] = useState(false);
  const [mods, setMods] = useState<ModInfo[]>([]);

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

  const refreshMods = useCallback(async () => {
    try {
      const list = await invoke<ModInfo[]>('get_mods');
      setMods(list);
    } catch (e) {
      console.error('Failed to load mods:', e);
      setMods([]);
    }
  }, []);

  const toggleMod = useCallback(async (name: string) => {
    try {
      const updated = await invoke<ModInfo>('toggle_mod', { name });
      setMods((prev) => prev.map((m) => (m.name === updated.name ? updated : m)));
    } catch (e) {
      console.error('Failed to toggle mod:', e);
    }
  }, []);

  const enableAllMods = useCallback(async () => {
    const disabled = mods.filter((m) => !m.enabled);
    await Promise.allSettled(
      disabled.map((m) => invoke<ModInfo>('toggle_mod', { name: m.name }))
    );
    await refreshMods();
  }, [mods, refreshMods]);

  const disableAllMods = useCallback(async () => {
    const enabled = mods.filter((m) => m.enabled);
    await Promise.allSettled(
      enabled.map((m) => invoke<ModInfo>('toggle_mod', { name: m.name }))
    );
    await refreshMods();
  }, [mods, refreshMods]);

  // Load mods on mount
  useEffect(() => {
    refreshMods();
  }, [refreshMods]);

  return { settings, mods, isOpen, setIsOpen, toggleValidation, refreshMods, toggleMod, enableAllMods, disableAllMods };
}
