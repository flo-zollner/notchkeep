<script lang="ts">
  import CashflowChart from '$lib/components/CashflowChart.svelte';
  import KPI from '$lib/components/KPI.svelte';
  import SankeyChart from '$lib/components/SankeyChart.svelte';
  import { goto } from '$app/navigation';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';
  import { api, fmtEur, type MonthlyFlow, type CashflowSlice, type Category } from '$lib/api';

  type RangeId = '1M' | '3M' | '6M' | '12M' | 'all';
  const RANGE_MONTHS: Record<RangeId, number> = {
    '1M': 1,
    '3M': 3,
    '6M': 6,
    '12M': 12,
    all: 24,
  };

  const now = new Date();
  const endYear = now.getFullYear();
  const endMonth = now.getMonth() + 1;

  let range = $state<RangeId>('1M');
  let flow = $state<MonthlyFlow[]>([]);
  let loading = $state(true);

  type Tab = 'timeseries' | 'composition';
  let activeTab = $state<Tab>('timeseries');

  type CompPeriodKind = 'range' | 'month';
  let compPeriodKind = $state<CompPeriodKind>('range');
  let compRange = $state<RangeId>('1M');
  let compYear = $state(endYear);
  let compMonth = $state(endMonth);

  let breakdown = $state<CashflowSlice[]>([]);
  let breakdownFrom = $state<string>('');
  let breakdownTo = $state<string>('');
  let categories = $state<Category[]>([]);
  let compLoading = $state(false);

  $effect(() => {
    const months = RANGE_MONTHS[range];
    loading = true;
    api
      .monthlyCashflow(endYear, endMonth, months)
      .then((rows) => {
        flow = rows;
      })
      .finally(() => {
        loading = false;
      });
  });

  function periodToDates(): { from: string; to: string } {
    if (compPeriodKind === 'month') {
      const from = `${compYear}-${String(compMonth).padStart(2, '0')}-01`;
      const nextY = compMonth === 12 ? compYear + 1 : compYear;
      const nextM = compMonth === 12 ? 1 : compMonth + 1;
      const to = `${nextY}-${String(nextM).padStart(2, '0')}-01`;
      return { from, to };
    }
    const months = RANGE_MONTHS[compRange];
    const nowD = new Date();
    const to = new Date(nowD.getFullYear(), nowD.getMonth() + 1, 1)
      .toISOString().slice(0, 10);
    const fromDate = new Date(nowD.getFullYear(), nowD.getMonth() - months + 1, 1);
    const from = fromDate.toISOString().slice(0, 10);
    return { from, to };
  }

  async function loadBreakdown() {
    const { from, to } = periodToDates();
    compLoading = true;
    breakdownFrom = from;
    breakdownTo = to;
    try {
      const [b, c] = await Promise.all([
        api.cashflowBreakdown(from, to),
        api.listCategories(),
      ]);
      breakdown = b;
      categories = c;
    } finally {
      compLoading = false;
    }
  }

  $effect(() => {
    if (activeTab === 'composition') {
      void compPeriodKind;
      void compRange;
      void compYear;
      void compMonth;
      void loadBreakdown();
    }
  });

  function compStepMonth(delta: -1 | 1) {
    let y = compYear;
    let m = compMonth + delta;
    if (m < 1) { m = 12; y -= 1; }
    else if (m > 12) { m = 1; y += 1; }
    compYear = y;
    compMonth = m;
  }

  function monthLabel(year: number, month: number): string {
    const labels = t().months;
    const yy = String(year % 100).padStart(2, '0');
    return `${labels[month - 1]} ${yy}`;
  }

  const chartData = $derived(
    flow.map((f) => ({
      m: monthLabel(f.year, f.month),
      in: f.inCents / 100,
      out: f.outCents / 100,
      year: f.year,
      month: f.month,
    })),
  );

  function navigateToMonthTx(year: number, month: number) {
    const m = String(month).padStart(2, '0');
    const from = `${year}-${m}-01`;
    const lastDay = new Date(year, month, 0).getDate();
    const to = `${year}-${m}-${String(lastDay).padStart(2, '0')}`;
    goto(`/transactions?from=${from}&to=${to}`);
  }

  const totalIn = $derived(flow.reduce((s, f) => s + f.inCents, 0));
  const totalOut = $derived(flow.reduce((s, f) => s + f.outCents, 0));
  const avgIn = $derived(flow.length > 0 ? totalIn / flow.length : 0);
  const avgOut = $derived(flow.length > 0 ? totalOut / flow.length : 0);
  const last = $derived(flow[flow.length - 1]);
  const lastNet = $derived(last ? last.inCents - last.outCents : 0);
  const avgNet = $derived(avgIn - avgOut);
  const savingsRate = $derived(avgIn > 0 ? (1 - avgOut / avgIn) * 100 : 0);
</script>

