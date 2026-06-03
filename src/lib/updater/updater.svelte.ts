import { platform } from '@tauri-apps/plugin-os';
import { settings, setUpdateConsent, setSkippedVersion } from '../settings.svelte';
import { decideStartupAction, type StartupAction } from './decide';
import { desktopBackend } from './backend-desktop';
import { androidBackend } from './backend-android';
import { filterByChannel } from './channel';

export type UpdateStatus =
  | 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error';

const backend = platform() === 'android' ? androidBackend : desktopBackend;

export const updateState = $state({
  status: 'idle' as UpdateStatus,
  availableVersion: null as string | null,
  notes: '' as string,
  downloaded: 0,
  total: 0,
  error: '' as string,
  supportsRestart: backend.supportsRestart,
});

/** Startup flow. Returns the resolved action so the layout can render the
 *  right dialog. Silent on errors during the automatic (enabled) path. */
export async function runStartupFlow(): Promise<StartupAction> {
  const first = decideStartupAction(settings.updateConsent, null, settings.skippedVersion);
  if (first !== 'check') return first;
  try {
    updateState.status = 'checking';
    const update = filterByChannel(await backend.check(), settings.releaseChannel);
    if (!update) {
      updateState.status = 'idle';
      return 'idle';
    }
    const action = decideStartupAction('enabled', update.version, settings.skippedVersion);
    if (action === 'show-update') {
      updateState.availableVersion = update.version;
      updateState.notes = update.notes;
      updateState.status = 'available';
    } else {
      updateState.availableVersion = null;
      updateState.notes = '';
      updateState.status = 'idle';
    }
    return action;
  } catch {
    updateState.status = 'idle';
    return 'idle';
  }
}

/** Manual check (Settings button). Always checks, ignores skippedVersion.
 *  Returns true if an update is available. */
export async function checkNow(): Promise<boolean> {
  try {
    updateState.status = 'checking';
    updateState.error = '';
    const update = filterByChannel(await backend.check(), settings.releaseChannel);
    if (!update) {
      updateState.status = 'idle';
      updateState.availableVersion = null;
      updateState.notes = '';
      return false;
    }
    updateState.availableVersion = update.version;
    updateState.notes = update.notes;
    updateState.status = 'available';
    return true;
  } catch (e) {
    updateState.status = 'error';
    updateState.error = e instanceof Error ? e.message : String(e);
    return false;
  }
}

export async function downloadAndInstall(): Promise<void> {
  try {
    updateState.status = 'downloading';
    updateState.downloaded = 0;
    updateState.total = 0;
    const result = await backend.downloadAndInstall((d, t) => {
      updateState.downloaded = d;
      updateState.total = t;
    });
    updateState.status = result === 'ready' ? 'ready' : 'idle';
  } catch (e) {
    updateState.status = 'error';
    updateState.error = e instanceof Error ? e.message : String(e);
  }
}

export function enableUpdates(): void { setUpdateConsent('enabled'); }
export function declineUpdates(): void { setUpdateConsent('declined'); }

export function skipCurrent(): void {
  if (updateState.availableVersion) setSkippedVersion(updateState.availableVersion);
  updateState.status = 'idle';
}

export async function restart(): Promise<void> { await backend.restart(); }
