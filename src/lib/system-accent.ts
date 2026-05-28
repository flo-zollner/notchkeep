/**
 * System-Accent integration.
 *
 * Reads the user's system accent color via the CSS `AccentColor` keyword
 * (resolved through a hidden DOM element) and writes it to the
 * `--accent`, `--accent-hover`, `--accent-soft`, `--accent-fg`
 * custom properties on `<html>`. Falls back silently to the design-system
 * default if the system does not expose an accent (older WebKitGTK,
 * GNOME < 47, Android < 12).
 */

export interface HSL {
  h: number;
  s: number;
  l: number;
}

export interface AccentTokens {
  accent: string;
  accentHover: string;
  accentSoft: string;
  accentFg: string;
}

const RGB_RE = /^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/;

export function parseRgbToHsl(input: string): HSL | null {
  const m = input.match(RGB_RE);
  if (!m) return null;
  const r = parseInt(m[1], 10) / 255;
  const g = parseInt(m[2], 10) / 255;
  const b = parseInt(m[3], 10) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0);
        break;
      case g:
        h = (b - r) / d + 2;
        break;
      case b:
        h = (r - g) / d + 4;
        break;
    }
    h *= 60;
  }
  return { h, s: s * 100, l: l * 100 };
}

function hslString(hsl: HSL): string {
  return `hsl(${hsl.h.toFixed(1)} ${hsl.s.toFixed(1)}% ${hsl.l.toFixed(1)}%)`;
}

export function deriveAccentTokens(base: HSL): AccentTokens {
  const hoverL = base.l > 50 ? Math.max(0, base.l - 6) : Math.min(100, base.l + 6);
  const isLight = base.l > 60;
  return {
    accent: hslString(base),
    accentHover: hslString({ ...base, l: hoverL }),
    accentSoft: hslString({ h: base.h, s: Math.min(base.s, 40), l: isLight ? 30 : 93 }),
    accentFg: isLight ? 'oklch(0.18 0.01 80)' : 'oklch(0.98 0.005 90)',
  };
}

export function readSystemAccent(): HSL | null {
  if (typeof document === 'undefined') return null;
  const el = document.createElement('div');
  el.style.position = 'absolute';
  el.style.visibility = 'hidden';
  el.style.pointerEvents = 'none';
  el.style.color = 'AccentColor';
  document.body.appendChild(el);
  const color = getComputedStyle(el).color;
  document.body.removeChild(el);
  return parseRgbToHsl(color);
}

export function applySystemAccent(): void {
  const hsl = readSystemAccent();
  if (!hsl) return;
  const tokens = deriveAccentTokens(hsl);
  const root = document.documentElement.style;
  root.setProperty('--accent', tokens.accent);
  root.setProperty('--accent-hover', tokens.accentHover);
  root.setProperty('--accent-soft', tokens.accentSoft);
  root.setProperty('--accent-fg', tokens.accentFg);
}
