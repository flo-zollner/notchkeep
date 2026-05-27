<script lang="ts">
  interface HistoryPoint {
    m: string;
    v: number;
  }
  interface Props {
    history: HistoryPoint[];
    height?: number;
    hide?: boolean;
    /** Index of the baseline point in the history array. null/undefined = first valid point. */
    baselineIdx?: number | null;
    /** Callback when the user clicks a data point to set the baseline. */
    onBaselineChange?: (newIdx: number) => void;
  }

  let { history, height = 220, hide = false, baselineIdx = null, onBaselineChange }: Props = $props();

  const w = 700;
  const padL = 44;
  const padR = 12;
  const padT = 16;
  const padB = 26;

  function fmtPct(n: number): string {
    if (hide) return '••%';
    return `${Math.round(n)}%`;
  }

  // Resolve effective baseline index: use prop if valid, else fallback to first positive point.
  const effectiveBaselineIdx = $derived.by(() => {
    if (baselineIdx !== null && baselineIdx !== undefined && baselineIdx >= 0 && baselineIdx < history.length && history[baselineIdx].v > 0) {
      return baselineIdx;
    }
    const fallback = history.findIndex((d) => d.v > 0);
    return fallback >= 0 ? fallback : null;
  });

  const baseline = $derived(
    effectiveBaselineIdx !== null ? history[effectiveBaselineIdx].v : null,
  );

  const indexed = $derived(
    baseline !== null
      ? history.map((d) => ({ m: d.m, pct: (d.v / baseline) * 100 }))
      : [],
  );

  const pctValues = $derived(indexed.map((d) => d.pct));
  const minPct = $derived(
    pctValues.length > 0
      ? Math.floor(Math.min(...pctValues, 100) / 10) * 10 - 5
      : 95,
  );
  const maxPct = $derived(
    pctValues.length > 0
      ? Math.ceil(Math.max(...pctValues, 100) / 10) * 10 + 5
      : 105,
  );

  const total = $derived(indexed.length);

  const xAt = (i: number) =>
    total > 1 ? padL + ((w - padL - padR) * i) / (total - 1) : padL;
  const yAt = (pct: number) =>
    padT + (height - padT - padB) * (1 - (pct - minPct) / (maxPct - minPct));

  const pts = $derived(
    indexed.map((d, i) => [xAt(i), yAt(d.pct)] as [number, number]),
  );

  const last = $derived(pts[pts.length - 1]);

  const ticks = $derived(
    Array.from({ length: 5 }, (_, i) => minPct + ((maxPct - minPct) * i) / 4),
  );

  const histPath = $derived(
    pts.length > 0
      ? 'M ' + pts.map(([x, y]) => `${x},${y}`).join(' L ')
      : '',
  );
  const histArea = $derived(
    pts.length > 0
      ? `M ${pts[0][0]},${height - padB} L ${pts.map(([x, y]) => `${x},${y}`).join(' L ')} L ${pts[pts.length - 1][0]},${height - padB} Z`
      : '',
  );

  const baselineY = $derived(yAt(100));

  function handleChartClick(ev: MouseEvent) {
    if (!onBaselineChange) return;
    const rect = (ev.currentTarget as Element).getBoundingClientRect();
    const xPx = ev.clientX - rect.left;
    const xViewport = (xPx / rect.width) * w;
    const rel = (xViewport - padL) / (w - padL - padR);
    const idx = Math.max(0, Math.min(history.length - 1, Math.round(rel * (history.length - 1))));
    onBaselineChange(idx);
  }

  // Hover state
  interface HoverState {
    idx: number;
    x: number;
    y: number;
    pct: number;
    label: string;
  }
  let hover = $state<HoverState | null>(null);

  function handlePointerMove(ev: PointerEvent) {
    if (total < 2) { hover = null; return; }
    const rect = (ev.currentTarget as Element).getBoundingClientRect();
    const xPx = ev.clientX - rect.left;
    const xViewport = (xPx / rect.width) * w;
    const rel = Math.max(0, Math.min(1, (xViewport - padL) / (w - padL - padR)));
    const idx = Math.round(rel * (total - 1));
    if (idx < 0 || idx >= total) { hover = null; return; }
    hover = {
      idx,
      x: xAt(idx),
      y: yAt(indexed[idx].pct),
      pct: indexed[idx].pct,
      label: indexed[idx].m,
    };
  }
  function handlePointerLeave() { hover = null; }

  const tooltipLeft = $derived(hover ? (hover.x / w) * 100 : 0);
  const tooltipTop = $derived(hover ? (hover.y / height) * 100 : 0);
