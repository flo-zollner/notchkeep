import type { Transaction } from '$lib/api';

export interface DayGroup {
  date: string;       // YYYY-MM-DD
  totalCents: number; // Summe amount_cents in dieser Gruppe (inkl. positives + negatives)
  txs: Transaction[];
}

/** Gruppiert eine bereits absteigend nach Datum sortierte Tx-Liste in Tages-Buckets. */
export function groupByDay(txs: Transaction[]): DayGroup[] {
  const groups: DayGroup[] = [];
  let current: DayGroup | null = null;
  for (const tx of txs) {
    if (!current || current.date !== tx.booking_date) {
      current = { date: tx.booking_date, totalCents: 0, txs: [] };
      groups.push(current);
    }
    current.txs.push(tx);
    current.totalCents += tx.amount_cents;
  }
  return groups;
}
