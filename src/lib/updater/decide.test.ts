import { describe, it, expect } from 'vitest';
import { decideStartupAction } from './decide';

describe('decideStartupAction', () => {
  it('returns show-activation when consent is unset', () => {
    expect(decideStartupAction('unset', null, null)).toBe('show-activation');
  });
  it('returns idle when consent is declined', () => {
    expect(decideStartupAction('declined', '0.2.3', null)).toBe('idle');
  });
  it('returns check when enabled and no version known yet', () => {
    expect(decideStartupAction('enabled', null, null)).toBe('check');
  });
  it('returns show-update when enabled and an update is available', () => {
    expect(decideStartupAction('enabled', '0.2.3', null)).toBe('show-update');
  });
  it('returns idle when the available version is skipped', () => {
    expect(decideStartupAction('enabled', '0.2.3', '0.2.3')).toBe('idle');
  });
  it('returns show-update when a newer version than the skipped one appears', () => {
    expect(decideStartupAction('enabled', '0.2.4', '0.2.3')).toBe('show-update');
  });
});
