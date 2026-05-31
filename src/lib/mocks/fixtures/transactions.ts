import type { Transaction } from '$lib/api';
import { mulberry32 } from './timeseries';

/**
 * Deterministischer Generator für Mock-Transaktionen. Erzeugt eine plausibel
 * gestreute Sequenz über die letzten 24 Monate, ohne reale Marken-/Personen-
 * namen (CONTRIBUTING.md, Privatsphäre). Counterparty-Pool ist absichtlich generisch.
 */

const COUNTERPARTIES = {
  groceries: ['Supermarkt', 'Bioladen', 'Bäckerei', 'Wochenmarkt', 'Drogerie'],
  dining: ['Restaurant Eckstein', 'Café Mitte', 'Imbiss am Park', 'Pizzeria Mustermann'],
  transport: ['Verkehrsbetriebe', 'Tankstelle', 'Carsharing', 'Bahn'],
  utilities: ['Stadtwerke', 'Versorger Strom', 'Wasser kommunal'],
  rent: ['Vermieter Mustermann GmbH'],
  income: ['Beispiel GmbH', 'Auftraggeber Demo'],
  subscriptions: ['Streaming-Abo', 'Cloud-Speicher', 'Newsletter Pro'],
  health: ['Apotheke', 'Praxis Demo'],
  shopping: ['Online-Shop', 'Buchhandlung', 'Elektronikmarkt'],
  invest: ['Sparplan-Ausführung', 'Kapitalmaßnahme'],
};

interface TxBlueprint {
  monthlyCount: number;
  amountRangeCents: [number, number];
  pool: string[];
  categoryId: number | null;
  kind: 'income' | 'expense';
}

const BLUEPRINTS: TxBlueprint[] = [
  // Monatliche fixe Kosten
  { monthlyCount: 1, amountRangeCents: [-128_000, -128_000], pool: COUNTERPARTIES.rent, categoryId: 21, kind: 'expense' },
  { monthlyCount: 2, amountRangeCents: [-12_000, -8_000], pool: COUNTERPARTIES.utilities, categoryId: 22, kind: 'expense' },
  { monthlyCount: 3, amountRangeCents: [-3_000, -800], pool: COUNTERPARTIES.subscriptions, categoryId: 6, kind: 'expense' },
  // Variable Kosten
  { monthlyCount: 8, amountRangeCents: [-7_500, -1_200], pool: COUNTERPARTIES.groceries, categoryId: 11, kind: 'expense' },
  { monthlyCount: 3, amountRangeCents: [-4_500, -1_500], pool: COUNTERPARTIES.dining, categoryId: 12, kind: 'expense' },
  { monthlyCount: 4, amountRangeCents: [-3_500, -300], pool: COUNTERPARTIES.transport, categoryId: 31, kind: 'expense' },
  { monthlyCount: 1, amountRangeCents: [-12_000, -3_500], pool: COUNTERPARTIES.transport, categoryId: 32, kind: 'expense' },
  { monthlyCount: 1, amountRangeCents: [-9_000, -2_200], pool: COUNTERPARTIES.health, categoryId: 5, kind: 'expense' },
  { monthlyCount: 2, amountRangeCents: [-15_000, -2_000], pool: COUNTERPARTIES.shopping, categoryId: null, kind: 'expense' }, // intentionally uncat
  // Einkommen
  { monthlyCount: 1, amountRangeCents: [380_000, 480_000], pool: COUNTERPARTIES.income, categoryId: 7, kind: 'income' },
  // Investitionen
  { monthlyCount: 1, amountRangeCents: [-50_000, -30_000], pool: COUNTERPARTIES.invest, categoryId: 8, kind: 'expense' },
];

/** YYYY-MM-DD string für (year, month, day). */
function isoDate(year: number, month: number, day: number): string {
  return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

function daysInMonth(year: number, month: number): number {
  return new Date(year, month, 0).getDate();
}

/** Erzeugt die Mock-Transaktionsliste deterministisch. */
export function generateSeedTransactions(opts: {
  endYear: number;
  endMonth: number;
  months: number;
}): Transaction[] {
  const txs: Transaction[] = [];
  let nextId = 1000;

  // Iteriere über jeden (year, month) im Range
  for (let i = opts.months - 1; i >= 0; i--) {
    let m = opts.endMonth - i;
    let y = opts.endYear;
    while (m <= 0) {
      m += 12;
      y -= 1;
    }
    const dim = daysInMonth(y, m);

    for (let bpIdx = 0; bpIdx < BLUEPRINTS.length; bpIdx++) {
      const bp = BLUEPRINTS[bpIdx];
      const rnd = mulberry32((y * 100 + m) * 1000 + bpIdx);

      for (let k = 0; k < bp.monthlyCount; k++) {
        const day = Math.min(dim, 1 + Math.floor(rnd() * dim));
        const [lo, hi] = bp.amountRangeCents;
        const amount = lo === hi ? lo : Math.round(lo + rnd() * (hi - lo));
        const counterparty = bp.pool[Math.floor(rnd() * bp.pool.length)];
        const accountId = bp.kind === 'income' ? 1 : bp.categoryId === 8 ? 3 : 1;

        txs.push({
          id: nextId++,
          account_id: accountId,
          booking_date: isoDate(y, m, day),
          value_date: isoDate(y, m, day),
          amount_cents: amount,
          currency: 'EUR',
          counterparty,
          purpose: null,
          raw_ref: null,
          category_id: bp.categoryId,
          source: 'mock',
          source_file_hash: null,
          imported_at: new Date(y, m - 1, day).toISOString(),
          manual_note: null,
          bucket_id: null,
          kind: bp.kind,
          counterparty_iban: null,
          holding_account_id: null,
          trade_side: null,
          paired_tx_id: null,
        });
      }
    }
  }

  // Sortiere absteigend nach (booking_date, id)
  txs.sort((a, b) => {
    if (a.booking_date !== b.booking_date) return b.booking_date.localeCompare(a.booking_date);
    return b.id - a.id;
  });

  return txs;
}

export const SEED_TRANSACTIONS = generateSeedTransactions({
  endYear: 2026,
  endMonth: 5,
  months: 24,
});
