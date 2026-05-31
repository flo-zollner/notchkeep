import type { UpdateConsent } from '../settings.svelte';

export type StartupAction = 'show-activation' | 'check' | 'show-update' | 'idle';

/**
 * Pure decision for what the app should do on startup regarding updates.
 * `availableVersion` is null when no update is known yet (pre-check); a string
 * when `check()` already reported an available version.
 */
export function decideStartupAction(
  consent: UpdateConsent,
  availableVersion: string | null,
  skippedVersion: string | null,
): StartupAction {
  if (consent === 'unset') return 'show-activation';
  if (consent === 'declined') return 'idle';
  if (availableVersion === null) return 'check';
  if (availableVersion === skippedVersion) return 'idle';
  return 'show-update';
}
