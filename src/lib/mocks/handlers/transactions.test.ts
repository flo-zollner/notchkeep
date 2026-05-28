import { describe, it, expect } from 'vitest';
import { createMockTauriInvoke } from '../index';
import type { ListTransactionsPage, Transaction, TxAggregate } from '$lib/api';

describe('transactions handlers', () => {
  it('list_transactions returns rows in date-desc order with default limit', async () => {
    const invoke = createMockTauriInvoke();
    const page = await invoke<ListTransactionsPage>('list_transactions', { filter: null });

    expect(page.rows.length).toBeGreaterThan(0);
    expect(page.rows.length).toBeLessThanOrEqual(200);
    for (let i = 1; i < page.rows.length; i++) {
      const prev = page.rows[i - 1];
      const cur = page.rows[i];
      // Same date or earlier
      expect(prev.booking_date >= cur.booking_date).toBe(true);
    }
  });

  it('list_transactions paginates via opaque cursor', async () => {
    const invoke = createMockTauriInvoke();
    const first = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { limit: 10 },
    });

    expect(first.rows).toHaveLength(10);
    expect(first.hasMore).toBe(true);
    expect(first.nextCursor).toBeTruthy();

    const second = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { limit: 10, cursor: first.nextCursor },
    });

    expect(second.rows.length).toBeGreaterThan(0);
    // No overlap with first page
    const firstIds = new Set(first.rows.map((r) => r.id));
    expect(second.rows.every((r) => !firstIds.has(r.id))).toBe(true);
  });

  it('list_transactions respects accountId filter', async () => {
    const invoke = createMockTauriInvoke();
    const all = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { limit: 5000 },
    });
    const target = all.rows[0].account_id;

    const filtered = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { accountId: target, limit: 5000 },
    });

    expect(filtered.rows.length).toBeGreaterThan(0);
    expect(filtered.rows.every((r) => r.account_id === target)).toBe(true);
  });

  it('list_transactions respects uncategorized filter', async () => {
    const invoke = createMockTauriInvoke();
    const uncat = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { uncategorized: true, limit: 5000 },
    });
    expect(uncat.rows.every((r) => r.category_id === null)).toBe(true);
  });

  it('list_transactions respects from/to date range', async () => {
    const invoke = createMockTauriInvoke();
    const page = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { from: '2026-01-01', to: '2026-03-31', limit: 5000 },
    });
    for (const r of page.rows) {
      expect(r.booking_date >= '2026-01-01').toBe(true);
      expect(r.booking_date <= '2026-03-31').toBe(true);
    }
  });

  it('aggregate_transactions returns coherent inCents/outCents/count', async () => {
    const invoke = createMockTauriInvoke();
    const agg = await invoke<TxAggregate>('aggregate_transactions', { filter: null });

    expect(agg.count).toBeGreaterThan(0);
    expect(agg.inCents).toBeGreaterThanOrEqual(0);
    expect(agg.outCents).toBeGreaterThanOrEqual(0);
  });

  it('create_transaction appends to the store', async () => {
    const invoke = createMockTauriInvoke();
    const before = await invoke<TxAggregate>('aggregate_transactions', { filter: null });

    const created = await invoke<Transaction>('create_transaction', {
      tx: {
        accountId: 1,
        bookingDate: '2026-05-28',
        amountCents: -1234,
        currency: 'EUR',
        counterparty: 'Test',
        purpose: 'Unit-Test',
      },
    });

    expect(created.amount_cents).toBe(-1234);
    expect(created.counterparty).toBe('Test');

    const after = await invoke<TxAggregate>('aggregate_transactions', { filter: null });
    expect(after.count).toBe(before.count + 1);
  });

  it('delete_transaction removes the row', async () => {
    const invoke = createMockTauriInvoke();
    const page = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { limit: 1 },
    });
    const victim = page.rows[0];

    await invoke('delete_transaction', { id: victim.id });

    const after = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { limit: 5000 },
    });
    expect(after.rows.find((r) => r.id === victim.id)).toBeUndefined();
  });

  it('assign_category updates the row category_id', async () => {
    const invoke = createMockTauriInvoke();
    const page = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { uncategorized: true, limit: 1 },
    });
    if (page.rows.length === 0) return; // no uncategorized rows seeded
    const target = page.rows[0];

    await invoke('assign_category', { transactionId: target.id, categoryId: 1 });

    const reloaded = await invoke<ListTransactionsPage>('list_transactions', {
      filter: { limit: 5000 },
    });
    const updated = reloaded.rows.find((r) => r.id === target.id);
    expect(updated?.category_id).toBe(1);
  });

  it('suggest_category returns null or a valid suggestion', async () => {
    const invoke = createMockTauriInvoke();
    const sugg = await invoke<{ categoryId: number; categoryName: string; score: number } | null>(
      'suggest_category',
      { name: 'Lebensmittel REWE', accountId: null },
    );
    if (sugg !== null) {
      expect(typeof sugg.categoryId).toBe('number');
      expect(sugg.score).toBeGreaterThan(0);
    }
  });
});
