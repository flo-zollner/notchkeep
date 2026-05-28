/**
 * Deterministische Pseudo-Zufalls- und Zeitreihen-Generatoren für die Mock-Schicht.
 * Gleiche Inputs → gleiche Outputs, damit Visual-Regression-Tests stabil bleiben.
 */

/** Mulberry32: kleiner, schneller, deterministischer 32-bit PRNG. */
export function mulberry32(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * Hash einer (year, month)-Kombination zu einem stabilen Seed.
 * Garantiert: gleiche (y, m) → gleicher Seed → gleicher PRNG-Stream.
 */
export function ymSeed(year: number, month: number, salt = 0): number {
  return ((year * 100 + month) * 2654435761 + salt) >>> 0;
}

/** Hash einer Datumstring zu Seed. */
export function dateSeed(iso: string, salt = 0): number {
  let h = salt >>> 0;
  for (let i = 0; i < iso.length; i++) {
    h = (h * 31 + iso.charCodeAt(i)) >>> 0;
  }
  return h;
}

/** Tage in einem Monat (1-basiert). */
export function daysInMonth(year: number, month: number): number {
  return new Date(year, month, 0).getDate();
}

/** Vorhergehender Monat (1-basiert). */
export function prevMonth(year: number, month: number): { year: number; month: number } {
  if (month === 1) return { year: year - 1, month: 12 };
  return { year, month: month - 1 };
}

/** Generiert die letzten N (year, month)-Paare endend bei endYear/endMonth, chronologisch sortiert. */
export function monthRange(
  endYear: number,
  endMonth: number,
  months: number,
): { year: number; month: number }[] {
  const out: { year: number; month: number }[] = [];
  let y = endYear;
  let m = endMonth;
  for (let i = 0; i < months; i++) {
    out.unshift({ year: y, month: m });
    const p = prevMonth(y, m);
    y = p.year;
    m = p.month;
  }
  return out;
}

/**
 * Net-Worth-Wert (in Cent) für einen bestimmten (Jahr, Monat). Anker bei
 * Anfang 2023 mit ~50.000 EUR, wächst monatlich um ~2,5 % mit deterministischem
 * Rauschen. Glatte Kurve mit gelegentlichen Dips — geeignet für Sparklines
 * und NetWorthIndexChart.
 */
export function netWorthCents(year: number, month: number): number {
  const ANCHOR_YEAR = 2023;
  const monthsFromAnchor = (year - ANCHOR_YEAR) * 12 + (month - 1);
  const base = 50_000_00; // 50.000 EUR in Cent
  const growth = Math.pow(1.025, monthsFromAnchor);
  // Deterministisches Rauschen ±4 % aus (year, month)-Seed
  const rnd = mulberry32(ymSeed(year, month, 7))();
  const noise = 0.96 + rnd * 0.08;
  return Math.round(base * growth * noise);
}

/** Monatliche Inflow/Outflow (in Cent). Stabil pro (y, m). */
export function monthlyFlow(
  year: number,
  month: number,
  opts: { excludeInvest?: boolean } = {},
): { inCents: number; outCents: number } {
  const rndIn = mulberry32(ymSeed(year, month, 1));
  const rndOut = mulberry32(ymSeed(year, month, 2));
  const inCents = Math.round((3500_00 + rndIn() * 2000_00) * 100) / 100; // 3500–5500 EUR
  let outCents = Math.round((2400_00 + rndOut() * 1400_00) * 100) / 100; // 2400–3800 EUR
  if (opts.excludeInvest) {
    // Investitionen ausgeklammert ≈ 400 EUR weniger Outflow
    outCents = Math.max(0, outCents - 400_00);
  }
  return { inCents, outCents };
}
