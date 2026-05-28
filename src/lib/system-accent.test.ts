import { describe, it, expect, beforeEach, vi } from 'vitest';
import { parseRgbToHsl, deriveAccentTokens, applySystemAccent } from './system-accent';

describe('parseRgbToHsl', () => {
  it('parses "rgb(255, 0, 0)" as red (H≈0, S=100, L=50)', () => {
    const r = parseRgbToHsl('rgb(255, 0, 0)');
    expect(r).not.toBeNull();
    expect(r!.h).toBeCloseTo(0, 0);
    expect(r!.s).toBeCloseTo(100, 0);
    expect(r!.l).toBeCloseTo(50, 0);
  });

  it('parses "rgb(0, 128, 255)" as blue-cyan (H≈210)', () => {
    const r = parseRgbToHsl('rgb(0, 128, 255)');
    expect(r).not.toBeNull();
    expect(r!.h).toBeCloseTo(210, 0);
  });

  it('parses "rgba(0, 200, 100, 0.8)" ignoring alpha', () => {
    const r = parseRgbToHsl('rgba(0, 200, 100, 0.8)');
    expect(r).not.toBeNull();
    expect(r!.s).toBeGreaterThan(50);
  });

  it('parses float RGB values from WebKitGTK (e.g. "rgb(50.196, 100.0, 200.392)")', () => {
    const r = parseRgbToHsl('rgb(50.196, 100.0, 200.392)');
    expect(r).not.toBeNull();
    expect(r!.h).toBeGreaterThan(200);
    expect(r!.h).toBeLessThan(230);
  });

  it('returns null for unparseable strings', () => {
    expect(parseRgbToHsl('AccentColor')).toBeNull();
    expect(parseRgbToHsl('')).toBeNull();
    expect(parseRgbToHsl('not-a-color')).toBeNull();
  });
});

describe('deriveAccentTokens', () => {
  it('produces all four tokens as valid hsl strings for medium-lightness accent', () => {
    const tokens = deriveAccentTokens({ h: 210, s: 80, l: 50 });
    expect(tokens.accent).toMatch(/^hsl\(/);
    expect(tokens.accentHover).toMatch(/^hsl\(/);
    expect(tokens.accentSoft).toMatch(/^hsl\(/);
    expect(tokens.accentFg).toMatch(/^(hsl|oklch)\(/);
  });

  it('picks dark accentFg when base is light (L > 60)', () => {
    const tokens = deriveAccentTokens({ h: 60, s: 80, l: 80 });
    expect(tokens.accentFg).toMatch(/oklch\(0\.\d/);
    const match = tokens.accentFg.match(/oklch\((0\.\d+)/);
    expect(match).not.toBeNull();
    expect(parseFloat(match![1])).toBeLessThan(0.5);
  });

  it('picks light accentFg when base is dark (L ≤ 60)', () => {
    const tokens = deriveAccentTokens({ h: 240, s: 80, l: 30 });
    const match = tokens.accentFg.match(/oklch\((0\.\d+)/);
    expect(match).not.toBeNull();
    expect(parseFloat(match![1])).toBeGreaterThan(0.8);
  });

  it('accentHover differs from accent (lightness shift)', () => {
    const tokens = deriveAccentTokens({ h: 210, s: 80, l: 50 });
    expect(tokens.accentHover).not.toBe(tokens.accent);
  });
});

describe('applySystemAccent', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('style');
  });

  it('sets --accent on documentElement when AccentColor resolves', () => {
    const realGCS = window.getComputedStyle.bind(window);
    vi.spyOn(window, 'getComputedStyle').mockImplementation((el: Element) => {
      const cs = realGCS(el);
      return new Proxy(cs, {
        get(target, prop) {
          if (prop === 'color') return 'rgb(50, 100, 200)';
          return Reflect.get(target, prop);
        },
      }) as CSSStyleDeclaration;
    });

    applySystemAccent();
    const accent = document.documentElement.style.getPropertyValue('--accent');
    expect(accent).toMatch(/^hsl\(/);
    vi.restoreAllMocks();
  });

  it('does not override --accent when AccentColor is unparseable', () => {
    const realGCS = window.getComputedStyle.bind(window);
    vi.spyOn(window, 'getComputedStyle').mockImplementation((el: Element) => {
      const cs = realGCS(el);
      return new Proxy(cs, {
        get(target, prop) {
          if (prop === 'color') return 'AccentColor';
          return Reflect.get(target, prop);
        },
      }) as CSSStyleDeclaration;
    });

    document.documentElement.style.setProperty('--accent', 'oklch(0.55 0.13 165)');
    applySystemAccent();
    const accent = document.documentElement.style.getPropertyValue('--accent');
    expect(accent).toBe('oklch(0.55 0.13 165)');
    vi.restoreAllMocks();
  });
});
