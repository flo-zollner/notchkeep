/**
 * Material Design 3 Dynamic Color Engine.
 *
 * Implements a pure-TypeScript OKLCH-based color engine following the MD3
 * tonal-palette / dynamic-color specification. No external color library is
 * used — all math follows Björn Ottosson's OKLCH derivation.
 *
 * Public API
 * ----------
 * - `srgbToOklch`    — sRGB (0-255) → OKLCH
 * - `buildScheme`    — build a full MD3 token map from a source hue/chroma
 * - `applyMaterialYou` — read the system accent and write the scheme to <html>
 *                        (Android only)
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** OKLCH color. L ∈ [0,1], C ≥ 0, H ∈ [0,360) degrees. */
export interface Oklch {
  L: number;
  C: number;
  H: number;
}

/** Internal tonal-palette descriptor. */
interface Palette {
  h: number;
  c: number;
}

// ---------------------------------------------------------------------------
// sRGB → OKLCH conversion (Ottosson, 2020)
// ---------------------------------------------------------------------------

/** Linearise a single sRGB channel value (0-255). */
function linearise(u255: number): number {
  const u = u255 / 255;
  return u <= 0.04045 ? u / 12.92 : Math.pow((u + 0.055) / 1.055, 2.4);
}

/**
 * Convert an sRGB triplet (each component 0-255) to OKLCH.
 *
 * Math follows https://bottosson.github.io/posts/oklab/ combined with the
 * standard Lab→LCH polar conversion.
 */
export function srgbToOklch(r: number, g: number, b: number): Oklch {
  const rl = linearise(r);
  const gl = linearise(g);
  const bl = linearise(b);

  // Linear sRGB → LMS (Ottosson M1 matrix)
  const lms_l = 0.4122214708 * rl + 0.5363325363 * gl + 0.0514459929 * bl;
  const lms_m = 0.2119034982 * rl + 0.6806995451 * gl + 0.1073969566 * bl;
  const lms_s = 0.0883024619 * rl + 0.2817188376 * gl + 0.6299787005 * bl;

  // Cube-root compression
  const l_ = Math.cbrt(lms_l);
  const m_ = Math.cbrt(lms_m);
  const s_ = Math.cbrt(lms_s);

  // LMS' → OKLab (Ottosson M2 matrix)
  const L = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
  const a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
  const b2 = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

  // OKLab → OKLCH
  const C = Math.hypot(a, b2);
  let H = (Math.atan2(b2, a) * 180) / Math.PI;
  if (H < 0) H += 360;

  return { L, C, H };
}

// ---------------------------------------------------------------------------
// Tonal-palette tone rendering
// ---------------------------------------------------------------------------

/**
 * Render a single tone (0-100) of the given palette as an oklch() CSS string.
 *
 * The chroma is tapered toward 0 at the poles (pure white / pure black) to
 * stay within the sRGB gamut and match MD3 expectations.
 */
function tone(pal: Palette, t: number): string {
  const L = t / 100;
  const taper = 1 - Math.pow(Math.abs(2 * L - 1), 2.2);
  const C = pal.c * (0.35 + 0.65 * taper);
  return `oklch(${L.toFixed(4)} ${C.toFixed(4)} ${pal.h.toFixed(2)})`;
}

// ---------------------------------------------------------------------------
// Scheme builder
// ---------------------------------------------------------------------------

/**
 * Build a complete MD3 token map from a source hue and chroma value.
 *
 * Returns a `Record<string, string>` where keys are CSS custom-property names
 * (e.g. `--accent`, `--md-sys-color-primary`) and values are `oklch(…)` CSS
 * color strings ready to pass to `element.style.setProperty`.
 *
 * @param sourceHue    - Hue angle of the brand/source color in degrees [0,360).
 * @param sourceChroma - Chroma of the source color. Clamped to [0.07, 0.15].
 * @param dark         - Whether to produce a dark-mode scheme.
 */
