import { invoke, Channel } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { isNewer } from './semver';

const ENDPOINT = 'https://github.com/flo-zollner/notchkeep/releases/download/updater-latest/android-latest.json';
type Manifest = { version: string; notes: string; url: string; sha256: string; signature: string; versionCode: number };
let pending: Manifest | null = null;

export const androidBackend = {
  supportsRestart: false,
  async check(): Promise<{ version: string; notes: string } | null> {
    const m = await invoke<Manifest>('plugin:apk-updater|check', { endpoint: ENDPOINT });
    const current = await getVersion();
    if (!isNewer(m.version, current)) { pending = null; return null; }
    pending = m;
    return { version: m.version, notes: m.notes ?? '' };
  },
  async downloadAndInstall(onProgress: (d: number, t: number) => void): Promise<'ready' | 'installer-launched'> {
    if (!pending) throw new Error('no pending update — call check() first');
    const ch = new Channel<{ downloaded: number; total: number }>();
    ch.onmessage = (p) => onProgress(p.downloaded, p.total);
    await invoke('plugin:apk-updater|download_and_install', { manifest: pending, onEvent: ch });
    return 'installer-launched';
  },
  async restart(): Promise<void> { /* Android: system installer handles relaunch */ },
};
