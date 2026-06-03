import { describe, it, expect, vi, beforeEach } from 'vitest';

const { checkMock } = vi.hoisted(() => ({ checkMock: vi.fn() }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: checkMock }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));
vi.mock('@tauri-apps/plugin-os', () => ({ platform: () => 'linux' }));

import { updateState, runStartupFlow, checkNow, skipCurrent } from './updater.svelte';
import { settings, _reloadForTests } from '../settings.svelte';

beforeEach(() => {
  localStorage.clear();
  _reloadForTests();
  checkMock.mockReset();
  updateState.status = 'idle';
  updateState.availableVersion = null;
});

describe('runStartupFlow', () => {
  it('does not call check() when consent is unset (shows activation)', async () => {
    const action = await runStartupFlow();
    expect(action).toBe('show-activation');
    expect(checkMock).not.toHaveBeenCalled();
  });

  it('does not call check() when consent is declined', async () => {
    settings.updateConsent = 'declined';
    const action = await runStartupFlow();
    expect(action).toBe('idle');
    expect(checkMock).not.toHaveBeenCalled();
  });

  it('shows update when enabled and an unskipped update is available', async () => {
    settings.updateConsent = 'enabled';
    checkMock.mockResolvedValue({ version: '0.2.3', currentVersion: '0.2.2' });
    const action = await runStartupFlow();
    expect(action).toBe('show-update');
    expect(updateState.availableVersion).toBe('0.2.3');
  });

  it('stays idle when the available update is the skipped version', async () => {
    settings.updateConsent = 'enabled';
    settings.skippedVersion = '0.2.3';
    checkMock.mockResolvedValue({ version: '0.2.3', currentVersion: '0.2.2' });
    const action = await runStartupFlow();
    expect(action).toBe('idle');
  });

  it('stays idle and silent when check() throws (offline)', async () => {
    settings.updateConsent = 'enabled';
    checkMock.mockRejectedValue(new Error('network'));
    const action = await runStartupFlow();
    expect(action).toBe('idle');
    expect(updateState.status).toBe('idle');
  });
});

describe('checkNow and skipCurrent', () => {
  it('checkNow returns true even when the available version is skipped', async () => {
    settings.updateConsent = 'enabled';
    settings.skippedVersion = '0.2.3';
    checkMock.mockResolvedValue({ version: '0.2.3', currentVersion: '0.2.2' });
    const has = await checkNow();
    expect(has).toBe(true);
    expect(updateState.status).toBe('available');
  });

  it('skipCurrent persists the available version; a newer version re-appears', async () => {
    settings.updateConsent = 'enabled';
    checkMock.mockResolvedValue({ version: '0.2.3', currentVersion: '0.2.2' });
    await runStartupFlow();          // availableVersion = 0.2.3
    skipCurrent();
    expect(settings.skippedVersion).toBe('0.2.3');
    // same version is now skipped → idle
    checkMock.mockResolvedValue({ version: '0.2.3', currentVersion: '0.2.2' });
    expect(await runStartupFlow()).toBe('idle');
    // newer version → show-update again
    checkMock.mockResolvedValue({ version: '0.2.4', currentVersion: '0.2.2' });
    expect(await runStartupFlow()).toBe('show-update');
  });
});

describe('release channel filtering', () => {
  it('stable channel ignores a prerelease (startup → idle)', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'stable';
    checkMock.mockResolvedValue({ version: '0.3.0-rc.2', currentVersion: '0.2.4' });
    expect(await runStartupFlow()).toBe('idle');
    expect(updateState.availableVersion).toBeNull();
  });

  it('beta channel offers a prerelease (startup → show-update)', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'beta';
    checkMock.mockResolvedValue({ version: '0.3.0-rc.2', currentVersion: '0.2.4' });
    expect(await runStartupFlow()).toBe('show-update');
    expect(updateState.availableVersion).toBe('0.3.0-rc.2');
  });

  it('stable channel still offers a stable update', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'stable';
    checkMock.mockResolvedValue({ version: '0.3.0', currentVersion: '0.2.4' });
    expect(await runStartupFlow()).toBe('show-update');
  });

  it('checkNow on stable returns false for a prerelease', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'stable';
    checkMock.mockResolvedValue({ version: '0.3.0-rc.2', currentVersion: '0.2.4' });
    expect(await checkNow()).toBe(false);
    expect(updateState.status).toBe('idle');
  });

  it('checkNow on beta returns true for a prerelease', async () => {
    settings.updateConsent = 'enabled';
    settings.releaseChannel = 'beta';
    checkMock.mockResolvedValue({ version: '0.3.0-rc.2', currentVersion: '0.2.4' });
    expect(await checkNow()).toBe(true);
    expect(updateState.status).toBe('available');
  });
});
