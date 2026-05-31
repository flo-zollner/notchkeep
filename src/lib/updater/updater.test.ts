import { describe, it, expect, vi, beforeEach } from 'vitest';

const { checkMock } = vi.hoisted(() => ({ checkMock: vi.fn() }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: checkMock }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: vi.fn() }));

import { updateState, runStartupFlow } from './updater.svelte';
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