<div class="topbar">
  <div>
    <h1>{t().nav.cashflow}</h1>
    <div class="sub">{flow.length} {t().common.months} · {t().common.monthAvg}</div>
  </div>
  {#if activeTab === 'timeseries'}
    <div class="seg">
      {#each ['1M', '3M', '6M', '12M', 'all'] as r (r)}
        <button class:on={range === r} onclick={() => (range = r as RangeId)}>
          {r === 'all' ? t().common.all : r}
        </button>
      {/each}
    </div>
  {/if}
</div>

<div class="tabs">
  <button class="tab" class:on={activeTab === 'timeseries'} onclick={() => (activeTab = 'timeseries')}>
    {t().cashflow.tabTimeseries}
  </button>
  <button class="tab" class:on={activeTab === 'composition'} onclick={() => (activeTab = 'composition')}>
    {t().cashflow.tabComposition}
  </button>
</div>

{#if activeTab === 'timeseries'}
<div class="kpi-grid">
  <KPI
    label={`${t().common.income} · Ø`}
    value={fmtEur(avgIn, { hide: settings.hide, decimals: eurDecimals() })}
  />
  <KPI
    label={`${t().common.expenses} · Ø`}
    value={fmtEur(avgOut, { hide: settings.hide, decimals: eurDecimals() })}
  />
  <KPI
    label={`${t().common.net} · ${t().common.thisMonth}`}
    value={fmtEur(lastNet, { hide: settings.hide, signed: true, decimals: eurDecimals() })}
    delta={(lastNet - avgNet) / 100}
  />
  <KPI
    label={`${t().common.savingsRate} · Ø`}
    value={`${savingsRate.toFixed(0)}%`}
  />
</div>

<div class="grid-12">
  <div class="card col-12 card-pad-lg">
    <div class="card-h">
      <div>
        <h3>{t().common.incomeVsExp}</h3>
        <div class="legend">
          <span><span class="dot pos"></span> {t().common.income}</span>
          <span class="sep"></span>
          <span><span class="dot neg"></span> {t().common.expenses}</span>
        </div>
      </div>
    </div>
    {#if loading && flow.length === 0}
      <div class="empty">…</div>
    {:else if flow.length === 0}
      <div class="empty">{t().common.spent}: 0 €</div>
    {:else}
      <CashflowChart data={chartData} height={240} hide={settings.hide} onPointClick={navigateToMonthTx} />
    {/if}
  </div>

  <div class="card col-6 card-pad-lg">
    <div class="card-h"><h3>{t().common.income}</h3></div>
    <div class="empty">
      <a href="/transactions">{t().nav.transactions}</a> · {t().common.income}
    </div>
    <div class="total">
      <span class="muted">Ø {t().common.month}</span>
      <span class="num bold">{fmtEur(avgIn, { hide: settings.hide, decimals: eurDecimals() })}</span>
    </div>
    <div class="total">
      <span class="muted">{t().common.thisMonth}</span>
      <span class="num bold">{fmtEur(last ? last.inCents : 0, { hide: settings.hide, decimals: eurDecimals() })}</span>
    </div>
  </div>

  <div class="card col-6 card-pad-lg">
    <div class="card-h"><h3>{t().common.expenses} · {t().common.perCat}</h3></div>
    <div class="empty">
      <a href="/reports">{t().nav.reports}</a> · {t().common.breakdown}
    </div>
    <div class="total">
      <span class="muted">Ø {t().common.month}</span>
      <span class="num bold">{fmtEur(avgOut, { hide: settings.hide, decimals: eurDecimals() })}</span>
    </div>
    <div class="total">
      <span class="muted">{t().common.thisMonth}</span>
      <span class="num bold">{fmtEur(last ? last.outCents : 0, { hide: settings.hide, decimals: eurDecimals() })}</span>
    </div>
  </div>
</div>
{/if}

{#if activeTab === 'composition'}
  <div class="comp-controls">
    <label class="period-kind">
      <span>{t().cashflow.periodRange} / {t().cashflow.periodMonth}</span>
      <select bind:value={compPeriodKind}>
        <option value="range">{t().cashflow.periodRange}</option>
        <option value="month">{t().cashflow.periodMonth}</option>
      </select>
    </label>
    {#if compPeriodKind === 'range'}
      <div class="seg">
        {#each ['1M', '3M', '6M', '12M', 'all'] as r (r)}
          <button class:on={compRange === r} onclick={() => (compRange = r as RangeId)}>
            {r === 'all' ? t().common.all : r}
          </button>
        {/each}
      </div>
    {:else}
      <div class="seg">
        <button onclick={() => compStepMonth(-1)}>◀</button>
        <button>
          {monthLabel(compYear, compMonth)}
        </button>
        <button onclick={() => compStepMonth(1)}>▶</button>
      </div>
    {/if}
  </div>
  <div class="card card-pad-lg">
    {#if compLoading && breakdown.length === 0}
      <div class="empty">…</div>
    {:else}
      <SankeyChart {breakdown} {categories} from={breakdownFrom} to={breakdownTo} />
    {/if}
  </div>
{/if}

<style>
  .legend {
    font-size: 11px;
    color: var(--text-faint);
    margin-top: 3px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .dot {
    display: inline-block;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    margin-right: 5px;
    vertical-align: -1px;
  }
  .dot.pos {
    background: var(--positive);
  }
  .dot.neg {
    background: var(--negative);
  }
  .sep {
    width: 1px;
    align-self: stretch;
  }
  .empty {
    padding: 14px 0;
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .empty a {
    color: var(--accent);
  }
  .total {
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
  }
  .muted {
    color: var(--text-muted);
  }
  .bold {
    font-weight: 600;
  }

  .tabs {
    display: flex;
    gap: 0;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }
  .tab {
    background: transparent;
    border: 0;
    padding: 10px 18px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    font: inherit;
  }
  .tab.on {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

  .comp-controls {
    display: flex;
    gap: 16px;
    align-items: end;
    margin-bottom: 14px;
  }
  .period-kind {
    display: grid;
    gap: 4px;
    font-size: 11px;
    color: var(--text-muted);
  }
  .period-kind select {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
    font: inherit;
    color: var(--text);
  }

  @media (max-width: 599px) {
    /* comp-controls wrap on phone */
    .comp-controls { flex-wrap: wrap; gap: 10px; }
  }
</style>
