<script lang="ts">
  import { api, type CostBasisPointDaily } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import DateField from './DateField.svelte';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';

  type Range = '1d' | '1w' | '12m' | '3y' | '5y' | 'all' | 'manual';
  let range = $state<Range>('12m');

  type Point = { costBasisCents: number; marketValueCents: number };
  let data = $state<Point[]>([]);
  let loading = $state(true);

  const now = new Date();
  const endYear = now.getFullYear();
  const endMonth = now.getMonth() + 1;
  const todayStr = now.toISOString().slice(0, 10);

  // Manual range: default to last 30 days
  let manualFrom = $state<string>(new Date(Date.now() - 30 * 86400000).toISOString().slice(0, 10));
  let manualTo = $state<string>(todayStr);

  async function load() {
    loading = true;
    try {
      if (range === '1d') {
        data = await api.costBasisHistoryDaily(todayStr, 2);
      } else if (range === '1w') {
        data = await api.costBasisHistoryDaily(todayStr, 7);
      } else if (range === 'manual') {
        // Auto-pick: < 60 days → daily, else monthly
        const fromD = new Date(manualFrom);
        const toD = new Date(manualTo);
        if (fromD > toD) { data = []; return; }
        const dayDiff = Math.round((toD.getTime() - fromD.getTime()) / 86400000) + 1;
        if (dayDiff <= 60) {
          data = await api.costBasisHistoryDaily(manualTo, Math.min(dayDiff, 366));
        } else {
          // Monthly: compute months from start_year/month to to_year/month
          const startY = fromD.getFullYear();
          const startM = fromD.getMonth() + 1;
          const toY = toD.getFullYear();
          const toM = toD.getMonth() + 1;
          const months = Math.max(1, (toY - startY) * 12 + (toM - startM) + 1);
          data = await api.costBasisHistory(toY, toM, Math.min(months, 120));
        }
      } else {
        const months = { '12m': 12, '3y': 36, '5y': 60, 'all': 120 }[range as '12m' | '3y' | '5y' | 'all'];
        data = await api.costBasisHistory(endYear, endMonth, months);
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void range; void manualFrom; void manualTo;
    load();
  });

  const W = 720;
  const H = 200;
  const PAD = 24;

  const maxV = $derived.by(() => {
    if (data.length === 0) return 1;
    return Math.max(
      ...data.map((d) => d.marketValueCents),
      ...data.map((d) => d.costBasisCents),
      1
    );
  });

  function pointsFor(field: 'costBasisCents' | 'marketValueCents') {
    if (data.length === 0) return [];
    const stepX = (W - 2 * PAD) / Math.max(data.length - 1, 1);
    return data.map((d, i) => ({
      x: PAD + i * stepX,
      y: H - PAD - (d[field] / maxV) * (H - 2 * PAD),
      v: d[field],
    }));
  }

  const costPoints = $derived(pointsFor('costBasisCents'));
  const marketPoints = $derived(pointsFor('marketValueCents'));

  function pathFrom(ps: Array<{x: number, y: number}>): string {
    if (ps.length === 0) return '';
    return ps.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');
  }

  const marketPath = $derived(pathFrom(marketPoints));
  const costPath = $derived(pathFrom(costPoints));

  const areaPath = $derived.by(() => {
    if (marketPoints.length === 0) return '';
    const top = marketPoints.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');
    const last = marketPoints[marketPoints.length - 1];
    const first = marketPoints[0];
    return `${top} L ${last.x} ${H - PAD} L ${first.x} ${H - PAD} Z`;
  });

  const latestMarket = $derived(data.length > 0 ? data[data.length - 1].marketValueCents : 0);
  const tp = $derived(t().portfolio);

  // Hover state
  interface HoverState {
    idx: number;
    x: number;
    marketY: number;
    costY: number;
    marketV: number;
    costV: number;
  }
  let hover = $state<HoverState | null>(null);
  let svgEl = $state<SVGSVGElement | undefined>(undefined);

  function handlePointerMove(ev: PointerEvent) {
    if (data.length < 2 || !svgEl) { hover = null; return; }
    const rect = svgEl.getBoundingClientRect();
    const xPx = ev.clientX - rect.left;
    const xViewport = (xPx / rect.width) * W;
    const rel = Math.max(0, Math.min(1, (xViewport - PAD) / (W - 2 * PAD)));
    const idx = Math.round(rel * (data.length - 1));
    if (idx < 0 || idx >= data.length) { hover = null; return; }
    hover = {
      idx,
      x: marketPoints[idx].x,
      marketY: marketPoints[idx].y,
      costY: costPoints[idx].y,
      marketV: data[idx].marketValueCents,
      costV: data[idx].costBasisCents,
    };
  }
  function handlePointerLeave() { hover = null; }

  const tooltipLeft = $derived(hover ? (hover.x / W) * 100 : 0);
  const tooltipTop = $derived(hover ? (hover.marketY / H) * 100 : 0);
</script>

<div class="head">
  <h3>{tp.performanceTitle}</h3>
  <div class="seg">
    <button class:on={range === '1d'} onclick={() => (range = '1d')}>{tp.rangeDay}</button>
    <button class:on={range === '1w'} onclick={() => (range = '1w')}>{tp.rangeWeek}</button>
    <button class:on={range === '12m'} onclick={() => (range = '12m')}>12M</button>
    <button class:on={range === '3y'}  onclick={() => (range = '3y')}>3J</button>
    <button class:on={range === '5y'}  onclick={() => (range = '5y')}>5J</button>
    <button class:on={range === 'all'} onclick={() => (range = 'all')}>all</button>
    <button class:on={range === 'manual'} onclick={() => (range = 'manual')}>{tp.rangeManual}</button>
  </div>
</div>

{#if range === 'manual'}
  <div class="manual-controls">
    <label>{tp.rangeFrom} <DateField bind:value={manualFrom} /></label>
    <label>{tp.rangeTo} <DateField bind:value={manualTo} /></label>
  </div>
{/if}

{#if loading && data.length === 0}
  <div class="empty">…</div>
{:else if data.length === 0}
  <div class="empty">{tp.emptyPositions}</div>
{:else}
  <div class="chart-wrap">
    <svg
      bind:this={svgEl}
      viewBox="0 0 {W} {H}"
      class="chart"
      preserveAspectRatio="xMidYMid meet"
      onpointermove={handlePointerMove}
      onpointerleave={handlePointerLeave}
    >
      <path d={areaPath} fill="var(--accent)" opacity="0.08" />
      <path d={costPath} stroke="var(--text-muted)" stroke-width="1.2" fill="none" stroke-dasharray="4 3" />
      <path d={marketPath} stroke="var(--accent)" stroke-width="1.5" fill="none" />
      {#each marketPoints as p, i (i)}
        {#if i === marketPoints.length - 1}
          <circle cx={p.x} cy={p.y} r="3" fill="var(--accent)" />
        {/if}
      {/each}
      {#if hover}
        <line
          x1={hover.x} x2={hover.x}
          y1={PAD} y2={H - PAD}
          stroke="var(--accent)" stroke-width="1" stroke-opacity="0.6"
          stroke-dasharray="3 3" pointer-events="none"
        />
        <circle
          cx={hover.x} cy={hover.marketY} r="4"
          fill="var(--accent)" stroke="var(--surface)" stroke-width="2"
          pointer-events="none"
        />
        <circle
          cx={hover.x} cy={hover.costY} r="3"
          fill="var(--text-muted)" stroke="var(--surface)" stroke-width="1.5"
          pointer-events="none"
        />
      {/if}
    </svg>
    {#if hover}
      <div class="chart-tooltip" style="left: {tooltipLeft}%; top: {tooltipTop}%;">
        <span class="value">{fmtEur(hover.marketV, { hide: settings.hide, decimals: eurDecimals() })}</span>
        <span class="label">Einstandswert: {fmtEur(hover.costV, { hide: settings.hide, decimals: eurDecimals() })}</span>
      </div>
    {/if}
  </div>
  <div class="foot">
    <span class="num">{fmtEur(latestMarket, { hide: settings.hide, decimals: eurDecimals() })}</span>
    <span class="muted">{tp.kpiMarketValue}</span>
  </div>
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }
  .head h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
  }
  .seg {
    display: flex;
    gap: 0;
  }
  .seg button {
    background: transparent;
    border: 1px solid var(--border);
    border-right: 0;
    color: var(--text-muted);
    font-size: 11px;
    padding: 4px 10px;
    cursor: pointer;
    font: inherit;
  }
  .seg button:first-child { border-top-left-radius: 6px; border-bottom-left-radius: 6px; }
  .seg button:last-child  { border-right: 1px solid var(--border); border-top-right-radius: 6px; border-bottom-right-radius: 6px; }
  .seg button.on {
    color: var(--text);
    background: var(--surface-2);
  }
  .empty {
    padding: 24px;
    text-align: center;
    color: var(--text-faint);
    font-size: 13px;
  }
  .chart-wrap {
    position: relative;
  }
  .chart {
    width: 100%;
    height: auto;
    max-height: 200px;
    display: block;
    cursor: crosshair;
  }
  .foot {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-top: 4px;
  }
  .foot .num {
    font-size: 18px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .muted {
    color: var(--text-muted);
    font-size: 12px;
  }
  .manual-controls {
    display: flex;
    gap: 12px;
    margin-bottom: 8px;
    font-size: 12px;
    color: var(--text-muted);
  }
  .manual-controls label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .manual-controls input {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
    font: inherit;
    color: var(--text);
  }
  .chart-tooltip {
    position: absolute;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 12px;
    color: var(--text);
    pointer-events: none;
    white-space: nowrap;
    z-index: 10;
    box-shadow: 0 2px 8px rgba(0,0,0,0.12);
    transform: translate(-50%, calc(-100% - 10px));
  }
  .chart-tooltip .label { color: var(--text-faint); font-size: 11px; display: block; }
  .chart-tooltip .value { font-family: var(--font-mono); font-weight: 600; display: block; }
</style>
