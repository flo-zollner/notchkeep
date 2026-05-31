/**
 * Tauri IPC mock helpers for Playwright E2E tests.
 *
 * The app is built on Tauri 2 and all Tauri/plugin calls go through
 * `window.__TAURI_INTERNALS__.invoke(cmd, args)`.  In a plain browser
 * (Playwright / pnpm dev) that object does not exist, so every import of
 * `@tauri-apps/api/core` or a Tauri plugin throws or hangs.
 *
 * `mockTauri()` uses `page.addInitScript` (runs before ANY page JS) to inject
 * a minimal implementation that:
 *   - provides `transformCallback` / `unregisterCallback` / `runCallback`
 *     (needed by the Channel class that the updater plugin uses internally)
 *   - intercepts `plugin:updater|check` – returns update metadata or null
 *   - intercepts `plugin:app|version`   – returns '0.2.2'
 *   - intercepts `plugin:event|listen`  – registers a no-op listener and
 *     returns a numeric id so that `listen()` from @tauri-apps/api/event
 *     doesn't reject (layout uses it for data_path_error / price_refresh_status)
 *   - intercepts `plugin:event|unlisten` – no-op
 *   - intercepts `plugin:resources|close` – no-op (called when Update is GC'd)
 *   - returns `null` for everything else so app code that awaits an invoke
 *     resolves instead of hanging
 *
 * The `plugin:updater|check` response shape must satisfy the `Update`
 * constructor from `@tauri-apps/plugin-updater`:
 *   class Update extends Resource {
 *     constructor(metadata) {
 *       super(metadata.rid);   // <-- Resource needs a numeric rid
 *       this.version = metadata.version;
 *       this.currentVersion = metadata.currentVersion;
 *       this.body = metadata.body;
 *     }
 *   }
 */

import type { Page } from '@playwright/test';

export interface MockTauriOpts {
  /** When set, `plugin:updater|check` returns an available update with this version. */
  updateVersion?: string;
}

export interface SeedSettingsOpts {
  onboardingCompleted?: boolean;
  updateConsent?: 'unset' | 'enabled' | 'declined';
  skippedVersion?: string | null;
  theme?: 'auto' | 'light' | 'dark';
  lang?: 'de' | 'en';
  hide?: boolean;
  showCents?: boolean;
  tourCompleted?: boolean;
}

/**
 * Seeds `localStorage['saldo.settings']` before the page loads.
 * Must be called BEFORE `page.goto()`.
 */
export async function seedSettings(page: Page, settings: SeedSettingsOpts): Promise<void> {
  const defaults = {
    theme: 'auto',
    lang: 'de',
    hide: false,
    showCents: false,
    onboardingCompleted: false,
    tourCompleted: false,
    updateConsent: 'unset',
    skippedVersion: null,
  };
  const merged = { ...defaults, ...settings };
  await page.addInitScript((s) => {
    localStorage.setItem('saldo.settings', JSON.stringify(s));
  }, merged);
}

/**
 * Installs a mock `window.__TAURI_INTERNALS__` before the page JS runs.
 * Must be called BEFORE `page.goto()`.
 */
