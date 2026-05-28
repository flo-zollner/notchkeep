import { settings } from '$lib/settings.svelte';

export function fmtEur(cents: number, opts: { hide?: boolean; signed?: boolean; decimals?: number } = {}): string {
  if (opts.hide) return '•••• €';
  const decimals = opts.decimals ?? 2;
  const value = cents / 100;
  const locale = settings.lang === 'en' ? 'en-US' : 'de-DE';
  const formatter = new Intl.NumberFormat(locale, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  });
  const formatted = formatter.format(Math.abs(value));
  const sign = value < 0 ? '−' : opts.signed && value > 0 ? '+' : '';
  return `${sign}${formatted} €`;
}

/**
 * Parses a Euro string from input fields. Tolerantly accepts both locale formats:
 * '1.234,56', '1,234.56', '1234.56', '1234,56', '-12,5', '5'.
 * Returns NaN for invalid input (callers check themselves).
 */
export function parseEur(input: string): number {
  if (input == null) return NaN;
  const trimmed = String(input).trim();
  if (trimmed === '') return NaN;
  const lastComma = trimmed.lastIndexOf(',');
  const lastDot = trimmed.lastIndexOf('.');
  let normalized: string;
  if (lastComma > lastDot) {
    // Comma is decimal separator → dots are thousands separators (remove).
    normalized = trimmed.replace(/\./g, '').replace(',', '.');
  } else if (lastDot > lastComma) {
    // Dot is decimal separator → commas are thousands separators (remove).
    normalized = trimmed.replace(/,/g, '');
  } else {
    // No separator — input is an integer.
    normalized = trimmed;
  }
  return parseFloat(normalized);
}

/** Locale-specific decimal separator (for placeholders / help texts). */
export function decimalSep(): string {
  return settings.lang === 'en' ? '.' : ',';
}

/** Variant of `parseEur` that directly returns cents as a rounded integer.
 *  Returns 0 for invalid input. */
export function parseEurCents(input: string): number {
  const n = parseEur(input);
  return Number.isFinite(n) ? Math.round(n * 100) : 0;
}

/**
 * Formats a cent amount as a plain-number string for input fields
 * (locale decimal separator, no thousands grouping, no currency symbol).
 * Example: 12345 → "123,45" (de) or "123.45" (en).
 */
export function fmtEurInput(cents: number, decimals = 2): string {
  const value = Math.abs(cents) / 100;
  const locale = settings.lang === 'en' ? 'en-US' : 'de-DE';
  return value.toLocaleString(locale, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
    useGrouping: false,
  });
}

/**
 * Formats an arbitrary float as a locale-aware plain-number string
 * (e.g. for shares/quantities with variable decimal precision).
 * `decimals` omitted → natural precision (like Number.toString()).
 */
export function fmtNumInput(value: number, decimals?: number): string {
  if (!Number.isFinite(value)) return '';
  const locale = settings.lang === 'en' ? 'en-US' : 'de-DE';
  if (decimals != null) {
    return value.toLocaleString(locale, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
      useGrouping: false,
    });
  }
  // Natural representation: use toString then swap '.' for ',' on de-locale.
  const s = value.toString();
  return settings.lang === 'en' ? s : s.replace('.', ',');
}
