import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { UpdateInfo } from '../types/updater';

interface UseUpdaterReturn {
  updateAvailable: boolean;
  updateInfo: UpdateInfo | null;
  isDownloading: boolean;
  downloadError: string | null;
  checkForUpdates: () => Promise<void>;
  applyUpdate: () => Promise<void>;
  dismissUpdate: () => void;
}

export function useUpdater(): UseUpdaterReturn {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);

  const checkForUpdates = useCallback(async () => {
    try {
      const info = await invoke<UpdateInfo | null>('check_for_updates');
      if (info) {
        setUpdateInfo(info);
        setDismissed(false);
      }
    } catch (err) {
      // Silently fail - update checks shouldn't block the launcher
      console.error('Update check failed:', err);
    }
  }, []);

  // Check on mount
  useEffect(() => {
    checkForUpdates();
  }, [checkForUpdates]);

  const applyUpdate = useCallback(async () => {
    if (!updateInfo) return;
    setIsDownloading(true);
    setDownloadError(null);
    try {
      await invoke('download_update', { update: updateInfo });
      // The update script will restart the app - we'll be killed
    } catch (err) {
      setDownloadError(String(err));
      setIsDownloading(false);
    }
  }, [updateInfo]);

  const dismissUpdate = useCallback(() => {
    setDismissed(true);
  }, []);

  return {
    updateAvailable: updateInfo !== null && !dismissed,
    updateInfo,
    isDownloading,
    downloadError,
    checkForUpdates,
    applyUpdate,
    dismissUpdate,
  };
}
