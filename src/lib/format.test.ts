import { describe, it, expect, beforeEach } from 'vitest';
import { fmtEur, parseEur, parseEurCents, decimalSep } from './format';
import { setLang, _reloadForTests } from './settings.svelte';

beforeEach(() => {
  localStorage.clear();
  _reloadForTests();
  setLang('de');
});

describe('fmtEur (DE locale)', () => {
  it('formats positive cents with 2 decimals by default', () => {
    expect(fmtEur(12345)).toBe('123,45 €');
  });

  it('formats negative amounts with minus sign (U+2212)', () => {
    // The minus sign is U+2212 (−), not an ASCII hyphen (-).
    expect(fmtEur(-12345)).toBe('−123,45 €');
  });

  it('respects decimals=0 (no fractional part)', () => {
    expect(fmtEur(12345, { decimals: 0 })).toBe('123 €');
  });

  it('groups thousands with German separator', () => {
    expect(fmtEur(1234567)).toBe('12.345,67 €');
  });

  it('masks amount when hide=true', () => {
    expect(fmtEur(12345, { hide: true })).toBe('•••• €');
  });

  it('adds + sign for positive when signed=true', () => {
    expect(fmtEur(12345, { signed: true })).toBe('+123,45 €');
  });

  it('omits + sign for zero even when signed=true', () => {
    expect(fmtEur(0, { signed: true })).toBe('0,00 €');
  });

  it('formats zero without sign', () => {
    expect(fmtEur(0)).toBe('0,00 €');
  });
});

describe('fmtEur (EN locale)', () => {
  beforeEach(() => {
    setLang('en');
  });

  it('uses comma as thousands separator and dot as decimal', () => {
    expect(fmtEur(1234567)).toBe('12,345.67 €');
  });

  it('formats negative with U+2212 minus', () => {
    expect(fmtEur(-12345)).toBe('−123.45 €');
  });

  it('formats large number with correct en-US grouping', () => {
    expect(fmtEur(123456789)).toBe('1,234,567.89 €');
  });
});

describe('parseEur', () => {
  it('parses "123,45" (DE comma-decimal) as 123.45', () => {
    expect(parseEur('123,45')).toBeCloseTo(123.45);
  });

  it('parses "123.45" (EN dot-decimal) as 123.45', () => {
    expect(parseEur('123.45')).toBeCloseTo(123.45);
  });

  it('parses "1.234,56" (DE with thousands separator) as 1234.56', () => {
    expect(parseEur('1.234,56')).toBeCloseTo(1234.56);
  });

  it('parses "1,234.56" (EN with thousands separator) as 1234.56', () => {
    expect(parseEur('1,234.56')).toBeCloseTo(1234.56);
  });

  it('parses integer string "1234" as 1234', () => {
    expect(parseEur('1234')).toBe(1234);
  });

  it('parses negative "-12,5" as -12.5', () => {
    expect(parseEur('-12,5')).toBeCloseTo(-12.5);
  });

  it('returns NaN for empty string', () => {
    expect(parseEur('')).toBeNaN();
  });

  it('returns NaN for whitespace-only string', () => {
    expect(parseEur('   ')).toBeNaN();
  });

  it('returns NaN for non-numeric input', () => {
    expect(parseEur('foo')).toBeNaN();
  });

  it('trims surrounding whitespace', () => {
    expect(parseEur('  42,5  ')).toBeCloseTo(42.5);
  });
});

describe('parseEurCents', () => {
  it('converts euro string to integer cents', () => {
    // "12,34" → comma is decimal separator in DE → 12.34 → 1234 cents
    expect(parseEurCents('12,34')).toBe(1234);
  });

  it('handles DE thousands + decimal: "1.234,56" → 123456 cents', () => {
    expect(parseEurCents('1.234,56')).toBe(123456);
  });

  it('returns 0 for invalid input', () => {
    expect(parseEurCents('foo')).toBe(0);
  });

  it('rounds fractional cents (0.005 euro = 0.5 cents → rounds to 1 cent)', () => {
    // 0.005 euros = 0.5 cents, Math.round(0.5) = 1
    expect(parseEurCents('0,005')).toBe(1);
  });

  it('returns 0 for empty string', () => {
    expect(parseEurCents('')).toBe(0);
  });

  // Note: "1,234" in DE locale — comma is last separator → decimal → 1.234 euros → 123 cents
  it('"1,234" in DE (comma=decimal) parses as 1.234 euros → 123 cents', () => {
    expect(parseEurCents('1,234')).toBe(123);
  });
});

describe('decimalSep', () => {
  it('returns "," in DE locale', () => {
    expect(decimalSep()).toBe(',');
  });

  it('returns "." in EN locale', () => {
    setLang('en');
    expect(decimalSep()).toBe('.');
  });
});
