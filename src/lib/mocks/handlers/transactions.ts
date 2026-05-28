import type {
  Transaction,
  TxFilter,
  ListTransactionsPage,
  TxAggregate,
  NewTransactionPayload,
  UpdateTransactionPayload,
  CategorySuggestion,
} from '$lib/api';
import type { HandlerRegistry } from '../tauri-shim';
import type { MockStore } from '../store';

const DEFAULT_LIMIT = 200;
const MAX_LIMIT = 5000;

/** Order: (booking_date desc, id desc). */
function sortDesc(rows: Transaction[]): Transaction[] {
  return rows.slice().sort((a, b) => {
    if (a.booking_date !== b.booking_date) return b.booking_date.localeCompare(a.booking_date);
    return b.id - a.id;
  });
}

function matchesFilter(tx: Transaction, f: TxFilter | null | undefined): boolean {
  if (!f) return true;
  if (f.accountId !== undefined && f.accountId !== null && tx.account_id !== f.accountId) {
    return false;
  }
  if (f.categoryId !== undefined && tx.category_id !== f.categoryId) return false;
  if (f.bucketId !== undefined && tx.bucket_id !== f.bucketId) return false;
  if (f.uncategorized && tx.category_id !== null) return false;
  if (f.from && tx.booking_date < f.from) return false;
  if (f.to && tx.booking_date > f.to) return false;
  if (f.minAmountCents !== undefined && Math.abs(tx.amount_cents) < f.minAmountCents) {
    return false;
  }
  if (f.search) {
    const needle = f.search.toLowerCase();
    const hay = `${tx.counterparty ?? ''} ${tx.purpose ?? ''}`.toLowerCase();
    if (!hay.includes(needle)) return false;
  }
  return true;
}

function parseCursor(cursor: string): { date: string; id: number } | null {
  const parts = cursor.split('|');
  if (parts.length !== 2) return null;
  const id = Number(parts[1]);
  if (!Number.isFinite(id)) return null;
  return { date: parts[0], id };
}

function isBeforeCursor(tx: Transaction, c: { date: string; id: number }): boolean {
  if (tx.booking_date < c.date) return true;
  if (tx.booking_date > c.date) return false;
  return tx.id < c.id;
}

function txClone(tx: Transaction): Transaction {
  return { ...tx };
}

