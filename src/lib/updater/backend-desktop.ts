import { invoke, Channel } from '@tauri-apps/api/core';
import { relaunch } from '@tauri-apps/plugin-process';
import type { ReleaseChannel } from '../settings.svelte';
import { updaterEndpoint } from './endpoints';

type UpdateInfo = { version: string; notes: string };

// Desktop drives the official updater via custom Rust commands (see
// src-tauri/src/commands/updater.rs) so the manifest endpoint can be chosen per
// release channel at runtime — the JS plugin's check() can only use the baked-in
// config endpoint. The Rust side keeps the official download/verify/install and
// the signature pubkey from tauri.conf.json.
export const desktopBackend = {
  supportsRestart: true,
  async check(channel: ReleaseChannel): Promise<{ version: string; notes: string } | null> {
    const endpoint = updaterEndpoint(channel, 'latest.json');
    return await invoke<UpdateInfo | null>('updater_check', { endpoint });
  },
  async downloadAndInstall(onProgress: (d: number, t: number) => void): Promise<'ready' | 'installer-launched'> {
    const ch = new Channel<{ downloaded: number; total: number }>();
    ch.onmessage = (p) => onProgress(p.downloaded, p.total);
    await invoke('updater_download_install', { onEvent: ch });
    return 'ready';
  },
  async restart(): Promise<void> { await relaunch(); },
};
