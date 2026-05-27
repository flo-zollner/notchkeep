<script lang="ts">
  import CashflowChart from '$lib/components/CashflowChart.svelte';
  import Donut from '$lib/components/Donut.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';
  import {
    api,
    fmtEur,
    type Category,
    type CategorySpending,
    type MonthlyFlow,
  } from '$lib/api';

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

  let tab = $state<'breakdown' | 'trend' | 'compare'>('breakdown');
  let categories = $state<Category[]>([]);
  let thisMonth = $state<CategorySpending[]>([]);
  let prevMonth = $state<CategorySpending[]>([]);
  let flow = $state<MonthlyFlow[]>([]);
  let loading = $state(true);

  $effect(() => {
    loading = true;
    Promise.all([
      api.listCategories(),
      api.categoryBreakdown(thisFrom, thisTo),
      api.categoryBreakdown(prevFrom, prevTo),
      api.monthlyCashflow(curYear, curMonth, 6),
    ])
      .then(([cs, ts, ps, f]) => {
        categories = cs;
        thisMonth = ts;
        prevMonth = ps;
        flow = f;
      })
      .finally(() => {
        loading = false;
      });
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
    const ids = new Set<number>([
      ...thisMonth.map((s) => s.categoryId),
      ...prevMonth.map((s) => s.categoryId),
    ]);
    const nowMap = new Map(thisMonth.map((s) => [s.categoryId, s.spentCents]));
    const prevMap = new Map(prevMonth.map((s) => [s.categoryId, s.spentCents]));
    return [...ids]
      .map((id, i): CompareRow => {
        const cat = catMap.get(id);
        return {
          id,
          name: cat?.name ?? `#${id}`,
          icon: cat?.icon ?? 'tag',
          color: cat?.color ?? fallbackColors[i % fallbackColors.length],
          prevCents: prevMap.get(id) ?? 0,
          nowCents: nowMap.get(id) ?? 0,
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
    { id: 'compare' as const, label: `${t().common.vs} ${t().common.lastMonth}` },
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
    <div class="card card-pad-lg empty">
      {t().common.spent}: 0 €
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
          <div class="bud-bar" style="margin-bottom: 11px;">
            <div class="bud-fill" style:width={`${p}%`} style:background={b.color}></div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
{:else if tab === 'trend'}
  <div class="card card-pad-lg">
    {#if flow.length === 0}
      <div class="empty">{t().common.spent}: 0 €</div>
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
{:else if tab === 'compare'}
  <div class="card card-pad-lg">
    <div class="card-h">
      <h3>{t().common.thisMonth} {t().common.vs} {t().common.lastMonth}</h3>
    </div>
    {#if compareRows.length === 0}
      <div class="empty">{t().common.spent}: 0 €</div>
    {:else}
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
    gap: 10px;
    margin-bottom: 6px;
  }
  .cat-ic {
    width: 26px;
    height: 26px;
    border-radius: 7px;
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
    width: 36px;
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
    padding: 10px 4px;
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
    grid-template-columns: 40px 1fr 1fr 1fr 110px;
    align-items: center;
    gap: 14px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }
  .compare-row:last-child {
    border-bottom: none;
  }
  .empty {
    text-align: center;
    color: var(--text-faint);
    font-size: 13px;
    padding: 24px;
  }

  @media (max-width: 599px) {
    /* compare-row: collapse 5-col grid to compact 2-col layout */
    .compare-row {
      grid-template-columns: 40px 1fr auto;
      grid-template-rows: auto auto;
      gap: 8px;
    }
    .compare-row > :nth-child(4) { grid-column: 2; }
    .compare-row > :nth-child(5) { grid-column: 2 / 4; }
  }
</style>
