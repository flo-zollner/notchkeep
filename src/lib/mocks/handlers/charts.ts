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
import type { HandlerRegistry } from '../tauri-shim';
import type { MockStore } from '../store';
import {
  monthRange,
  netWorthCents,
  monthlyFlow,
  mulberry32,
  ymSeed,
  dateSeed,
  daysInMonth,
} from '../fixtures/timeseries';

/** Default-Anker für Mock-History: Januar 2023. */
const ANCHOR = { year: 2023, month: 1 };

function monthsSinceAnchor(year: number, month: number): number {
  return (year - ANCHOR.year) * 12 + (month - ANCHOR.month) + 1;
}

export function createChartHandlers(store: MockStore): HandlerRegistry {
  return {
    monthly_cashflow: (raw) => {
      const args = raw as {
        endYear: number;
        endMonth: number;
        months: number;
        excludeInvest?: boolean | null;
      };
      const range = monthRange(args.endYear, args.endMonth, args.months);
      const result: MonthlyFlow[] = range.map(({ year, month }) => {
        const f = monthlyFlow(year, month, {
          excludeInvest: args.excludeInvest ?? false,
        });
        return { year, month, inCents: f.inCents, outCents: f.outCents };
      });
      return result;
    },

    account_monthly_cashflow: (raw) => {
      const args = raw as {
        accountId: number;
        endYear: number;
        endMonth: number;
        months: number;
      };
      // Account-Anteil = 1/N (gleichmäßig auf aktive Konten verteilt)
      const activeCount = Math.max(1, store.accounts.filter((a) => !a.archived).length);
      const range = monthRange(args.endYear, args.endMonth, args.months);
      return range.map(({ year, month }) => {
        const f = monthlyFlow(year, month);
        const rnd = mulberry32(ymSeed(year, month, args.accountId))();
        // Anteil 60–140% des Durchschnitts, damit Konten unterschiedlich aussehen
        const factor = (0.6 + rnd * 0.8) / activeCount;
        return {
          year,
          month,
          inCents: Math.round(f.inCents * factor),
          outCents: Math.round(f.outCents * factor),
        };
      });
    },

    net_worth_history: (raw) => {
      const args = raw as { endYear: number; endMonth: number; months: number };
      // months=0 → komplette History seit Anker (UI-Konvention auf /networth)
      const total =
        args.months === 0 ? monthsSinceAnchor(args.endYear, args.endMonth) : args.months;
      const range = monthRange(args.endYear, args.endMonth, total);
      const result: NetWorthPoint[] = range.map(({ year, month }) => ({
        year,
        month,
        totalCents: netWorthCents(year, month),
      }));
      return result;
    },

    net_worth_forecast: (raw) => {
      const args = raw as {
        endYear: number;
        endMonth: number;
        historyWindow: number;
        forecastMonths: number;
      };
      // Forecast extrapoliert ab endYear/endMonth nach vorn.
      const out: NetWorthForecastPoint[] = [];
      let y = args.endYear;
      let m = args.endMonth;
      for (let i = 0; i < args.forecastMonths; i++) {
        m += 1;
        if (m > 12) {
          m = 1;
          y += 1;
        }
        const mid = netWorthCents(y, m);
        // Konfidenz-Band weitet sich mit Horizont
        const spread = Math.round(mid * 0.04 * (i + 1));
        out.push({
          year: y,
          month: m,
          midCents: mid,
          loCents: mid - spread,
          hiCents: mid + spread,
        });
      }
      return out;
    },

    daily_spending: (raw) => {
      const args = raw as { year: number; month: number };
      const days = daysInMonth(args.year, args.month);
      const out: number[] = [];
      for (let d = 1; d <= days; d++) {
        const rnd = mulberry32(ymSeed(args.year, args.month, d))();
        const dow = new Date(args.year, args.month - 1, d).getDay();
        const weekendBoost = dow === 0 || dow === 6 ? 1.6 : 1.0;
        // 0–80 EUR werktags, höher am Wochenende, plus gelegentliche 0-Tage
        const skipDay = rnd < 0.15;
        const value = skipDay ? 0 : Math.round(rnd * 80_00 * weekendBoost);
        out.push(value);
      }
      return out;
    },

    monthly_spending: (raw) => {
      const args = raw as { year: number; month: number };
      // Pro Top-Level-Kategorie (parent_id null), Ausgaben-Heuristik
      const cats = store.categories.filter((c) => c.parent_id === null);
      return cats.map((c, idx): CategorySpending => {
        const rnd = mulberry32(ymSeed(args.year, args.month, c.id + idx))();
        // Spending zwischen 20 und 800 EUR, einige Kategorien (Einkommen) negativ/0
        const isIncome = c.name === 'Einkommen';
        const isInvest = c.name === 'Investitionen';
        const base = isIncome ? 0 : isInvest ? 600_00 : 80_00 + rnd * 720_00;
        return {
          categoryId: c.id,
          spentCents: Math.round(base),
        };
      });
    },

    category_breakdown: (raw) => {
      const args = raw as { from: string; to: string };
      // Aggregiere über alle Top-Level-Kategorien gewichtet mit Monatszahl im Range
      const cats = store.categories.filter((c) => c.parent_id === null);
      const monthsApprox = Math.max(
        1,
        Math.round(
          (Date.parse(args.to) - Date.parse(args.from)) / (1000 * 60 * 60 * 24 * 30),
        ),
      );
      return cats.map((c): CategorySpending => {
        const rnd = mulberry32(dateSeed(args.from + args.to, c.id))();
        const isIncome = c.name === 'Einkommen';
        const base = isIncome ? 0 : 80_00 + rnd * 720_00;
        return {
          categoryId: c.id,
          spentCents: Math.round(base * monthsApprox),
        };
      });
    },

    cashflow_breakdown: (raw) => {
      const args = raw as { from: string; to: string };
      // Für Sankey: pro Kategorie ein Slice +/- je nach Charakter
      const cats = store.categories.filter((c) => c.parent_id === null);
      const slices: CashflowSlice[] = [];
      for (const c of cats) {
        const isIncome = c.name === 'Einkommen';
        const rnd = mulberry32(dateSeed(args.from + args.to, c.id + 100))();
        const sign: 1 | -1 = isIncome ? 1 : -1;
        const value = isIncome ? 3500_00 + rnd * 2000_00 : 80_00 + rnd * 720_00;
        slices.push({
          categoryId: c.id,
          sign,
          sumAbsCents: Math.round(value),
        });
      }
      // Plus ein uncategorized-Slice (categoryId null) wie das echte Backend
      slices.push({
        categoryId: null,
        sign: -1,
        sumAbsCents: 60_00,
      });
      return slices;
    },

    uncategorized_monthly_spent: (raw) => {
      const args = raw as { year: number; month: number };
      const rnd = mulberry32(ymSeed(args.year, args.month, 99))();
      return Math.round(40_00 + rnd * 120_00);
    },

    investment_flow_for_month: (raw) => {
      const args = raw as { year: number; month: number };
      const rndB = mulberry32(ymSeed(args.year, args.month, 11));
      const rndS = mulberry32(ymSeed(args.year, args.month, 12));
      const rndD = mulberry32(ymSeed(args.year, args.month, 13));
      const buysCents = Math.round(400_00 + rndB() * 600_00);
      const sellsCents = Math.round(rndS() * 200_00);
      const dividendsCents = Math.round(rndD() * 80_00);
      const result: InvestmentFlow = {
        buysCents,
        sellsCents,
        dividendsCents,
        netInvestedCents: buysCents - sellsCents,
      };
      return result;
    },

    cost_basis_history: (raw) => {
      const args = raw as { endYear: number; endMonth: number; months: number };
      const range = monthRange(args.endYear, args.endMonth, args.months);
      return range.map(({ year, month }): CostBasisPoint => {
        const nw = netWorthCents(year, month);
        // Portfolio-Anteil ~40 % am Net Worth, Marktwert > Cost Basis im Schnitt
        const marketValueCents = Math.round(nw * 0.4);
        const costBasisCents = Math.round(marketValueCents * 0.85);
        return { year, month, costBasisCents, marketValueCents };
      });
    },

    cost_basis_history_daily: (raw) => {
      const args = raw as { endDate: string; days: number };
      const end = new Date(args.endDate);
      const out: CostBasisPointDaily[] = [];
      for (let i = args.days - 1; i >= 0; i--) {
        const d = new Date(end);
        d.setDate(end.getDate() - i);
        const iso = d.toISOString().slice(0, 10);
        const baseNw = netWorthCents(d.getFullYear(), d.getMonth() + 1);
        const rnd = mulberry32(dateSeed(iso, 17))();
        const wiggle = 0.98 + rnd * 0.04;
        const marketValueCents = Math.round(baseNw * 0.4 * wiggle);
        const costBasisCents = Math.round(marketValueCents * 0.85);
        out.push({ date: iso, costBasisCents, marketValueCents });
      }
      return out;
    },
  };
}