export function createTransactionHandlers(store: MockStore): HandlerRegistry {
  return {
    list_transactions: (raw) => {
      const args = (raw ?? {}) as { filter?: TxFilter | null };
      const f = args.filter ?? null;
      const limit = Math.max(1, Math.min(f?.limit ?? DEFAULT_LIMIT, MAX_LIMIT));

      let rows = sortDesc(store.transactions.filter((tx) => matchesFilter(tx, f)));

      if (f?.cursor) {
        const c = parseCursor(f.cursor);
        if (c) rows = rows.filter((tx) => isBeforeCursor(tx, c));
      }

      const page = rows.slice(0, limit).map(txClone);
      const hasMore = rows.length > limit;
      const nextCursor = hasMore
        ? `${page[page.length - 1].booking_date}|${page[page.length - 1].id}`
        : null;

      const result: ListTransactionsPage = { rows: page, nextCursor, hasMore };
      return result;
    },

    aggregate_transactions: (raw) => {
      const args = (raw ?? {}) as { filter?: TxFilter | null };
      const f = args.filter ?? null;
      let inCents = 0;
      let outCents = 0;
      let count = 0;
      for (const tx of store.transactions) {
        if (!matchesFilter(tx, f)) continue;
        count++;
        if (tx.amount_cents > 0) inCents += tx.amount_cents;
        else outCents += -tx.amount_cents;
      }
      const result: TxAggregate = { inCents, outCents, count };
      return result;
    },

    create_transaction: (raw) => {
      const args = raw as { tx: NewTransactionPayload };
      const p = args.tx;
      const tx: Transaction = {
        id: store.nextTransactionId++,
        account_id: p.accountId,
        booking_date: p.bookingDate,
        value_date: p.bookingDate,
        amount_cents: p.amountCents,
        currency: p.currency ?? 'EUR',
        counterparty: p.counterparty ?? null,
        purpose: p.purpose ?? null,
        raw_ref: null,
        category_id: p.categoryId ?? null,
        source: 'mock',
        source_file_hash: null,
        imported_at: new Date().toISOString(),
        manual_note: p.manualNote ?? null,
        bucket_id: p.bucketId ?? null,
        kind: p.kind ?? (p.amountCents >= 0 ? 'income' : 'expense'),
        counterparty_iban: p.counterpartyIban ?? null,
        holding_account_id: null,
        trade_side: null,
        paired_tx_id: null,
      };
      store.transactions.push(tx);
      return txClone(tx);
    },

    update_transaction: (raw) => {
      const args = raw as { tx: UpdateTransactionPayload };
      const idx = store.transactions.findIndex((t) => t.id === args.tx.id);
      if (idx === -1) throw { message: `Transaction ${args.tx.id} not found` };
      const cur = store.transactions[idx];
      const updated: Transaction = {
        ...cur,
        account_id: args.tx.accountId,
        booking_date: args.tx.bookingDate,
        amount_cents: args.tx.amountCents,
        currency: args.tx.currency,
        counterparty: args.tx.counterparty,
        purpose: args.tx.purpose,
        category_id: args.tx.categoryId,
        bucket_id: args.tx.bucketId,
        manual_note: args.tx.manualNote,
        counterparty_iban: args.tx.counterpartyIban,
        kind: args.tx.kind ?? cur.kind,
      };
      store.transactions[idx] = updated;
      return txClone(updated);
    },

    delete_transaction: (raw) => {
      const { id } = raw as { id: number };
      const idx = store.transactions.findIndex((t) => t.id === id);
      if (idx !== -1) store.transactions.splice(idx, 1);
      return null;
    },

    assign_category: (raw) => {
      const args = raw as { transactionId: number; categoryId: number | null };
      const idx = store.transactions.findIndex((t) => t.id === args.transactionId);
      if (idx === -1) throw { message: `Transaction ${args.transactionId} not found` };
      store.transactions[idx] = { ...store.transactions[idx], category_id: args.categoryId };
      return null;
    },

    assign_account: (raw) => {
      const args = raw as { transactionId: number; accountId: number };
      const idx = store.transactions.findIndex((t) => t.id === args.transactionId);
      if (idx === -1) throw { message: `Transaction ${args.transactionId} not found` };
      store.transactions[idx] = { ...store.transactions[idx], account_id: args.accountId };
      return null;
    },

    assign_bucket: (raw) => {
      const args = raw as { transactionId: number; bucketId: number | null };
      const idx = store.transactions.findIndex((t) => t.id === args.transactionId);
      if (idx === -1) throw { message: `Transaction ${args.transactionId} not found` };
      store.transactions[idx] = { ...store.transactions[idx], bucket_id: args.bucketId };
      return null;
    },

    suggest_category: (raw) => {
      const args = raw as { name: string; accountId?: number | null };
      const needle = args.name.toLowerCase();
      // Look at past txs with non-null category_id that share a counterparty substring
      const candidates = store.transactions.filter((t) => {
        if (t.category_id === null) return false;
        if (!t.counterparty) return false;
        return needle.includes(t.counterparty.toLowerCase()) ||
          t.counterparty.toLowerCase().includes(needle);
      });
      if (candidates.length === 0) return null;
      // Pick the most common category among candidates
      const counts = new Map<number, number>();
      for (const t of candidates) {
        counts.set(t.category_id!, (counts.get(t.category_id!) ?? 0) + 1);
      }
      let bestId = -1;
      let bestCount = 0;
      for (const [id, c] of counts) {
        if (c > bestCount) {
          bestId = id;
          bestCount = c;
        }
      }
      const cat = store.categories.find((c) => c.id === bestId);
      if (!cat) return null;
      const result: CategorySuggestion = {
        categoryId: cat.id,
        categoryName: cat.name,
        score: bestCount / candidates.length,
      };
      return result;
    },

    detect_transfers: () => 0,
    cleanup_phantom_mirrors: () => 0,
  };
}
