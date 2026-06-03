import { describe, it, expect, vi, beforeEach } from 'vitest';

// Desktop drives the updater through the `updater_check` Rust command (invoke),
// not the JS plugin — so mock @tauri-apps/api/core.
const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
  Channel: class {
    onmessage: ((p: unknown) => void) | null = null;
  },
}));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('@tauri-apps/plugin-os', () => ({ platform: () => 'linux' }));

import { updateState, runStartupFlow, checkNow, skipCurrent } from './updater.svelte';
import { settings, _reloadForTests } from '../settings.svelte';

beforeEach(() => {
  localStorage.clear();
  _reloadForTests();
  invokeMock.mockReset();
  updateState.status = 'idle';
  updateState.availableVersion = null;
});

describe('runStartupFlow', () => {
  it('does not call check() when consent is unset (shows activation)', async () => {
    const action = await runStartupFlow();
    expect(action).toBe('show-activation');
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('does not call check() when consent is declined', async () => {
    settings.updateConsent = 'declined';
    const action = await runStartupFlow();
    expect(action).toBe('idle');
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('shows update when enabled and an unskipped update is available', async () => {
    settings.updateConsent = 'enabled';
    invokeMock.mockResolvedValue({ version: '0.2.3', notes: '' });
    const action = await runStartupFlow();
    expect(action).toBe('show-update');
    expect(updateState.availableVersion).toBe('0.2.3');
  });

  it('stays idle when the available update is the skipped version', async () => {
    settings.updateConsent = 'enabled';
    settings.skippedVersion = '0.2.3';
    invokeMock.mockResolvedValue({ version: '0.2.3', notes: '' });
    const action = await runStartupFlow();
    expect(action).toBe('idle');
  });

  it('stays idle and silent when check() throws (offline)', async () => {
    settings.updateConsent = 'enabled';
    invokeMock.mockRejectedValue(new Error('network'));
    const action = await runStartupFlow();
    expect(action).toBe('idle');
    expect(updateState.status).toBe('idle');
  });
});

describe('checkNow and skipCurrent', () => {
  it('checkNow returns true even when the available version is skipped', async () => {
    settings.updateConsent = 'enabled';
    settings.skippedVersion = '0.2.3';
    invokeMock.mockResolvedValue({ version: '0.2.3', notes: '' });
    const has = await checkNow();
    expect(has).toBe(true);
    expect(updateState.status).toBe('available');
  });

  it('skipCurrent persists the available version; a newer version re-appears', async () => {
    settings.updateConsent = 'enabled';
    invokeMock.mockResolvedValue({ version: '0.2.3', notes: '' });
    await runStartupFlow();          // availableVersion = 0.2.3
    skipCurrent();
    expect(settings.skippedVersion).toBe('0.2.3');
    // same version is now skipped → idle
    invokeMock.mockResolvedValue({ version: '0.2.3', notes: '' });
    expect(await runStartupFlow()).toBe('idle');
    // newer version → show-update again
    invokeMock.mockResolvedValue({ version: '0.2.4', notes: '' });
    expect(await runStartupFlow()).toBe('show-update');
  });
});

describe('release channel filtering (safety net on top of per-channel endpoints)', () => {
  it('stable channel ignores a prerelease (startup → idle)', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'stable';
    invokeMock.mockResolvedValue({ version: '0.3.0-rc.2', notes: '' });
    expect(await runStartupFlow()).toBe('idle');
    expect(updateState.availableVersion).toBeNull();
  });

  it('beta channel offers a prerelease (startup → show-update)', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'beta';
    invokeMock.mockResolvedValue({ version: '0.3.0-rc.2', notes: '' });
    expect(await runStartupFlow()).toBe('show-update');
    expect(updateState.availableVersion).toBe('0.3.0-rc.2');
  });

  it('stable channel still offers a stable update', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'stable';
    invokeMock.mockResolvedValue({ version: '0.3.0', notes: '' });
    expect(await runStartupFlow()).toBe('show-update');
  });

  it('checkNow on stable returns false for a prerelease', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'stable';
    invokeMock.mockResolvedValue({ version: '0.3.0-rc.2', notes: '' });
    expect(await checkNow()).toBe(false);
    expect(updateState.status).toBe('idle');
  });

  it('checkNow on beta returns true for a prerelease', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'beta';
    invokeMock.mockResolvedValue({ version: '0.3.0-rc.2', notes: '' });
    expect(await checkNow()).toBe(true);
    expect(updateState.status).toBe('available');
  });
});

// Regression guard for the channel-endpoint bug (2026-06-03): the check must hit
// the correct manifest per channel — stable → /releases/latest/, beta → rolling
// updater-latest. A wrong stable endpoint previously 404'd and hid stable updates.
describe('per-channel endpoint wiring (desktop)', () => {
  it('stable check() hits /releases/latest/', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'stable';
    invokeMock.mockResolvedValue(null);
    await checkNow();
    const [cmd, args] = invokeMock.mock.calls[0];
    expect(cmd).toBe('updater_check');
    expect((args as { endpoint: string }).endpoint).toBe(
      'https://github.com/flo-zollner/notchkeep/releases/latest/download/latest.json',
    );
  });

  it('beta check() hits the rolling updater-latest manifest', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'beta';
    invokeMock.mockResolvedValue(null);
    await checkNow();
    const [, args] = invokeMock.mock.calls[0];
    expect((args as { endpoint: string }).endpoint).toBe(
      'https://github.com/flo-zollner/notchkeep/releases/download/updater-latest/latest.json',
    );
  });
});
