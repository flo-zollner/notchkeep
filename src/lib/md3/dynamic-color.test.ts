import { describe, it, expect, beforeEach } from 'vitest';
import { srgbToOklch, buildScheme, applyMaterialYou } from './dynamic-color';

/** Parse an `oklch(L C H)` string into numbers (ignores optional alpha). */
function parse(v: string): { L: number; C: number; H: number } {
  const m = v.match(/oklch\(\s*([\d.]+)\s+([\d.]+)\s+(-?[\d.]+)/);
  if (!m) throw new Error(`not an oklch() value: ${v}`);
  return { L: +m[1], C: +m[2], H: +m[3] };
}

/** Every CSS custom property the scheme must define (contract for consumers). */
const REQUIRED_KEYS = [
  // Remapped base tokens (so the whole app recolors on Android)
  '--bg',
  '--surface',
  '--surface-2',
  '--surface-hover',
  '--border',
  '--border-strong',
  '--text',
  '--text-muted',
  '--text-faint',
  '--accent',
  '--accent-hover',
  '--accent-soft',
  '--accent-fg',
  // MD3 system color roles used directly by components
  '--md-sys-color-primary',
  '--md-sys-color-on-primary',
  '--md-sys-color-primary-container',
  '--md-sys-color-on-primary-container',
  '--md-sys-color-secondary-container',
  '--md-sys-color-on-secondary-container',
  '--md-sys-color-surface-container-low',
  '--md-sys-color-surface-container',
  '--md-sys-color-surface-container-high',
];

describe('srgbToOklch', () => {
  it('maps white to L≈1, C≈0', () => {
    const c = srgbToOklch(255, 255, 255);
    expect(c.L).toBeGreaterThan(0.98);
    expect(c.C).toBeLessThan(0.002);
  });

  it('maps black to L≈0', () => {
    const c = srgbToOklch(0, 0, 0);
    expect(c.L).toBeLessThan(0.02);
  });

  it('maps sRGB red to its known OKLCH hue/lightness', () => {
    const c = srgbToOklch(255, 0, 0);
    expect(c.L).toBeCloseTo(0.628, 1);
    expect(c.H).toBeGreaterThan(25);
    expect(c.H).toBeLessThan(33);
    expect(c.C).toBeGreaterThan(0.2);
  });
});

describe('buildScheme', () => {
  it('defines every required color role', () => {
    const scheme = buildScheme(165, 0.13, false);
    for (const key of REQUIRED_KEYS) {
      expect(scheme[key], `missing ${key}`).toBeDefined();
      expect(scheme[key]).toMatch(/^oklch\(/);
    }
  });

  it('light scheme: surface is light, on-surface text is dark', () => {
    const s = buildScheme(165, 0.13, false);
    expect(parse(s['--surface']).L).toBeGreaterThan(0.9);
    expect(parse(s['--text']).L).toBeLessThan(0.3);
  });

  it('dark scheme: surface is dark, on-surface text is light', () => {
    const s = buildScheme(165, 0.13, true);
    expect(parse(s['--surface']).L).toBeLessThan(0.2);
    expect(parse(s['--text']).L).toBeGreaterThan(0.8);
  });

  it('preserves the source hue on primary/accent', () => {
    const s = buildScheme(165, 0.13, false);
    expect(parse(s['--accent']).H).toBeCloseTo(165, 0);
    expect(parse(s['--md-sys-color-primary']).H).toBeCloseTo(165, 0);
  });

  it('primary tone differs between light (darker) and dark (lighter)', () => {
    const light = parse(buildScheme(165, 0.13, false)['--accent']).L;
    const dark = parse(buildScheme(165, 0.13, true)['--accent']).L;
    expect(dark).toBeGreaterThan(light);
  });

  it('clamps a near-grey source up to a vivid minimum chroma', () => {
    const s = buildScheme(165, 0.001, false);
    expect(parse(s['--accent']).C).toBeGreaterThan(0.05);
  });

  it('container roles are tonally distinct from their on-* pair', () => {
    const s = buildScheme(165, 0.13, false);
    const container = parse(s['--md-sys-color-primary-container']).L;
    const onContainer = parse(s['--md-sys-color-on-primary-container']).L;
    expect(Math.abs(container - onContainer)).toBeGreaterThan(0.4);
  });

  it('surface container tiers ascend in tonal elevation (light)', () => {
    const s = buildScheme(165, 0.13, false);
    const low = parse(s['--md-sys-color-surface-container-low']).L;
    const mid = parse(s['--md-sys-color-surface-container']).L;
    const high = parse(s['--md-sys-color-surface-container-high']).L;
    // Light mode: higher elevation = slightly darker tonal surface
    expect(low).toBeGreaterThanOrEqual(mid);
    expect(mid).toBeGreaterThanOrEqual(high);
  });
});

describe('applyMaterialYou', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('style');
    delete document.documentElement.dataset.platform;
  });

  it('sets MD3 color custom properties on <html> when platform is android', () => {
    document.documentElement.dataset.platform = 'android';
    applyMaterialYou();
    const accent = document.documentElement.style.getPropertyValue('--accent');
    expect(accent).toMatch(/^oklch\(/);
    expect(document.documentElement.style.getPropertyValue('--md-sys-color-primary')).toMatch(/^oklch\(/);
  });

  it('does nothing when platform is not android', () => {
    document.documentElement.dataset.platform = 'web';
    applyMaterialYou();
    expect(document.documentElement.style.getPropertyValue('--accent')).toBe('');
  });
});
