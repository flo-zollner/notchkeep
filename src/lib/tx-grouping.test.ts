import { describe, it, expect } from 'vitest';
import { groupByDay } from './tx-grouping';
import type { Transaction } from './api';

function tx(date: string, amount: number, id = Math.floor(Math.random() * 1_000_000)): Transaction {
  return {
    id,
    account_id: 1,
    booking_date: date,
    value_date: null,
    amount_cents: amount,
    currency: 'EUR',
    counterparty: null,
    purpose: null,
    raw_ref: null,
    category_id: null,
    source: 'manual',
    source_file_hash: null,
    imported_at: '2026-01-01T00:00:00Z',
    manual_note: null,
    bucket_id: null,
    kind: 'expense',
    counterparty_iban: null,
    holding_account_id: null,
    trade_side: null,
    paired_tx_id: null,
  };
}

describe('groupByDay', () => {
  it('returns empty array for empty input', () => {
    expect(groupByDay([])).toEqual([]);
  });

  it('groups single transaction into one bucket', () => {
    const result = groupByDay([tx('2026-01-15', 1000)]);
    expect(result).toHaveLength(1);
    expect(result[0].date).toBe('2026-01-15');
    expect(result[0].totalCents).toBe(1000);
    expect(result[0].txs).toHaveLength(1);
  });

  it('groups multiple transactions of same day into one bucket and sums them', () => {
    const result = groupByDay([
      tx('2026-01-15', 1000),
      tx('2026-01-15', -500),
      tx('2026-01-15', 200),
    ]);
    expect(result).toHaveLength(1);
    expect(result[0].totalCents).toBe(700); // 1000 - 500 + 200
    expect(result[0].txs).toHaveLength(3);
  });

  it('creates separate buckets for different days, preserving input order', () => {
    const result = groupByDay([
      tx('2026-01-15', 1000),
      tx('2026-01-14', -500),
      tx('2026-01-13', 200),
    ]);
    expect(result).toHaveLength(3);
    expect(result.map((g) => g.date)).toEqual(['2026-01-15', '2026-01-14', '2026-01-13']);
    expect(result.map((g) => g.totalCents)).toEqual([1000, -500, 200]);
  });

  it('handles negative-only days correctly', () => {
    const result = groupByDay([
      tx('2026-01-15', -100),
      tx('2026-01-15', -200),
    ]);
    expect(result[0].totalCents).toBe(-300);
  });

  it('respects input order — does NOT re-sort across order breaks', () => {
    // Function assumes input is already sorted descending. When the same date
    // appears non-consecutively, it opens a new bucket rather than merging.
    const result = groupByDay([
      tx('2026-01-15', 100),
      tx('2026-01-14', 200),
      tx('2026-01-15', 300), // same date as first but after a different day
    ]);
    expect(result).toHaveLength(3); // 3 separate buckets, not 2
    expect(result.map((g) => g.date)).toEqual(['2026-01-15', '2026-01-14', '2026-01-15']);
  });

  it('each bucket contains the correct transactions', () => {
    const t1 = tx('2026-03-01', 500, 1);
    const t2 = tx('2026-03-01', 300, 2);
    const t3 = tx('2026-02-28', 100, 3);
    const result = groupByDay([t1, t2, t3]);
    expect(result[0].txs).toEqual([t1, t2]);
    expect(result[1].txs).toEqual([t3]);
  });
});
