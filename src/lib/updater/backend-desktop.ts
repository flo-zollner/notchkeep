import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

let pending: Update | null = null;

export const desktopBackend = {
  supportsRestart: true,
  async check(): Promise<{ version: string; notes: string } | null> {
    const u = await check();
    pending = u;
    return u ? { version: u.version, notes: u.body ?? '' } : null;
  },
  async downloadAndInstall(onProgress: (d: number, t: number) => void): Promise<'ready' | 'installer-launched'> {
    if (!pending) return 'ready';
    let downloaded = 0, total = 0;
    await pending.downloadAndInstall((e) => {
      if (e.event === 'Started') total = e.data.contentLength ?? 0;
      else if (e.event === 'Progress') { downloaded += e.data.chunkLength; onProgress(downloaded, total); }
    });
    return 'ready';
  },
  async restart(): Promise<void> { await relaunch(); },
};
