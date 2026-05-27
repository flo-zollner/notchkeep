<script lang="ts">
  import { api, type PriceRow } from '$lib/api';
  import DateField from './DateField.svelte';
  import { settings, t } from '$lib/settings.svelte';

  interface Props {
    securityId: number;
    reloadKey?: number;
  }
  let { securityId, reloadKey = 0 }: Props = $props();

  type Range = '1d' | '1w' | '1m' | '3m' | '1y' | '5y' | 'all' | 'manual';
  let range = $state<Range>('1y');
  let allPrices = $state<PriceRow[]>([]);
  let loading = $state(true);

  const todayStr = new Date().toISOString().slice(0, 10);
  let manualFrom = $state<string>(new Date(Date.now() - 30 * 86400000).toISOString().slice(0, 10));
  let manualTo = $state<string>(todayStr);

  async function load() {
    loading = true;
    try {
      allPrices = await api.getPriceHistory(securityId);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void securityId;
    void reloadKey;
    load();
  });

  function cutoffDate(r: Range): string | null {
    if (r === 'all') return null;
    if (r === 'manual') return manualFrom;
    const now = new Date();
    if (r === '1d') {
      now.setDate(now.getDate() - 1);
      return now.toISOString().slice(0, 10);
    }
    if (r === '1w') {
      now.setDate(now.getDate() - 7);
      return now.toISOString().slice(0, 10);
    }
    const monthsAgo = { '1m': 1, '3m': 3, '1y': 12, '5y': 60 }[r as '1m' | '3m' | '1y' | '5y'];
    now.setMonth(now.getMonth() - monthsAgo);
    return now.toISOString().slice(0, 10);
  }

  const filteredPrices = $derived.by(() => {
    if (range === 'all') return allPrices;
    if (range === 'manual') {
      return allPrices.filter((p) => p.date >= manualFrom && p.date <= manualTo);
    }
    const cutoff = cutoffDate(range);
    if (!cutoff) return allPrices;
    return allPrices.filter((p) => p.date >= cutoff);
  });

  const W = 720;
  const H = 200;
  const PAD = 24;

  const points = $derived.by(() => {
    if (filteredPrices.length === 0) return [];
    const max = Math.max(...filteredPrices.map((p) => p.closeMicro), 1);
    const min = Math.min(...filteredPrices.map((p) => p.closeMicro));
    const rng = Math.max(max - min, 1);
    const stepX = (W - 2 * PAD) / Math.max(filteredPrices.length - 1, 1);
    return filteredPrices.map((p, i) => ({
      x: PAD + i * stepX,
      y: H - PAD - ((p.closeMicro - min) / rng) * (H - 2 * PAD),
      v: p.closeMicro,
      d: p.date,
    }));
  });

  const path = $derived.by(() => {
    if (points.length === 0) return '';
    return points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');
  });

  function fmtMicro(m: number): string {
    return (m / 1_000_000).toLocaleString(settings.lang === 'en' ? 'en' : 'de', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
  }

  const latest = $derived(filteredPrices.length > 0 ? filteredPrices[filteredPrices.length - 1] : null);
  const tp = $derived(t().portfolio);

  // Hover state
  interface HoverState {
    idx: number;
    x: number;
    y: number;
    v: number;
    d: string;
  }
  let hover = $state<HoverState | null>(null);
  let svgEl = $state<SVGSVGElement | undefined>(undefined);

  function handlePointerMove(ev: PointerEvent) {
    if (points.length < 2 || !svgEl) { hover = null; return; }
    const rect = svgEl.getBoundingClientRect();
    const xPx = ev.clientX - rect.left;
    const xViewport = (xPx / rect.width) * W;
    const rel = Math.max(0, Math.min(1, (xViewport - PAD) / (W - 2 * PAD)));
    const idx = Math.round(rel * (points.length - 1));
    if (idx < 0 || idx >= points.length) { hover = null; return; }
    hover = {
      idx,
      x: points[idx].x,
      y: points[idx].y,
      v: points[idx].v,
      d: points[idx].d,
    };
  }
  function handlePointerLeave() { hover = null; }

  const tooltipLeft = $derived(hover ? (hover.x / W) * 100 : 0);
  const tooltipTop = $derived(hover ? (hover.y / H) * 100 : 0);
</script>

<div class="head">
  <h3>{tp.priceHistoryTitle}</h3>
  <div class="seg">
    <button class:on={range === '1d'} onclick={() => (range = '1d')}>{tp.rangeDay}</button>
    <button class:on={range === '1w'} onclick={() => (range = '1w')}>{tp.rangeWeek}</button>
    <button class:on={range === '1m'} onclick={() => (range = '1m')}>1M</button>
    <button class:on={range === '3m'} onclick={() => (range = '3m')}>3M</button>
    <button class:on={range === '1y'} onclick={() => (range = '1y')}>1J</button>
    <button class:on={range === '5y'} onclick={() => (range = '5y')}>5J</button>
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

{#if loading && allPrices.length === 0}
  <div class="empty">…</div>
{:else if filteredPrices.length === 0}
  <div class="empty">{tp.pricesNotSet}</div>
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
      <path d={path} stroke="var(--accent)" stroke-width="1.5" fill="none" />
      {#if points.length > 0}
        <circle cx={points[points.length - 1].x} cy={points[points.length - 1].y} r="3" fill="var(--accent)" />
      {/if}
      {#if hover}
        <line
          x1={hover.x} x2={hover.x}
          y1={PAD} y2={H - PAD}
          stroke="var(--accent)" stroke-width="1" stroke-opacity="0.6"
          stroke-dasharray="3 3" pointer-events="none"
        />
        <circle
          cx={hover.x} cy={hover.y} r="4"
          fill="var(--accent)" stroke="var(--surface)" stroke-width="2"
          pointer-events="none"
        />
      {/if}
    </svg>
    {#if hover}
      <div class="chart-tooltip" style="left: {tooltipLeft}%; top: {tooltipTop}%;">
        <span class="label">{hover.d}</span>
        <span class="value">{fmtMicro(hover.v)} €</span>
      </div>
    {/if}
  </div>
  {#if latest}
    <div class="foot">
      <span class="num">{fmtMicro(latest.closeMicro)} €</span>
      <span class="muted">· {tp.lastUpdate.replace('{date}', latest.date)}</span>
    </div>
  {/if}
{/if}

<style>
  .head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .head h3 { margin: 0; font-size: 14px; font-weight: 500; }
  .seg { display: flex; gap: 0; }
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
  .seg button:last-child { border-right: 1px solid var(--border); border-top-right-radius: 6px; border-bottom-right-radius: 6px; }
  .seg button.on { color: var(--text); background: var(--surface-2); }
  .empty { padding: 24px; text-align: center; color: var(--text-faint); font-size: 13px; }
  .chart-wrap {
    position: relative;
  }
  .chart { width: 100%; height: 200px; display: block; cursor: crosshair; }
  .foot { display: flex; align-items: baseline; gap: 8px; margin-top: 4px; }
  .foot .num { font-size: 18px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .muted { color: var(--text-muted); font-size: 12px; }
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
