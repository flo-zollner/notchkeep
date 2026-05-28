import { describe, it, expect } from 'vitest';
import { createMockTauriInvoke } from '../index';
import type {
  Holding,
  Security,
  AllocationSlice,
  PortfolioKpis,
  DividendEntry,
  TradeWithTx,
  AccountMarketValue,
  PriceRow,
  RefreshReport,
} from '$lib/api';

describe('portfolio handlers', () => {
  it('list_holdings returns Holding[] with synthetic ISINs', async () => {
    const invoke = createMockTauriInvoke();
    const holdings = await invoke<Holding[]>('list_holdings');

    expect(holdings.length).toBeGreaterThan(0);
    for (const h of holdings) {
      expect(h.isin).toMatch(/^[A-Z]{2}[0-9A-Z]{10}$/);
      // No real ticker brands
      expect(h.name).not.toMatch(/Vanguard|iShares|MSCI|S&P|Xtrackers|Lyxor/i);
      expect(h.sharesMicro).toBeGreaterThan(0);
    }
  });

  it('list_securities returns Security[] with at least one of each common asset type', async () => {
    const invoke = createMockTauriInvoke();
    const secs = await invoke<Security[]>('list_securities', { includeArchived: false });

    expect(secs.length).toBeGreaterThan(0);
    const types = new Set(secs.map((s) => s.assetType));
    expect(types.has('etf_equity') || types.has('stock')).toBe(true);
  });

  it('asset_allocation by asset_type returns slices summing to portfolio value', async () => {
    const invoke = createMockTauriInvoke();
    const holdings = await invoke<Holding[]>('list_holdings');
    const totalValue = holdings.reduce((s, h) => s + h.marketValueCents, 0);

    const slices = await invoke<AllocationSlice[]>('asset_allocation', {
      dimension: 'asset_type',
    });
    const sum = slices.reduce((s, a) => s + a.valueCents, 0);
    expect(sum).toBe(totalValue);
  });

  it('portfolio_kpis returns coherent numbers', async () => {
    const invoke = createMockTauriInvoke();
    const kpis = await invoke<PortfolioKpis>('portfolio_kpis', { year: 2026 });

    expect(kpis.marketValueCents).toBeGreaterThan(0);
    expect(kpis.costBasisCents).toBeGreaterThan(0);
    expect(kpis.unrealizedCents).toBe(kpis.marketValueCents - kpis.costBasisCents);
  });

  it('dividend_history returns DividendEntry[] tied to known securities', async () => {
    const invoke = createMockTauriInvoke();
    const divs = await invoke<DividendEntry[]>('dividend_history');
    const secs = await invoke<Security[]>('list_securities', { includeArchived: false });
    const secIds = new Set(secs.map((s) => s.id));

    expect(divs.length).toBeGreaterThan(0);
    for (const d of divs) {
      expect(secIds.has(d.securityId)).toBe(true);
      expect(d.amountCents).toBeGreaterThan(0);
    }
  });

  it('list_trades returns TradeWithTx[] with trade-kind transactions', async () => {
    const invoke = createMockTauriInvoke();
    const trades = await invoke<TradeWithTx[]>('list_trades', { securityId: null });

    expect(trades.length).toBeGreaterThan(0);
    for (const tw of trades) {
      expect(['buy', 'sell', 'dividend', 'corporate_action', 'tax']).toContain(
        tw.trade.side,
      );
    }
  });

  it('portfolio_value_by_account_today returns one entry per depot account', async () => {
    const invoke = createMockTauriInvoke();
    const vals = await invoke<AccountMarketValue[]>('portfolio_value_by_account_today');
    expect(vals.length).toBeGreaterThan(0);
    expect(vals.every((v) => typeof v.marketValueCents === 'number')).toBe(true);
  });

  it('get_price_history returns PriceRow[] for a known security', async () => {
    const invoke = createMockTauriInvoke();
    const secs = await invoke<Security[]>('list_securities', { includeArchived: false });
    const history = await invoke<PriceRow[]>('get_price_history', {
      securityId: secs[0].id,
    });
    expect(history.length).toBeGreaterThan(0);
    for (const p of history) {
      expect(p.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(p.closeMicro).toBeGreaterThan(0);
    }
  });

  it('refresh_prices returns a RefreshReport stub', async () => {
    const invoke = createMockTauriInvoke();
    const report = await invoke<RefreshReport>('refresh_prices');
    expect(typeof report.securitiesTotal).toBe('number');
    expect(typeof report.pricesUpdated).toBe('number');
  });
});
