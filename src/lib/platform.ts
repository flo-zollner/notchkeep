/**
 * Platform detection for platform-scoped styling.
 *
 * Writes the resolved platform to `documentElement.dataset.platform` so CSS can
 * scope rules via `html[data-platform="android"]`. The Material Design 3 layer
 * is applied only on Android; every other platform keeps the default design.
 *
 * Resolution order:
 *  1. Dev/test override — `?platform=android` query param (persisted to
 *     localStorage), so the mock browser and Playwright can exercise the
 *     Android layer without a real device.
 *  2. Tauri `plugin-os` `platform()` — returns 'android' | 'linux' | … on a
 *     real Tauri runtime, and `undefined` in a plain browser (it does not throw,
 *     which is why the mock build already calls it at module load).
 *  3. Fallback `'web'`.
 */
import { platform } from '@tauri-apps/plugin-os';

const OVERRIDE_KEY = 'platformOverride';

export function resolvePlatform(): string {
  try {
    const q = new URLSearchParams(window.location.search).get('platform');
    if (q) window.localStorage.setItem(OVERRIDE_KEY, q);
    const stored = window.localStorage.getItem(OVERRIDE_KEY);
    if (stored) return stored;
  } catch {
    // localStorage / location unavailable — fall through to runtime detection
  }
  try {
    const p = platform();
    if (p) return p;
  } catch {
    // plugin-os not present (plain browser) — fall through
  }
  return 'web';
}

/** Sets `documentElement.dataset.platform`. No-op outside the browser. */
export function applyPlatform(): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.platform = resolvePlatform();
}
