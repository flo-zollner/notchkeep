<script lang="ts">
  import CashflowChart from '$lib/components/CashflowChart.svelte';
  import Donut from '$lib/components/Donut.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import SankeyChart from '$lib/components/SankeyChart.svelte';
  import DateField from '$lib/components/DateField.svelte';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';
  import {
    api,
    type CashflowSlice,
    type Category,
    type CategorySpending,
    type MonthlyFlow,
  } from '$lib/api';
  import { fmtEur } from '$lib/format';

  type RangeId = '1M' | '3M' | '6M' | '12M' | 'all';
  const RANGE_MONTHS: Record<RangeId, number> = {
    '1M': 1,
    '3M': 3,
    '6M': 6,
    '12M': 12,
    all: 24,
  };

  const now = new Date();
  const curYear = now.getFullYear();
  const curMonth = now.getMonth() + 1;

  function stepMonth(y: number, m: number, delta: number): { y: number; m: number } {
    const total = y * 12 + (m - 1) + delta;
    return { y: Math.floor(total / 12), m: (((total % 12) + 12) % 12) + 1 };
  }
  function monthStart(y: number, m: number): string {
    return `${y.toString().padStart(4, '0')}-${m.toString().padStart(2, '0')}-01`;
  }

  const thisFrom = monthStart(curYear, curMonth);
  const next = stepMonth(curYear, curMonth, 1);
  const thisTo = monthStart(next.y, next.m);
  const prev = stepMonth(curYear, curMonth, -1);
  const prevFrom = monthStart(prev.y, prev.m);
  const prevTo = thisFrom;

  let tab = $state<'breakdown' | 'trend' | 'compare' | 'composition'>('breakdown');
  let categories = $state<Category[]>([]);
  let thisMonth = $state<CategorySpending[]>([]);
  let flow = $state<MonthlyFlow[]>([]);
  let loading = $state(true);

  // Compare tab: two arbitrary periods against each other.
  type PeriodMode = 'range' | 'month' | 'custom';

  // Defaults: A = last month, B = this month (status quo of the old report)
  const threeAgo = stepMonth(curYear, curMonth, -3);
  const defaultCustomFrom = monthStart(threeAgo.y, threeAgo.m);
  const defaultCustomTo = monthStart(curYear, curMonth);

  let cmpAMode = $state<PeriodMode>('month');
  let cmpARange = $state<RangeId>('3M');
  let cmpAAvg = $state(false);
  let cmpAYear = $state(prev.y);
  let cmpAMonth = $state(prev.m);
  let cmpACustomFrom = $state<string>(defaultCustomFrom);
  let cmpACustomTo = $state<string>(defaultCustomTo);

  let cmpBMode = $state<PeriodMode>('month');
  let cmpBRange = $state<RangeId>('3M');
  let cmpBAvg = $state(false);
  let cmpBYear = $state(curYear);
  let cmpBMonth = $state(curMonth);
  let cmpBCustomFrom = $state<string>(defaultCustomFrom);
  let cmpBCustomTo = $state<string>(defaultCustomTo);

  let cmpAData = $state<CategorySpending[]>([]);
  let cmpBData = $state<CategorySpending[]>([]);
  let cmpADivisor = $state(1);
  let cmpBDivisor = $state(1);
  let cmpALabel = $state('');
  let cmpBLabel = $state('');
  let cmpLoading = $state(false);

  function rangeLabel(r: RangeId): string {
    return r === 'all' ? t().common.all : r;
  }

  /** Difference in days between two ISO dates (half-open interval). */
  function dayDiff(fromIso: string, toIso: string): number {
    const a = Date.parse(fromIso);
    const b = Date.parse(toIso);
    if (Number.isNaN(a) || Number.isNaN(b)) return 0;
    return Math.max(0, Math.round((b - a) / (1000 * 60 * 60 * 24)));
  }

  function shortDate(iso: string): string {
    // ISO 'YYYY-MM-DD' → 'DD.MM.YY' for a compact range label.
    const m = iso.match(/^(\d{4})-(\d{2})-(\d{2})/);
    if (!m) return iso;
    return `${m[3]}.${m[2]}.${m[1].slice(2)}`;
  }

  /** Returns from/to (half-open), a divisor for normalization (for averages), and a UI label. */
  function periodToQuery(
    mode: PeriodMode,
    range: RangeId,
    avg: boolean,
    year: number,
    month: number,
    customFrom: string,
    customTo: string,
  ): { from: string; to: string; divisor: number; label: string } {
    if (mode === 'month') {
      const from = monthStart(year, month);
      const nx = stepMonth(year, month, 1);
      return { from, to: monthStart(nx.y, nx.m), divisor: 1, label: monthLabel(year, month) };
    }
    if (mode === 'custom') {
      // 30.4375 ≈ average days per month (Gregorian). For averages we compute the month equivalent.
      const days = dayDiff(customFrom, customTo);
      const months = Math.max(1, days / 30.4375);
      const divisor = avg ? months : 1;
      const range = `${shortDate(customFrom)}–${shortDate(customTo)}`;
      const label = avg ? `Ø ${range}` : range;
      return { from: customFrom, to: customTo, divisor, label };
    }
    // 'range': last N months up to (exclusive) start of next month.
    const months = RANGE_MONTHS[range];
    const nx = stepMonth(curYear, curMonth, 1);
    const to = monthStart(nx.y, nx.m);
    const start = stepMonth(curYear, curMonth, -(months - 1));
    const from = monthStart(start.y, start.m);
    const divisor = avg ? months : 1;
    const label = avg ? `Ø ${rangeLabel(range)}` : rangeLabel(range);
    return { from, to, divisor, label };
  }

  // Composition tab (Sankey)
  type PeriodKind = 'range' | 'month';
  let periodKind = $state<PeriodKind>('range');
  let rangeId = $state<RangeId>('1M');
  let pickedYear = $state(curYear);
  let pickedMonth = $state(curMonth);
  let sankeyBreakdown = $state<CashflowSlice[]>([]);
  let sankeyFrom = $state<string>('');
  let sankeyTo = $state<string>('');
  let sankeyLoading = $state(false);

  function periodToDates(): { from: string; to: string } {
    if (periodKind === 'month') {
      const from = `${pickedYear}-${String(pickedMonth).padStart(2, '0')}-01`;
      const nextY = pickedMonth === 12 ? pickedYear + 1 : pickedYear;
      const nextM = pickedMonth === 12 ? 1 : pickedMonth + 1;
      const to = `${nextY}-${String(nextM).padStart(2, '0')}-01`;
      return { from, to };
    }
    const months = RANGE_MONTHS[rangeId];
    const nowD = new Date();
    const to = new Date(nowD.getFullYear(), nowD.getMonth() + 1, 1)
      .toISOString().slice(0, 10);
    const fromDate = new Date(nowD.getFullYear(), nowD.getMonth() - months + 1, 1);
    const from = fromDate.toISOString().slice(0, 10);
    return { from, to };
  }

  async function loadSankey() {
    const { from, to } = periodToDates();
    sankeyLoading = true;
    sankeyFrom = from;
    sankeyTo = to;
    try {
      sankeyBreakdown = await api.cashflowBreakdown(from, to);
    } finally {
      sankeyLoading = false;
    }
  }

  $effect(() => {
    if (tab !== 'composition') return;
    void periodKind;
    void rangeId;
    void pickedYear;
    void pickedMonth;
    void loadSankey();
  });

  function stepPickedMonth(delta: -1 | 1) {
    let y = pickedYear;
    let m = pickedMonth + delta;
    if (m < 1) { m = 12; y -= 1; }
    else if (m > 12) { m = 1; y += 1; }
    pickedYear = y;
    pickedMonth = m;
  }

  $effect(() => {
    loading = true;
    Promise.all([
      api.listCategories(),
      api.categoryBreakdown(thisFrom, thisTo),
      api.monthlyCashflow(curYear, curMonth, 6),
    ])
      .then(([cs, ts, f]) => {
        categories = cs;
        thisMonth = ts;
        flow = f;
      })
      .finally(() => {
        loading = false;
      });
  });

  async function loadCompare() {
    const qA = periodToQuery(cmpAMode, cmpARange, cmpAAvg, cmpAYear, cmpAMonth, cmpACustomFrom, cmpACustomTo);
    const qB = periodToQuery(cmpBMode, cmpBRange, cmpBAvg, cmpBYear, cmpBMonth, cmpBCustomFrom, cmpBCustomTo);
    cmpLoading = true;
    cmpALabel = qA.label;
    cmpBLabel = qB.label;
    cmpADivisor = qA.divisor;
    cmpBDivisor = qB.divisor;
    try {
      const [a, b] = await Promise.all([
        api.categoryBreakdown(qA.from, qA.to),
        api.categoryBreakdown(qB.from, qB.to),
      ]);
      cmpAData = a;
      cmpBData = b;
    } finally {
      cmpLoading = false;
    }
  }

  $effect(() => {
    if (tab !== 'compare') return;
    void cmpAMode;
    void cmpARange;
    void cmpAAvg;
    void cmpAYear;
    void cmpAMonth;
    void cmpACustomFrom;
    void cmpACustomTo;
    void cmpBMode;
    void cmpBRange;
    void cmpBAvg;
    void cmpBYear;
    void cmpBMonth;
    void cmpBCustomFrom;
    void cmpBCustomTo;
    void loadCompare();
  });

  function monthLabel(year: number, month: number): string {
    const labels = t().months;
    const yy = String(year % 100).padStart(2, '0');
    return `${labels[month - 1]} ${yy}`;
  }

  const catMap = $derived(new Map(categories.map((c) => [c.id, c])));

  const fallbackColors = ['var(--c1)', 'var(--c2)', 'var(--c3)', 'var(--c4)', 'var(--c5)', 'var(--c6)'];

  type BreakdownRow = {
    id: number;
    name: string;
    icon: string;
    color: string;
    spentCents: number;
  };

  const breakdown = $derived.by<BreakdownRow[]>(() =>
    thisMonth
      .map((s, i): BreakdownRow => {
        const cat = catMap.get(s.categoryId);
        return {
          id: s.categoryId,
          name: cat?.name ?? `#${s.categoryId}`,
          icon: cat?.icon ?? 'tag',
          color: cat?.color ?? fallbackColors[i % fallbackColors.length],
          spentCents: s.spentCents,
        };
      })
      .sort((a, b) => b.spentCents - a.spentCents),
  );

  const donutData = $derived(
    breakdown.map((b) => ({ name: b.name, v: b.spentCents / 100, color: b.color })),
  );

  const grandSpent = $derived(breakdown.reduce((s, b) => s + b.spentCents, 0));

  const chartData = $derived(
    flow.map((f) => ({
      m: monthLabel(f.year, f.month),
      in: f.inCents / 100,
      out: f.outCents / 100,
    })),
  );

  type CompareRow = {
    id: number;
    name: string;
    icon: string;
    color: string;
    prevCents: number;
    nowCents: number;
  };

  const compareRows = $derived.by<CompareRow[]>(() => {
    const aDiv = cmpADivisor || 1;
    const bDiv = cmpBDivisor || 1;
    const ids = new Set<number>([
      ...cmpAData.map((s) => s.categoryId),
      ...cmpBData.map((s) => s.categoryId),
    ]);
    const aMap = new Map(cmpAData.map((s) => [s.categoryId, Math.round(s.spentCents / aDiv)]));
    const bMap = new Map(cmpBData.map((s) => [s.categoryId, Math.round(s.spentCents / bDiv)]));
    return [...ids]
      .map((id, i): CompareRow => {
        const cat = catMap.get(id);
        return {
          id,
          name: cat?.name ?? `#${id}`,
          icon: cat?.icon ?? 'tag',
          color: cat?.color ?? fallbackColors[i % fallbackColors.length],
          prevCents: aMap.get(id) ?? 0,
          nowCents: bMap.get(id) ?? 0,
        };
      })
      .sort((a, b) => b.nowCents - a.nowCents);
  });
