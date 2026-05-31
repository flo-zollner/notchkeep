import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { settings, setUpdateConsent, setSkippedVersion } from '../settings.svelte';
import { decideStartupAction, type StartupAction } from './decide';

export type UpdateStatus =
  | 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error';

export const updateState = $state({
  status: 'idle' as UpdateStatus,
  availableVersion: null as string | null,
  notes: '' as string,
  downloaded: 0,
  total: 0,
  error: '' as string,
});

let pending: Update | null = null;

/** Startup flow. Returns the resolved action so the layout can render the
 *  right dialog. Silent on errors during the automatic (enabled) path. */
export async function runStartupFlow(): Promise<StartupAction> {
  const first = decideStartupAction(settings.updateConsent, null, settings.skippedVersion);
  if (first !== 'check') return first;
  try {
    updateState.status = 'checking';
    const update = await check();
    if (!update) {
      updateState.status = 'idle';
      return 'idle';
    }
    pending = update;
    updateState.availableVersion = update.version;
    updateState.notes = update.body ?? '';
    const action = decideStartupAction('enabled', update.version, settings.skippedVersion);
    updateState.status = action === 'show-update' ? 'available' : 'idle';
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
    const update = await check();
    if (!update) {
      updateState.status = 'idle';
      updateState.availableVersion = null;
      return false;
    }
    pending = update;
    updateState.availableVersion = update.version;
    updateState.notes = update.body ?? '';
    updateState.status = 'available';
    return true;
  } catch (e) {
    updateState.status = 'error';
    updateState.error = e instanceof Error ? e.message : String(e);
    return false;
  }
}

export async function downloadAndInstall(): Promise<void> {
  if (!pending) return;
  try {
    updateState.status = 'downloading';
    updateState.downloaded = 0;
    updateState.total = 0;
    await pending.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          updateState.total = event.data.contentLength ?? 0;
          break;
        case 'Progress':
          updateState.downloaded += event.data.chunkLength;
          break;
        case 'Finished':
          break;
      }
    });
    updateState.status = 'ready';
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

export async function restart(): Promise<void> { await relaunch(); }
