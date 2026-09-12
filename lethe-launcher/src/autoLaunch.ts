import type { SyncPhase } from './types/launcher';

export function shouldAutoLaunch(
  isAuthenticated: boolean,
  skipValidation: boolean,
  phase: SyncPhase,
) {
  return isAuthenticated && (phase === 'complete' || (skipValidation && phase === 'idle'));
}