</script>

<div class="topbar">
  <div>
    <h1>{t().nav.reports}</h1>
    <div class="sub">{monthLabel(curYear, curMonth)}</div>
  </div>
  <button class="btn" disabled><Icon name="download" size={13} /> CSV</button>
</div>

<div class="tabs">
  {#each [
    { id: 'breakdown' as const, label: t().common.breakdown },
    { id: 'trend' as const, label: t().common.incomeVsExp },
    { id: 'compare' as const, label: t().common.compare },
    { id: 'composition' as const, label: t().cashflow.tabComposition },
  ] as tabDef (tabDef.id)}
    <button class={`tab ${tab === tabDef.id ? 'on' : ''}`} onclick={() => (tab = tabDef.id)}>
      {tabDef.label}
    </button>
  {/each}
</div>

{#if loading && breakdown.length === 0 && flow.length === 0}
  <div class="card card-pad-lg empty">…</div>
{:else if tab === 'breakdown'}
  {#if breakdown.length === 0}
    <div class="card card-pad-lg">
      <EmptyState icon="reports" title="Noch keine Daten" description="Für den gewählten Zeitraum gibt es noch keine Buchungen." />
    </div>
  {:else}
    <div class="grid-12">
      <div class="card col-5 card-pad-lg">
        <div class="card-h"><h3>{t().common.perCat}</h3></div>
        <Donut data={donutData} size={200} hide={settings.hide} />
      </div>
      <div class="card col-7 card-pad-lg">
        <div class="card-h"><h3>{t().common.topCats}</h3></div>
        {#each breakdown as b (b.id)}
          {@const p = grandSpent > 0 ? (b.spentCents / grandSpent) * 100 : 0}
          <div class="cat-row">
            <span class="cat-ic" style:color={b.color}>
              <Icon name={b.icon} size={13} />
            </span>
            <span class="cat-name">{b.name}</span>
            <span class="num cat-val">{fmtEur(b.spentCents, { hide: settings.hide, decimals: eurDecimals() })}</span>
            <span class="num cat-pct">{p.toFixed(0)}%</span>
          </div>
          <div class="bud-bar bud-bar-gap">
            <div class="bud-fill" style:width={`${p}%`} style:background={b.color}></div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
{:else if tab === 'trend'}
  <div class="card card-pad-lg">
    {#if flow.length === 0}
      <EmptyState icon="reports" title="Noch keine Daten" description="Für den gewählten Zeitraum gibt es noch keine Buchungen." />
    {:else}
      <CashflowChart data={chartData} height={300} hide={settings.hide} />
      <table>
        <thead>
          <tr>
            <th>{t().common.month}</th>
            <th class="right">{t().common.income}</th>
            <th class="right">{t().common.expenses}</th>
            <th class="right">{t().common.net}</th>
            <th class="right">{t().common.savingsRate}</th>
          </tr>
        </thead>
        <tbody>
          {#each flow as f (`${f.year}-${f.month}`)}
            {@const net = f.inCents - f.outCents}
            {@const sr = f.inCents > 0 ? (net / f.inCents) * 100 : 0}
            <tr>
              <td>{monthLabel(f.year, f.month)}</td>
              <td class="num right">{fmtEur(f.inCents, { hide: settings.hide, decimals: eurDecimals() })}</td>
              <td class="num right muted">{fmtEur(f.outCents, { hide: settings.hide, decimals: eurDecimals() })}</td>
              <td class="num right bold">{fmtEur(net, { hide: settings.hide, signed: true, decimals: eurDecimals() })}</td>
              <td class="num right" class:pos={sr >= 0} class:neg={sr < 0}>{sr.toFixed(0)}%</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
{:else if tab === 'composition'}
  <div class="comp-controls">
    <label class="period-kind">
      <span>{t().cashflow.periodRange} / {t().cashflow.periodMonth}</span>
      <select bind:value={periodKind}>
        <option value="range">{t().cashflow.periodRange}</option>
        <option value="month">{t().cashflow.periodMonth}</option>
      </select>
    </label>
    {#if periodKind === 'range'}
      <div class="seg">
        {#each ['1M', '3M', '6M', '12M', 'all'] as r (r)}
          <button class:on={rangeId === r} onclick={() => (rangeId = r as RangeId)}>
            {r === 'all' ? t().common.all : r}
          </button>
        {/each}
      </div>
    {:else}
      <div class="seg">
        <button aria-label="Vorheriger Monat" onclick={() => stepPickedMonth(-1)}>◀</button>
        <button>{monthLabel(pickedYear, pickedMonth)}</button>
        <button aria-label="Nächster Monat" onclick={() => stepPickedMonth(1)}>▶</button>
      </div>
    {/if}
  </div>
  <div class="card card-pad-lg">
    {#if sankeyLoading && sankeyBreakdown.length === 0}
      <div class="empty">…</div>
    {:else}
      <SankeyChart breakdown={sankeyBreakdown} {categories} from={sankeyFrom} to={sankeyTo} />
    {/if}
  </div>
{:else if tab === 'compare'}
  <div class="cmp-controls">
    <div class="cmp-picker">
      <div class="cmp-picker-h">A</div>
      <select bind:value={cmpAMode}>
        <option value="range">{t().cashflow.periodRange}</option>
        <option value="month">{t().cashflow.periodMonth}</option>
        <option value="custom">{t().common.from}–{t().common.to}</option>
      </select>
      {#if cmpAMode === 'range'}
        <div class="seg">
          {#each ['1M', '3M', '6M', '12M', 'all'] as r (r)}
            <button class:on={cmpARange === r} onclick={() => (cmpARange = r as RangeId)}>{rangeLabel(r as RangeId)}</button>
          {/each}
        </div>
        <div class="seg agg">
          <button class:on={!cmpAAvg} onclick={() => (cmpAAvg = false)} aria-label="Summe">Σ</button>
          <button class:on={cmpAAvg} onclick={() => (cmpAAvg = true)} aria-label="Durchschnitt pro Monat">Ø</button>
        </div>
      {:else if cmpAMode === 'month'}
        <div class="seg">
          <button aria-label="Vorheriger Monat" onclick={() => { const s = stepMonth(cmpAYear, cmpAMonth, -1); cmpAYear = s.y; cmpAMonth = s.m; }}>◀</button>
          <button>{monthLabel(cmpAYear, cmpAMonth)}</button>
          <button aria-label="Nächster Monat" onclick={() => { const s = stepMonth(cmpAYear, cmpAMonth, 1); cmpAYear = s.y; cmpAMonth = s.m; }}>▶</button>
        </div>
      {:else}
        <div class="date-range">
          <DateField bind:value={cmpACustomFrom} max={cmpACustomTo} />
          <span class="dash">–</span>
          <DateField bind:value={cmpACustomTo} min={cmpACustomFrom} />
        </div>
        <div class="seg agg">
          <button class:on={!cmpAAvg} onclick={() => (cmpAAvg = false)} aria-label="Summe">Σ</button>
          <button class:on={cmpAAvg} onclick={() => (cmpAAvg = true)} aria-label="Durchschnitt pro Monat">Ø</button>
        </div>
      {/if}
    </div>
    <div class="cmp-picker">
      <div class="cmp-picker-h">B</div>
      <select bind:value={cmpBMode}>
        <option value="range">{t().cashflow.periodRange}</option>
        <option value="month">{t().cashflow.periodMonth}</option>
        <option value="custom">{t().common.from}–{t().common.to}</option>
      </select>
      {#if cmpBMode === 'range'}
        <div class="seg">
          {#each ['1M', '3M', '6M', '12M', 'all'] as r (r)}
            <button class:on={cmpBRange === r} onclick={() => (cmpBRange = r as RangeId)}>{rangeLabel(r as RangeId)}</button>
          {/each}
        </div>
        <div class="seg agg">
          <button class:on={!cmpBAvg} onclick={() => (cmpBAvg = false)} aria-label="Summe">Σ</button>
          <button class:on={cmpBAvg} onclick={() => (cmpBAvg = true)} aria-label="Durchschnitt pro Monat">Ø</button>
        </div>
      {:else if cmpBMode === 'month'}
        <div class="seg">
          <button aria-label="Vorheriger Monat" onclick={() => { const s = stepMonth(cmpBYear, cmpBMonth, -1); cmpBYear = s.y; cmpBMonth = s.m; }}>◀</button>
          <button>{monthLabel(cmpBYear, cmpBMonth)}</button>
          <button aria-label="Nächster Monat" onclick={() => { const s = stepMonth(cmpBYear, cmpBMonth, 1); cmpBYear = s.y; cmpBMonth = s.m; }}>▶</button>
        </div>
      {:else}
        <div class="date-range">
          <DateField bind:value={cmpBCustomFrom} max={cmpBCustomTo} />
          <span class="dash">–</span>
          <DateField bind:value={cmpBCustomTo} min={cmpBCustomFrom} />
        </div>
        <div class="seg agg">
          <button class:on={!cmpBAvg} onclick={() => (cmpBAvg = false)} aria-label="Summe">Σ</button>
          <button class:on={cmpBAvg} onclick={() => (cmpBAvg = true)} aria-label="Durchschnitt pro Monat">Ø</button>
        </div>
      {/if}
    </div>
  </div>
  <div class="card card-pad-lg">
    <div class="card-h">
      <h3>{cmpBLabel || '…'} {t().common.vs} {cmpALabel || '…'}</h3>
    </div>
    {#if cmpLoading && compareRows.length === 0}
      <div class="empty">…</div>
    {:else if compareRows.length === 0}
      <EmptyState icon="reports" title="Noch keine Daten" description="Für den gewählten Zeitraum gibt es noch keine Buchungen." />
    {:else}
      <div class="compare-row compare-head">
        <span></span>
        <span></span>
        <span class="num right muted">{cmpALabel}</span>
        <span class="num right muted">{cmpBLabel}</span>
        <span class="num right muted">Δ</span>
      </div>
      {#each compareRows as r (r.id)}
        {@const diff = r.nowCents - r.prevCents}
        <div class="compare-row">
          <span class="cat-ic" style:color={r.color}>
            <Icon name={r.icon} size={13} />
          </span>
          <span class="cat-name">{r.name}</span>
          <span class="num right muted">{fmtEur(r.prevCents, { hide: settings.hide, decimals: eurDecimals() })}</span>
          <span class="num right bold">{fmtEur(r.nowCents, { hide: settings.hide, decimals: eurDecimals() })}</span>
          <span class={`num pill ${diff > 0 ? 'down' : 'up'}`}>
            {fmtEur(diff, { hide: settings.hide, signed: true, decimals: eurDecimals() })}
          </span>
        </div>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .cat-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .cat-ic {
    width: 24px;
    height: 24px;
    border-radius: var(--r-sm);
    background: var(--surface-2);
    display: grid;
    place-items: center;
  }
  .cat-name {
    flex: 1;
    font-size: 13.5px;
    font-weight: 500;
  }
  .cat-val {
    font-size: 13px;
    font-weight: 500;
  }
  .cat-pct {
    font-size: 11px;
    color: var(--text-faint);
    width: 32px;
    text-align: right;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    margin-top: 24px;
  }
  thead tr {
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
    font-weight: 500;
  }
  th,
  td {
    padding: 8px 4px;
    text-align: left;
  }
  th.right,
  td.right {
    text-align: right;
  }
  tbody tr {
    border-bottom: 1px solid var(--border);
  }
  .muted {
    color: var(--text-muted);
  }
  .bold {
    font-weight: 600;
  }
  .pos {
    color: var(--positive);
  }
  .neg {
    color: var(--negative);
  }
  .compare-row {
    display: grid;
    grid-template-columns: 32px 1fr 1fr 1fr auto;
    align-items: center;
    gap: 14px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .compare-row:last-child {
    border-bottom: none;
  }
  .bud-bar-gap {
    margin-bottom: 12px;
  }

  .comp-controls {
    display: flex;
    gap: 16px;
    align-items: end;
    margin-bottom: 14px;
  }

  .cmp-controls {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    margin-bottom: 14px;
  }
  .cmp-picker {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
  }
  .cmp-picker-h {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    width: 16px;
  }
  .cmp-picker select {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 4px 8px;
    font: inherit;
    color: var(--text);
  }
  .compare-head {
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 500;
    padding: 8px 0 8px;
  }
  .date-range {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .date-range input[type='date'] {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 4px 8px;
    font: inherit;
    font-size: 12px;
    color: var(--text);
    color-scheme: dark light;
  }
  .date-range .dash {
    color: var(--text-muted);
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
    border-radius: var(--r-sm);
    padding: 4px 8px;
    font: inherit;
    color: var(--text);
  }

  @media (max-width: 599px) {
    .comp-controls { flex-wrap: wrap; gap: 10px; }
    .cmp-controls { grid-template-columns: 1fr; }
  }
  @media (max-width: 599px) {
    /* compare-row: collapse 5-col grid to compact 2-col layout */
    .compare-row {
      grid-template-columns: 32px 1fr auto;
      grid-template-rows: auto auto;
      gap: 8px;
    }
    .compare-row > :nth-child(4) { grid-column: 2; }
    .compare-row > :nth-child(5) { grid-column: 2 / 4; }
  }
</style>
