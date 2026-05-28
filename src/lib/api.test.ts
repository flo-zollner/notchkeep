import { describe, it, expect } from 'vitest';
import { isTradeTx, type Transaction } from './api';

function makeTx(overrides: Partial<Transaction>): Transaction {
  return {
    id: 1,
    account_id: 1,
    booking_date: '2026-01-01',
    value_date: null,
    amount_cents: 0,
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
    ...overrides,
  };
}

describe('isTradeTx', () => {
  it('returns true for kind="buy"', () => {
    expect(isTradeTx(makeTx({ kind: 'buy' }))).toBe(true);
  });

  it('returns true for kind="sell"', () => {
    expect(isTradeTx(makeTx({ kind: 'sell' }))).toBe(true);
  });

  it('returns true for kind="dividend"', () => {
    expect(isTradeTx(makeTx({ kind: 'dividend' }))).toBe(true);
  });

  it('returns true for kind="corporate_action"', () => {
    expect(isTradeTx(makeTx({ kind: 'corporate_action' }))).toBe(true);
  });

  it('returns true for kind="tax" (Thesaurierungs-KeSt)', () => {
    expect(isTradeTx(makeTx({ kind: 'tax' }))).toBe(true);
  });

  it('returns false for kind="expense" (regular cash outflow)', () => {
    expect(isTradeTx(makeTx({ kind: 'expense' }))).toBe(false);
  });

  it('returns false for kind="income"', () => {
    expect(isTradeTx(makeTx({ kind: 'income' }))).toBe(false);
  });

  it('returns false for kind="fee" (cash editing, not a trade)', () => {
    expect(isTradeTx(makeTx({ kind: 'fee' }))).toBe(false);
  });

  it('returns false for kind="transfer"', () => {
    expect(isTradeTx(makeTx({ kind: 'transfer' }))).toBe(false);
  });
});
