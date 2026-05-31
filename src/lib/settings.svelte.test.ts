import { describe, it, expect, beforeEach } from 'vitest';
import {
  settings,
  setTheme,
  setOnboardingCompleted,
  setTourCompleted,
  setUpdateConsent,
  setSkippedVersion,
  _reloadForTests,
} from './settings.svelte';

describe('settings.theme', () => {
  beforeEach(() => {
    localStorage.clear();
    _reloadForTests();
  });

  it('defaults to "auto" when nothing is persisted', () => {
    expect(settings.theme).toBe('auto');
  });

  it('loads legacy "light" from localStorage', () => {
    localStorage.setItem(
      'saldo.settings',
      JSON.stringify({ theme: 'light', lang: 'de', hide: false, showCents: false })
    );
    _reloadForTests();
    expect(settings.theme).toBe('light');
  });

  it('loads legacy "dark" from localStorage', () => {
    localStorage.setItem(
      'saldo.settings',
      JSON.stringify({ theme: 'dark', lang: 'de', hide: false, showCents: false })
    );
    _reloadForTests();
    expect(settings.theme).toBe('dark');
  });

  it('falls back to "auto" for unknown theme values', () => {
    localStorage.setItem(
      'saldo.settings',
      JSON.stringify({ theme: 'rainbow', lang: 'de', hide: false, showCents: false })
    );
    _reloadForTests();
    expect(settings.theme).toBe('auto');
  });

  it('persists theme on setTheme("dark")', () => {
    setTheme('dark');
    const raw = localStorage.getItem('saldo.settings');
    expect(raw).not.toBeNull();
    expect(JSON.parse(raw!).theme).toBe('dark');
    expect(settings.theme).toBe('dark');
  });

  it('accepts "auto" via setTheme()', () => {
    setTheme('auto');
    expect(settings.theme).toBe('auto');
    expect(JSON.parse(localStorage.getItem('saldo.settings')!).theme).toBe('auto');
  });
});

describe('settings.onboarding flags', () => {
  beforeEach(() => {
    localStorage.clear();
    _reloadForTests();
  });

  it('defaults onboardingCompleted and tourCompleted to false', () => {
    expect(settings.onboardingCompleted).toBe(false);
    expect(settings.tourCompleted).toBe(false);
  });

  it('persists onboardingCompleted via setter', () => {
    setOnboardingCompleted(true);
    expect(settings.onboardingCompleted).toBe(true);
    expect(JSON.parse(localStorage.getItem('saldo.settings')!).onboardingCompleted).toBe(true);
  });

  it('persists tourCompleted via setter', () => {
    setTourCompleted(true);
    expect(settings.tourCompleted).toBe(true);
    expect(JSON.parse(localStorage.getItem('saldo.settings')!).tourCompleted).toBe(true);
  });

  it('loads persisted flags from localStorage', () => {
    localStorage.setItem(
      'saldo.settings',
      JSON.stringify({ theme: 'auto', lang: 'de', hide: false, showCents: false, onboardingCompleted: true, tourCompleted: true })
    );
    _reloadForTests();
    expect(settings.onboardingCompleted).toBe(true);
    expect(settings.tourCompleted).toBe(true);
  });

  it('coerces missing flags to false (legacy settings without the keys)', () => {
    localStorage.setItem(
      'saldo.settings',
      JSON.stringify({ theme: 'dark', lang: 'de', hide: false, showCents: false })
    );
    _reloadForTests();
    expect(settings.onboardingCompleted).toBe(false);
    expect(settings.tourCompleted).toBe(false);
  });
});

describe('settings.update preferences', () => {
  beforeEach(() => {
    localStorage.clear();
    _reloadForTests();
  });

  it('defaults updateConsent to "unset" and skippedVersion to null', () => {
    expect(settings.updateConsent).toBe('unset');
    expect(settings.skippedVersion).toBeNull();
  });

  it('persists updateConsent', () => {
    setUpdateConsent('enabled');
    _reloadForTests();
    expect(settings.updateConsent).toBe('enabled');
  });

  it('persists skippedVersion', () => {
    setSkippedVersion('0.2.3');
    _reloadForTests();
    expect(settings.skippedVersion).toBe('0.2.3');
  });

  it('falls back to defaults for legacy localStorage without the fields', () => {
    localStorage.setItem('saldo.settings', JSON.stringify({ theme: 'dark' }));
    _reloadForTests();
    expect(settings.updateConsent).toBe('unset');
    expect(settings.skippedVersion).toBeNull();
  });
});
