import { describe, it, expect } from 'vitest';
import { createMockTauriInvoke } from '../index';
import type {
  MonthlyFlow,
  NetWorthPoint,
  NetWorthForecastPoint,
  CategorySpending,
  CashflowSlice,
  CostBasisPoint,
  CostBasisPointDaily,
  InvestmentFlow,
} from '$lib/api';

describe('charts handlers', () => {
  it('monthly_cashflow returns the requested number of months with both in/out > 0', async () => {
    const invoke = createMockTauriInvoke();
    const flows = await invoke<MonthlyFlow[]>('monthly_cashflow', {
      endYear: 2026,
      endMonth: 5,
      months: 12,
      excludeInvest: null,
    });

    expect(flows).toHaveLength(12);
    for (const f of flows) {
      expect(f.inCents).toBeGreaterThan(0);
      expect(f.outCents).toBeGreaterThan(0);
    }
    // Sortiert chronologisch
    expect(flows[0].year * 100 + flows[0].month).toBeLessThan(
      flows[11].year * 100 + flows[11].month,
    );
  });

  it('monthly_cashflow is deterministic — same inputs return same outputs', async () => {
    const a = await createMockTauriInvoke()<MonthlyFlow[]>('monthly_cashflow', {
      endYear: 2026,
      endMonth: 5,
      months: 6,
      excludeInvest: null,
    });
    const b = await createMockTauriInvoke()<MonthlyFlow[]>('monthly_cashflow', {
      endYear: 2026,
      endMonth: 5,
      months: 6,
      excludeInvest: null,
    });
    expect(a).toEqual(b);
  });

  it('net_worth_history grows on average over time', async () => {
    const invoke = createMockTauriInvoke();
    const hist = await invoke<NetWorthPoint[]>('net_worth_history', {
      endYear: 2026,
      endMonth: 5,
      months: 24,
    });

    expect(hist).toHaveLength(24);
    // Letzter Punkt > erster Punkt (Wachstumstrend, individuelle Dips ok)
    expect(hist[hist.length - 1].totalCents).toBeGreaterThan(hist[0].totalCents);
  });

  it('net_worth_history with months=0 returns full available history (>= 24)', async () => {
    const invoke = createMockTauriInvoke();
    const hist = await invoke<NetWorthPoint[]>('net_worth_history', {
      endYear: 2026,
      endMonth: 5,
      months: 0,
    });
    expect(hist.length).toBeGreaterThanOrEqual(24);
  });

  it('net_worth_forecast returns lo <= mid <= hi for each forecast point', async () => {
    const invoke = createMockTauriInvoke();
    const fc = await invoke<NetWorthForecastPoint[]>('net_worth_forecast', {
      endYear: 2026,
      endMonth: 5,
      historyWindow: 6,
      forecastMonths: 6,
    });

    expect(fc).toHaveLength(6);
    for (const p of fc) {
      expect(p.loCents).toBeLessThanOrEqual(p.midCents);
      expect(p.midCents).toBeLessThanOrEqual(p.hiCents);
    }
  });

  it('daily_spending returns an array length matching days in the month', async () => {
    const invoke = createMockTauriInvoke();
    const feb2026 = await invoke<number[]>('daily_spending', { year: 2026, month: 2 });
    const jan2026 = await invoke<number[]>('daily_spending', { year: 2026, month: 1 });

    expect(feb2026).toHaveLength(28);
    expect(jan2026).toHaveLength(31);
    expect(jan2026.every((v) => typeof v === 'number' && v >= 0)).toBe(true);
  });

  it('category_breakdown returns one entry per seeded leaf category in the range', async () => {
    const invoke = createMockTauriInvoke();
    const breakdown = await invoke<CategorySpending[]>('category_breakdown', {
      from: '2026-01-01',
      to: '2026-05-31',
    });

    expect(breakdown.length).toBeGreaterThan(0);
    expect(breakdown.every((e) => e.spentCents >= 0)).toBe(true);
  });

  it('cashflow_breakdown contains both positive and negative slices', async () => {
    const invoke = createMockTauriInvoke();
    const slices = await invoke<CashflowSlice[]>('cashflow_breakdown', {
      from: '2026-01-01',
      to: '2026-05-31',
    });

    expect(slices.some((s) => s.sign === 1)).toBe(true);
    expect(slices.some((s) => s.sign === -1)).toBe(true);
  });

  it('cost_basis_history typically has marketValue >= costBasis (growth scenario)', async () => {
    const invoke = createMockTauriInvoke();
    const hist = await invoke<CostBasisPoint[]>('cost_basis_history', {
      endYear: 2026,
      endMonth: 5,
      months: 12,
    });
    expect(hist).toHaveLength(12);
    // Final point shows growth (mock represents a profitable portfolio)
    const last = hist[hist.length - 1];
    expect(last.marketValueCents).toBeGreaterThan(last.costBasisCents);
  });

  it('cost_basis_history_daily returns N daily points', async () => {
    const invoke = createMockTauriInvoke();
    const hist = await invoke<CostBasisPointDaily[]>('cost_basis_history_daily', {
      endDate: '2026-05-28',
      days: 30,
    });
    expect(hist).toHaveLength(30);
    expect(hist[0].date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  it('investment_flow_for_month returns a coherent InvestmentFlow', async () => {
    const invoke = createMockTauriInvoke();
    const flow = await invoke<InvestmentFlow>('investment_flow_for_month', {
      year: 2026,
      month: 5,
    });
    expect(typeof flow.buysCents).toBe('number');
    expect(typeof flow.sellsCents).toBe('number');
    expect(typeof flow.dividendsCents).toBe('number');
    expect(flow.netInvestedCents).toBe(flow.buysCents - flow.sellsCents);
  });
});