export async function mockTauri(page: Page, opts: MockTauriOpts = {}): Promise<void> {
  await page.addInitScript((options) => {
    // ------------------------------------------------------------------ //
    // 1. Callback registry – mirrors the real Tauri internals so that
    //    Channel (used by the updater download flow) works.
    // ------------------------------------------------------------------ //
    const callbacks = new Map<number, (data: unknown) => void>();
    let _nextId = 1;

    function transformCallback(callback: (data: unknown) => void, once = false): number {
      const id = _nextId++;
      callbacks.set(id, (data: unknown) => {
        if (once) callbacks.delete(id);
        callback(data);
      });
      return id;
    }

    function unregisterCallback(id: number) {
      callbacks.delete(id);
    }

    function runCallback(id: number, data: unknown) {
      const fn = callbacks.get(id);
      if (fn) fn(data);
    }

    // ------------------------------------------------------------------ //
    // 2. Event-listener registry – minimal, just enough so that `listen()`
    //    from @tauri-apps/api/event resolves to an unlisten function.
    // ------------------------------------------------------------------ //
    const eventListeners = new Map<string, number[]>();
    let _nextListenerId = 1000;

    function handleEventListen(args: { event: string; handler: number }) {
      const id = _nextListenerId++;
      if (!eventListeners.has(args.event)) eventListeners.set(args.event, []);
      eventListeners.get(args.event)!.push(id);
      return id;
    }

    // ------------------------------------------------------------------ //
    // 3. Main invoke handler
    // ------------------------------------------------------------------ //
    async function invoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
      // ---------- updater ----------
      if (cmd === 'plugin:updater|check') {
        if (options.updateVersion) {
          // Return the metadata shape that `new Update(metadata)` expects.
          // `rid` is required by Resource base class.
          return {
            rid: 1,
            available: true,
            currentVersion: '0.2.2',
            version: options.updateVersion,
            date: null,
            body: 'Release notes',
            rawJson: '{}',
          };
        }
        return null; // no update available
      }

      // ---------- app version ----------
      if (cmd === 'plugin:app|version') {
        return '0.2.2';
      }

      // ---------- resources (close rid) ----------
      if (cmd === 'plugin:resources|close') {
        return null;
      }

      // ---------- event plugin ----------
      if (cmd === 'plugin:event|listen') {
        return handleEventListen(args as { event: string; handler: number });
      }
      if (cmd === 'plugin:event|unlisten') {
        return null;
      }
      if (cmd === 'plugin:event|emit') {
        return null;
      }

      // ---------- updater download (not exercised in render tests) ----------
      if (cmd === 'plugin:updater|download_and_install' ||
          cmd === 'plugin:updater|download' ||
          cmd === 'plugin:updater|install') {
        return null;
      }

      // ---------- process plugin (relaunch) ----------
      if (cmd === 'plugin:process|restart' || cmd === 'plugin:process|exit') {
        return null;
      }

      // ---------- dialog plugin ----------
      if (cmd.startsWith('plugin:dialog|')) {
        return null;
      }

      // ---------- opener plugin ----------
      if (cmd.startsWith('plugin:opener|')) {
        return null;
      }

      // ---------- app DB commands (list_accounts, list_transactions, etc.) ----------
      // Return benign empty values so the app boots without errors.
      // These are the commands defined in src-tauri/src/commands/*.rs
      if (
        cmd === 'list_accounts' ||
        cmd === 'list_institutions'
      ) {
        return [];
      }
      if (
        cmd === 'get_settings' ||
        cmd === 'get_net_worth' ||
        cmd === 'get_dashboard' ||
        cmd === 'check_sync_conflicts'
      ) {
        return null;
      }

      // Catch-all: return null for any other command so awaited calls resolve.
      return null;
    }

    // ------------------------------------------------------------------ //
    // 4. Install on window
    // ------------------------------------------------------------------ //
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
      invoke,
      transformCallback,
      unregisterCallback,
      runCallback,
      callbacks,
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { windowLabel: 'main', label: 'main' },
      },
    };

    // @tauri-apps/plugin-os reads these synchronously from a global (NOT via
    // invoke). The updater module calls platform() at import time to pick the
    // desktop vs android backend — without this, platform() throws and the
    // whole updater (and its dialogs) never initialises. 'linux' → desktop
    // backend, which routes through the plugin:updater mock above.
    (window as unknown as Record<string, unknown>).__TAURI_OS_PLUGIN_INTERNALS__ = {
      platform: 'linux',
      arch: 'x86_64',
      family: 'unix',
      os_type: 'linux',
      version: '0.0.0',
      eol: '\n',
      exe_extension: '',
    };
  }, opts);
}
