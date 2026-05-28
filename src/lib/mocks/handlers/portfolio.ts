import type {
  Security,
  Holding,
  AllocationSlice,
  PortfolioKpis,
  DividendEntry,
  TradeWithTx,
  AccountMarketValue,
  PriceRow,
  RefreshReport,
  SecurityBucketAllocation,
  BucketHoldingRow,
  SecurityBreakdown,
} from '$lib/api';
import type { HandlerRegistry } from '../tauri-shim';
import type { MockStore } from '../store';
import {
  SEED_SECURITIES,
  SEED_HOLDINGS,
  generateDividends,
  generateTrades,
  generatePriceHistory,
} from '../fixtures/securities';

export function createPortfolioHandlers(_store: MockStore): HandlerRegistry {
  // Lokaler Mock-State, geteilt zwischen den Portfolio-Handlern
  const securities: Security[] = SEED_SECURITIES.map((s) => ({ ...s }));
  const holdings: Holding[] = SEED_HOLDINGS.map((h) => ({ ...h }));
  const dividends: DividendEntry[] = generateDividends();
  const trades: TradeWithTx[] = generateTrades();
  const breakdowns: Map<string, SecurityBreakdown[]> = new Map();
  const allocations: Map<number, SecurityBucketAllocation[]> = new Map();

  function bdKey(securityId: number, dimension: string): string {
    return `${securityId}:${dimension}`;
  }

  return {
    list_securities: (raw) => {
      const args = (raw ?? {}) as { includeArchived?: boolean };
      const source = args.includeArchived ? securities : securities.filter((s) => !s.archived);
      return source.map((s) => ({ ...s }));
    },

    get_security: (raw) => {
      const { id } = raw as { id: number };
      const sec = securities.find((s) => s.id === id);
      if (!sec) throw { message: `Security ${id} not found` };
      return { ...sec };
    },

    create_security: (raw) => {
      const { payload } = raw as { payload: Omit<Security, 'id' | 'archived' | 'createdAt'> };
      const sec: Security = {
        ...(payload as Security),
        id: Math.max(...securities.map((s) => s.id)) + 1,
        archived: false,
        createdAt: new Date().toISOString(),
      };
      securities.push(sec);
      return { ...sec };
    },

    update_security: (raw) => {
      const { id, payload } = raw as { id: number; payload: Partial<Security> };
      const idx = securities.findIndex((s) => s.id === id);
      if (idx === -1) throw { message: `Security ${id} not found` };
      securities[idx] = { ...securities[idx], ...payload };
      return { ...securities[idx] };
    },

    delete_security: (raw) => {
      const { id } = raw as { id: number };
      const idx = securities.findIndex((s) => s.id === id);
      if (idx === -1) return false;
      securities.splice(idx, 1);
      return true;
    },

    list_holdings: () => holdings.map((h) => ({ ...h })),

    list_trades: (raw) => {
      const args = (raw ?? {}) as { securityId?: number | null };
      const filtered =
        args.securityId == null
          ? trades
          : trades.filter((t) => t.trade.securityId === args.securityId);
      return filtered.map((t) => ({ trade: { ...t.trade }, tx: { ...t.tx } }));
    },

    get_trade: (raw) => {
      const { txId } = raw as { txId: number };
      const found = trades.find((t) => t.trade.txId === txId);
      if (!found) throw { message: `Trade ${txId} not found` };
      return { trade: { ...found.trade }, tx: { ...found.tx } };
    },

    create_trade: () => {
      throw { message: 'Mock: create_trade nicht unterstützt' };
    },
    update_trade: () => {
      throw { message: 'Mock: update_trade nicht unterstützt' };
    },
    delete_trade: () => true,

    dividend_history: () => dividends.map((d) => ({ ...d })),

    asset_allocation: (raw) => {
      const args = raw as { dimension: 'asset_type' | 'country' | 'sector' };
      const buckets = new Map<string, number>();
      for (const h of holdings) {
        const sec = securities.find((s) => s.id === h.securityId);
        if (!sec) continue;
        let key: string | null;
        if (args.dimension === 'asset_type') key = sec.assetType;
        else if (args.dimension === 'country') key = sec.country ?? 'unbekannt';
        else key = sec.sector ?? 'unbekannt';
        buckets.set(key, (buckets.get(key) ?? 0) + h.marketValueCents);
      }
      const slices: AllocationSlice[] = [];
      for (const [key, valueCents] of buckets) slices.push({ key, valueCents });
      return slices;
    },

    portfolio_kpis: (_raw) => {
      const marketValueCents = holdings.reduce((s, h) => s + h.marketValueCents, 0);
      const costBasisCents = holdings.reduce((s, h) => s + h.costBasisCents, 0);
      const result: PortfolioKpis = {
        marketValueCents,
        costBasisCents,
        unrealizedCents: marketValueCents - costBasisCents,
        realizedYtdCents: 0,
      };
      return result;
    },

    portfolio_value_by_account_today: () => {
      // Alle holdings landen am Depot-Account (4)
      const result: AccountMarketValue[] = [
        {
          accountId: 4,
          marketValueCents: holdings.reduce((s, h) => s + h.marketValueCents, 0),
        },
      ];
      return result;
    },

    realized_gains_summary: () => 0,

    get_price_history: (raw) => {
      const { securityId } = raw as { securityId: number };
      return generatePriceHistory(securityId, 90);
    },

    set_manual_price: () => null,

    fetch_security_history: () => 0,

    refresh_prices: () => {
      const report: RefreshReport = {
        securitiesTotal: securities.length,
        pricesUpdated: securities.length,
        pricesFailed: 0,
        fxUpdated: 0,
        fxFailed: 0,
      };
      return report;
    },

    get_breakdown: (raw) => {
      const args = raw as { securityId: number; dimension: 'country' | 'sector' };
      return (breakdowns.get(bdKey(args.securityId, args.dimension)) ?? []).map((b) => ({ ...b }));
    },

    set_breakdown: (raw) => {
      const args = raw as {
        securityId: number;
        dimension: 'country' | 'sector';
        rows: { key: string; weightBps: number }[];
      };
      breakdowns.set(
        bdKey(args.securityId, args.dimension),
        args.rows.map((r) => ({
          securityId: args.securityId,
          dimension: args.dimension,
          key: r.key,
          weightBps: r.weightBps,
        })),
      );
      return null;
    },

    list_security_allocations: (raw) => {
      const { securityId } = raw as { securityId: number };
      return (allocations.get(securityId) ?? []).map((a) => ({ ...a }));
    },

    set_security_allocations: (raw) => {
      const args = raw as { securityId: number; items: { bucketId: number; sharesMicro: number }[] };
      const list: SecurityBucketAllocation[] = args.items.map((item, idx) => ({
        id: idx + 1,
        securityId: args.securityId,
        bucketId: item.bucketId,
        sharesMicro: item.sharesMicro,
      }));
      allocations.set(args.securityId, list);
      return null;
    },

    bucket_holdings: (raw) => {
      const { bucketId: _bucketId } = raw as { bucketId: number };
      const result: BucketHoldingRow[] = [];
      return result;
    },
  };
}
