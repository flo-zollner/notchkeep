import { describe, it, expect, beforeEach } from 'vitest';
import { settings, setTheme, _reloadForTests } from './settings.svelte';

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