</script>

{#if baseline === null}
  <div class="empty">Kein positiver Startwert für Indexierung verfügbar</div>
{:else}
  <div class="wrap">
    <svg
      viewBox="0 0 {w} {height}"
      width="100%"
      {height}
      preserveAspectRatio="none"
    >
      <defs>
        <linearGradient id="nwiArea" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--accent)" stop-opacity="0.18" />
          <stop offset="100%" stop-color="var(--accent)" stop-opacity="0" />
        </linearGradient>
      </defs>

      {#each ticks as tick, i (i)}
        <g>
          <line
            x1={padL}
            x2={w - padR}
            y1={yAt(tick)}
            y2={yAt(tick)}
            stroke="var(--border)"
            stroke-dasharray="2 4"
          />
          <text
            x={padL - 8}
            y={yAt(tick) + 3}
            text-anchor="end"
            font-size="10"
            fill="var(--text-faint)"
            font-family="var(--font-mono)">{fmtPct(tick)}</text
          >
        </g>
      {/each}

      <!-- Baseline reference line at 100% -->
      <line
        x1={padL}
        x2={w - padR}
        y1={baselineY}
        y2={baselineY}
        stroke="var(--accent)"
        stroke-width="1.5"
        stroke-dasharray="5 3"
        opacity="0.5"
      />

      <path d={histArea} fill="url(#nwiArea)" />
      <path d={histPath} fill="none" stroke="var(--accent)" stroke-width="2" />

      {#if last}
        <line
          x1={last[0]}
          x2={last[0]}
          y1={padT}
          y2={height - padB}
          stroke="var(--border-strong)"
          stroke-dasharray="3 3"
        />
        <circle
          cx={last[0]}
          cy={last[1]}
          r="4"
          fill="var(--accent)"
          stroke="var(--surface)"
          stroke-width="2"
        />
      {/if}

      {#each indexed as point, i (i)}
        {#if i % 2 === 0}
          <text
            x={xAt(i)}
            y={height - 6}
            text-anchor="middle"
            font-size="10"
            fill="var(--text-faint)"
            font-family="var(--font-mono)">{point.m}</text
          >
        {/if}
      {/each}

      <!-- Baseline marker -->
      {#if effectiveBaselineIdx !== null && effectiveBaselineIdx < pts.length}
        <circle
          cx={pts[effectiveBaselineIdx][0]}
          cy={pts[effectiveBaselineIdx][1]}
          r="5"
          fill="var(--accent)"
          stroke="var(--surface)"
          stroke-width="2"
        />
        <text
          x={pts[effectiveBaselineIdx][0]}
          y={pts[effectiveBaselineIdx][1] - 10}
          text-anchor="middle"
          font-size="10"
          font-weight="600"
          fill="var(--accent)"
        >100%</text>
      {/if}

      {#if hover}
        <line
          x1={hover.x} x2={hover.x}
          y1={padT} y2={height - padB}
          stroke="var(--accent)" stroke-width="1" stroke-opacity="0.6"
          stroke-dasharray="3 3" pointer-events="none"
        />
        <circle
          cx={hover.x} cy={hover.y} r="4"
          fill="var(--accent)" stroke="var(--surface)" stroke-width="2"
          pointer-events="none"
        />
      {/if}

      <!-- Invisible overlay — rendered last so it sits on top -->
      <rect
        x={padL}
        y={padT}
        width={w - padL - padR}
        height={height - padT - padB}
        fill="transparent"
        style="cursor: pointer;"
        onclick={handleChartClick}
        onpointermove={handlePointerMove}
        onpointerleave={handlePointerLeave}
      />
    </svg>
    {#if hover}
      <div class="chart-tooltip" style="left: {tooltipLeft}%; top: {tooltipTop}%;">
        <span class="label">{hover.label}</span>
        <span class="value">{fmtPct(hover.pct)}</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  .wrap {
    width: 100%;
    position: relative;
  }
  svg {
    display: block;
  }
  .empty {
    padding: 24px;
    text-align: center;
    color: var(--text-faint);
    font-size: 13px;
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