export function buildScheme(
  sourceHue: number,
  sourceChroma: number,
  dark: boolean
): Record<string, string> {
  // Clamp chroma so near-grey inputs still produce a vivid palette
  const Cp = Math.max(0.07, Math.min(0.15, sourceChroma));

  // Tonal palettes
  const primary: Palette = { h: sourceHue, c: Cp };
  const secondary: Palette = { h: sourceHue, c: Cp * 0.34 };
  const tertiary: Palette = { h: (sourceHue + 60) % 360, c: Cp * 0.70 };
  const neutral: Palette = { h: sourceHue, c: 0.008 };
  const neutralVariant: Palette = { h: sourceHue, c: 0.016 };
  const error: Palette = { h: 27, c: 0.17 };

  // Helper shortcuts
  const p = (t: number) => tone(primary, t);
  const sec = (t: number) => tone(secondary, t);
  const ter = (t: number) => tone(tertiary, t);
  const n = (t: number) => tone(neutral, t);
  const nv = (t: number) => tone(neutralVariant, t);
  const err = (t: number) => tone(error, t);

  // MD3 system color roles
  let roles: Record<string, string>;

  if (!dark) {
    // Light scheme
    roles = {
      '--md-sys-color-primary': p(40),
      '--md-sys-color-on-primary': p(100),
      '--md-sys-color-primary-container': p(90),
      '--md-sys-color-on-primary-container': p(10),
      '--md-sys-color-secondary': sec(40),
      '--md-sys-color-secondary-container': sec(90),
      '--md-sys-color-on-secondary-container': sec(10),
      '--md-sys-color-tertiary': ter(40),
      '--md-sys-color-surface': n(98),
      '--md-sys-color-on-surface': n(10),
      '--md-sys-color-on-surface-variant': nv(30),
      '--md-sys-color-surface-variant': nv(90),
      '--md-sys-color-outline': nv(50),
      '--md-sys-color-outline-variant': nv(80),
      '--md-sys-color-surface-container-lowest': n(100),
      '--md-sys-color-surface-container-low': n(96),
      '--md-sys-color-surface-container': n(94),
      '--md-sys-color-surface-container-high': n(92),
      '--md-sys-color-surface-container-highest': n(90),
      '--md-sys-color-background': n(98),
      '--md-sys-color-error': err(40),
      '--md-sys-color-error-container': err(90),
    };
  } else {
    // Dark scheme
    roles = {
      '--md-sys-color-primary': p(80),
      '--md-sys-color-on-primary': p(20),
      '--md-sys-color-primary-container': p(30),
      '--md-sys-color-on-primary-container': p(90),
      '--md-sys-color-secondary': sec(80),
      '--md-sys-color-secondary-container': sec(30),
      '--md-sys-color-on-secondary-container': sec(90),
      '--md-sys-color-tertiary': ter(80),
      '--md-sys-color-surface': n(6),
      '--md-sys-color-on-surface': n(90),
      '--md-sys-color-on-surface-variant': nv(80),
      '--md-sys-color-surface-variant': nv(30),
      '--md-sys-color-outline': nv(60),
      '--md-sys-color-outline-variant': nv(30),
      '--md-sys-color-surface-container-lowest': n(4),
      '--md-sys-color-surface-container-low': n(10),
      '--md-sys-color-surface-container': n(12),
      '--md-sys-color-surface-container-high': n(17),
      '--md-sys-color-surface-container-highest': n(22),
      '--md-sys-color-background': n(6),
      '--md-sys-color-error': err(80),
      '--md-sys-color-error-container': err(30),
    };
  }

  // Remapped base tokens — the rest of the app uses these generic names
  const accentHover = dark ? p(85) : p(30);
  const textFaint = dark ? nv(55) : nv(45);

  const base: Record<string, string> = {
    '--bg': roles['--md-sys-color-surface'],
    '--surface': roles['--md-sys-color-surface-container-low'],
    '--surface-2': roles['--md-sys-color-surface-container'],
    '--surface-hover': roles['--md-sys-color-surface-container-high'],
    '--border': roles['--md-sys-color-outline-variant'],
    '--border-strong': roles['--md-sys-color-outline'],
    '--text': roles['--md-sys-color-on-surface'],
    '--text-muted': roles['--md-sys-color-on-surface-variant'],
    '--text-faint': textFaint,
    '--accent': roles['--md-sys-color-primary'],
    '--accent-hover': accentHover,
    '--accent-soft': roles['--md-sys-color-secondary-container'],
    '--accent-fg': roles['--md-sys-color-on-primary'],
  };

  return { ...base, ...roles };
}

// ---------------------------------------------------------------------------
// applyMaterialYou — Android dynamic color integration
// ---------------------------------------------------------------------------

/** Determine whether dark mode is active. */
function isDarkMode(): boolean {
  if (typeof document === 'undefined' || typeof window === 'undefined') return false;
  const theme = document.documentElement.dataset.theme;
  if (theme === 'dark') return true;
  if (theme === 'light') return false;
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

/**
 * Read the OS/system accent color via the CSS `AccentColor` keyword, convert
 * it to OKLCH, build a full MD3 scheme, and write every token as a custom
 * property on `<html>`.
 *
 * This function is a no-op in every environment except:
 * - A real browser context (`document` must be defined), AND
 * - The page root element must carry `data-platform="android"`.
 *
 * On failure (unresolvable color, near-grey result) the function falls back
 * to the brand teal `{ L: 0.55, C: 0.13, H: 165 }`.
 */
export function applyMaterialYou(): void {
  if (typeof document === 'undefined') return;

  const root = document.documentElement;
  if (root.dataset.platform !== 'android') return;

  // --- Read system accent color ---
  const FALLBACK: Oklch = { L: 0.55, C: 0.13, H: 165 };
  let src: Oklch = FALLBACK;

  try {
    const el = document.createElement('div');
    el.style.position = 'absolute';
    el.style.visibility = 'hidden';
    el.style.pointerEvents = 'none';
    el.style.color = 'AccentColor';
    document.body.appendChild(el);
    const colorStr = getComputedStyle(el).color;
    document.body.removeChild(el);

    // Parse rgb(r, g, b) or rgba(r, g, b, a)
    const m = colorStr.match(/rgba?\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)/);
    if (m) {
      const converted = srgbToOklch(parseFloat(m[1]), parseFloat(m[2]), parseFloat(m[3]));
      // Only use if the color has meaningful chroma (not a grey)
      if (converted.C > 0.02) {
        src = converted;
      }
    }
  } catch {
    // Silently fall back to brand default
  }

  const scheme = buildScheme(src.H, src.C, isDarkMode());
  for (const [key, value] of Object.entries(scheme)) {
    root.style.setProperty(key, value);
  }
}
