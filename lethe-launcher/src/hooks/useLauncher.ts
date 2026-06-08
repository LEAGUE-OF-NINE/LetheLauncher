import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  CheckProgressPayload,
  DownloadProgressPayload,
  StatusChangePayload,
  ErrorPayload,
  SyncPhase,
} from '../types/launcher';

interface LauncherState {
  phase: SyncPhase;
  statusMessage: string;
  checkProgress: CheckProgressPayload | null;
  downloadProgress: DownloadProgressPayload | null;
  currentFile: string;
  error: string | null;
}

export function useLauncher(authToken: string | null) {
  const [state, setState] = useState<LauncherState>({
    phase: 'idle',
    statusMessage: 'Initializing...',
    checkProgress: null,
    downloadProgress: null,
    currentFile: '',
    error: null,
  });

  // Keep authToken in a ref so the sync-complete listener always has the latest
  const tokenRef = useRef(authToken);
  tokenRef.current = authToken;

  const startSync = useCallback(async () => {
    try {
      setState((prev) => ({
        ...prev,
        phase: 'manifest',
        statusMessage: 'Connecting...',
        error: null,
      }));

      await invoke('start_sync');
    } catch (err) {
      setState((prev) => ({
        ...prev,
        phase: 'error',
        error: String(err),
        statusMessage: 'An error occurred',
      }));
    }
  }, []);

  const launchGame = useCallback(async (token?: string | null) => {
    const t = token ?? tokenRef.current;
    setState((prev) => ({
      ...prev,
      phase: 'launching',
      statusMessage: 'Starting game...',
    }));
    try {
      await invoke('launch', { token: t });
    } catch (err) {
      setState((prev) => ({
        ...prev,
        phase: 'error',
        error: String(err),
        statusMessage: 'Failed to launch game',
      }));
    }
  }, []);

  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    listen<StatusChangePayload>('status-change', (event) => {
      setState((prev) => {
        const msg = event.payload.message.toLowerCase();
        let phase: SyncPhase = prev.phase;
        if (msg.includes('downloading manifest')) phase = 'manifest';
        else if (msg.includes('checking')) phase = 'checking';
        else if (msg.includes('downloading')) phase = 'downloading';
        else if (msg.includes('complete') || msg.includes('up to date')) phase = 'complete';

        return { ...prev, phase, statusMessage: event.payload.message };
      });
    }).then((fn) => unlisteners.push(fn));

    listen<CheckProgressPayload>('check-progress', (event) => {
      setState((prev) => ({
        ...prev,
        phase: 'checking',
        checkProgress: event.payload,
        currentFile: event.payload.current_file,
      }));
    }).then((fn) => unlisteners.push(fn));

    listen<DownloadProgressPayload>('download-progress', (event) => {
      setState((prev) => ({
        ...prev,
        phase: 'downloading',
        downloadProgress: event.payload,
        currentFile: event.payload.current_file,
      }));
    }).then((fn) => unlisteners.push(fn));

    listen<void>('sync-complete', () => {
      setState((prev) => ({
        ...prev,
        phase: 'complete',
        statusMessage: 'All files up to date',
      }));
    }).then((fn) => unlisteners.push(fn));

    listen<ErrorPayload>('error', (event) => {
      setState((prev) => ({
        ...prev,
        phase: 'error',
        error: event.payload.message,
        statusMessage: 'An error occurred',
      }));
    }).then((fn) => unlisteners.push(fn));

    return () => { unlisteners.forEach((fn) => fn()); };
  }, []);

  return { ...state, startSync, launchGame };
}
