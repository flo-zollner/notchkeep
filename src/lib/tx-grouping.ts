import type { Transaction } from '$lib/api';

export interface DayGroup {
  date: string;       // YYYY-MM-DD
  totalCents: number; // Sum of amount_cents in this group (including positive and negative)
  txs: Transaction[];
}

/** Groups a transaction list that is already sorted descending by date into day buckets. */
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
