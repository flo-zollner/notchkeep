<script lang="ts">
  import { fmtEur } from '$lib/api';

  export type SavingsChartMode = 'bars' | 'line' | 'inout';

  interface Flow {
    year: number;
    month: number;
    inCents: number;
    outCents: number;
  }
  interface Props {
    flows: Flow[];
    mode: SavingsChartMode;
    hide?: boolean;
    onPointClick?: (year: number, month: number) => void;
  }
  let { flows, mode, hide = false, onPointClick }: Props = $props();

  let containerW = $state(720);
  const H = 170;
  const PAD_R = 14;
  const PAD_T = 14;
  const PAD_B = 26;
  const PAD_L = $derived(mode === 'inout' ? 60 : 44);
  const plotW = $derived(Math.max(120, containerW - PAD_L - PAD_R));
  const plotH = H - PAD_T - PAD_B;

  function fmtCompactEur(cents: number): string {
    if (hide) return '•• €';
    const eur = cents / 100;
    const abs = Math.abs(eur);
    const sign = eur < 0 ? '−' : '';
    if (abs >= 1_000_000) return `${sign}${(abs / 1_000_000).toFixed(1).replace('.', ',')} M €`;
    if (abs >= 10_000) return `${sign}${Math.round(abs / 1_000)}k €`;
    if (abs >= 1_000) return `${sign}${(abs / 1_000).toFixed(1).replace('.', ',')}k €`;
    return `${sign}${Math.round(abs)} €`;
  }

  const rates = $derived(
    flows.map((f) => (f.inCents > 0 ? ((f.inCents - f.outCents) / f.inCents) * 100 : 0))
  );
  const validRates = $derived(rates.filter((_, i) => flows[i].inCents > 0));
  const avg = $derived(
    validRates.length > 0 ? validRates.reduce((s, r) => s + r, 0) / validRates.length : 0
  );

  function niceMax(v: number): number {
    if (v <= 25) return 25;
    if (v <= 50) return 50;
    if (v <= 75) return 75;
    return Math.ceil(v / 25) * 25;
  }
  function niceMin(v: number): number {
    if (v >= 0) return 0;
    return -Math.ceil(-v / 25) * 25;
  }

  const yMax = $derived(niceMax(Math.max(...rates, 25)));
  const yMin = $derived(niceMin(Math.min(...rates, 0)));
  const yRange = $derived(Math.max(yMax - yMin, 1));

  function yForRate(r: number): number {
    return PAD_T + (1 - (r - yMin) / yRange) * plotH;
  }
  function xSlot(i: number, n: number): number {
    if (n <= 1) return PAD_L + plotW / 2;
    return PAD_L + ((i + 0.5) / n) * plotW;
  }
  const barW = $derived(Math.max(8, (plotW / Math.max(flows.length, 1)) * 0.55));

  const maxAbsCents = $derived(
    Math.max(...flows.map((f) => Math.max(f.inCents, f.outCents)), 1)
  );

  const ticks = $derived.by(() => {
    if (yMin === 0) return [0, Math.round(yMax / 2), yMax];
    return [yMin, 0, yMax];
  });

  function ml(f: Flow): string {
    const months = ['Jan', 'Feb', 'Mär', 'Apr', 'Mai', 'Jun', 'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dez'];
    return `${months[f.month - 1]} ${f.year}`;
  }
  function mlShort(f: Flow): string {
    return `${String(f.month).padStart(2, '0')}/${String(f.year % 100).padStart(2, '0')}`;
  }

  function showTick(i: number, n: number): boolean {
    if (n <= 8) return true;
    const step = Math.ceil(n / 6);
    return i % step === 0 || i === n - 1;
  }

  let svgEl: SVGSVGElement | undefined = $state();
  let hoverIdx = $state<number | null>(null);
  let pressStart = $state<{ x: number; t: number } | null>(null);

  function onMove(e: PointerEvent) {
    if (!svgEl || flows.length === 0) return;
    const rect = svgEl.getBoundingClientRect();
    const xPx = e.clientX - rect.left;
    let best = 0;
    let bestD = Infinity;
    for (let i = 0; i < flows.length; i++) {
      const d = Math.abs(xSlot(i, flows.length) - xPx);
      if (d < bestD) {
        bestD = d;
        best = i;
      }
    }
    hoverIdx = best;
  }
  function onLeave() {
    hoverIdx = null;
  }
  function onDown(e: PointerEvent) {
    pressStart = { x: e.clientX, t: Date.now() };
  }
  function onUp(e: PointerEvent) {
    const wasClick =
      pressStart !== null &&
      Math.abs(e.clientX - pressStart.x) < 4 &&
      Date.now() - pressStart.t < 500;
    pressStart = null;
    if (wasClick && hoverIdx !== null && onPointClick) {
      const f = flows[hoverIdx];
      if (f) onPointClick(f.year, f.month);
    }
  }

  const tooltipX = $derived(hoverIdx !== null ? xSlot(hoverIdx, flows.length) : 0);
  const tooltipSide = $derived(tooltipX > containerW * 0.6 ? 'right' : 'left');
</script>

<div class="wrap" bind:clientWidth={containerW}>
  {#if flows.length === 0}
    <div class="empty">—</div>
  {:else}
    <svg
      bind:this={svgEl}
      viewBox="0 0 {containerW} {H}"
      width={containerW}
      height={H}
      role="img"
      aria-label="Sparquote"
      style="cursor: {onPointClick ? 'pointer' : 'default'};"
      onpointermove={onMove}
      onpointerleave={onLeave}
      onpointerdown={onDown}
      onpointerup={onUp}
    >
      <defs>
        <linearGradient id="srcAreaFill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stop-color="var(--accent)" stop-opacity="0.32" />
          <stop offset="100%" stop-color="var(--accent)" stop-opacity="0.02" />
        </linearGradient>
      </defs>

      {#if mode === 'inout'}
        {@const halfH = plotH / 2}
        {@const yMid = PAD_T + halfH}
        <text x={PAD_L - 7} y={PAD_T + 4} text-anchor="end" class="axis">
          +{fmtCompactEur(maxAbsCents)}
        </text>
        <text x={PAD_L - 7} y={yMid + 3} text-anchor="end" class="axis">0</text>
        <text x={PAD_L - 7} y={PAD_T + plotH} text-anchor="end" class="axis">
          −{fmtCompactEur(maxAbsCents)}
        </text>
        <line x1={PAD_L} y1={yMid} x2={containerW - PAD_R} y2={yMid} class="axis-line" />

        {#if hoverIdx !== null}
          <line
            x1={xSlot(hoverIdx, flows.length)}
            y1={PAD_T}
            x2={xSlot(hoverIdx, flows.length)}
            y2={PAD_T + plotH}
            class="crosshair"
          />
        {/if}

        {#each flows as f, i (i)}
          {@const x = xSlot(i, flows.length)}
          {@const hIn = (f.inCents / maxAbsCents) * halfH}
          {@const hOut = (f.outCents / maxAbsCents) * halfH}
          {@const bw = Math.max(5, barW * 0.42)}
          {@const isHover = hoverIdx === i}
          <rect
            x={x - bw - 1.5}
            y={yMid - hIn}
            width={bw}
            height={Math.max(1.5, hIn)}
            class="bar bar-pos"
            class:dim={hoverIdx !== null && !isHover}
            rx="2"
          ></rect>
          <rect
            x={x + 1.5}
            y={yMid}
            width={bw}
            height={Math.max(1.5, hOut)}
            class="bar bar-neg"
            class:dim={hoverIdx !== null && !isHover}
            rx="2"
          ></rect>
        {/each}
      {:else}
        {#each ticks as tk (tk)}
          {@const y = yForRate(tk)}
          <line
            x1={PAD_L}
            y1={y}
            x2={containerW - PAD_R}
            y2={y}
            class="grid"
            class:zero={tk === 0}
          />
          <text x={PAD_L - 7} y={y + 3} text-anchor="end" class="axis">{tk}%</text>
        {/each}

        {#if hoverIdx !== null}
          <line
            x1={xSlot(hoverIdx, flows.length)}
            y1={PAD_T}
            x2={xSlot(hoverIdx, flows.length)}
            y2={PAD_T + plotH}
            class="crosshair"
          />
        {/if}

        {#if mode === 'bars'}
          {#each flows as f, i (i)}
            {@const r = rates[i]}
            {@const x = xSlot(i, flows.length)}
            {@const y0 = yForRate(0)}
            {@const y = yForRate(r)}
            {@const isHover = hoverIdx === i}
            <rect
              x={x - barW / 2}
              y={Math.min(y, y0)}
              width={barW}
              height={Math.max(2, Math.abs(y - y0))}
              class={r >= 0 ? 'bar bar-pos' : 'bar bar-neg'}
              class:dim={hoverIdx !== null && !isHover}
              rx="3"
            ></rect>
          {/each}
        {:else}
          {@const y0 = yForRate(0)}
          {@const ptsArr = flows.map((_, i) => `${xSlot(i, flows.length).toFixed(1)},${yForRate(rates[i]).toFixed(1)}`)}
          {@const pts = ptsArr.join(' ')}
          {@const areaPts = `${xSlot(0, flows.length).toFixed(1)},${y0.toFixed(1)} ${pts} ${xSlot(flows.length - 1, flows.length).toFixed(1)},${y0.toFixed(1)}`}
          <polygon points={areaPts} fill="url(#srcAreaFill)" />
          <polyline points={pts} class="line" />
          {#each flows as _, i (i)}
            {@const isHover = hoverIdx === i}
            <circle
              cx={xSlot(i, flows.length)}
              cy={yForRate(rates[i])}
              r={isHover ? 5 : 3}
              class="dot"
              class:dim={hoverIdx !== null && !isHover}
            ></circle>
          {/each}
        {/if}

        {#if validRates.length > 0}
          {@const yAvg = yForRate(avg)}
          <line
            x1={PAD_L}
            y1={yAvg}
            x2={containerW - PAD_R}
            y2={yAvg}
            class="avg-line"
          />
          <text
            x={containerW - PAD_R - 4}
            y={yAvg - 4}
            text-anchor="end"
            class="avg-label"
          >
            ∅ {avg.toFixed(0)}%
          </text>
        {/if}
      {/if}

      {#each flows as f, i (i)}
        {#if showTick(i, flows.length)}
          <text
            x={xSlot(i, flows.length)}
            y={H - 8}
            text-anchor="middle"
            class="axis x-label"
            class:active={hoverIdx === i}
          >
            {mlShort(f)}
          </text>
        {/if}
      {/each}
    </svg>

    {#if hoverIdx !== null}
      {@const f = flows[hoverIdx]}
      {@const r = rates[hoverIdx]}
      <div
        class="tooltip"
        class:right={tooltipSide === 'right'}
        style:left="{tooltipX}px"
      >
        <div class="tt-h">{ml(f)}</div>
        <div class="tt-row">
          <span class="tt-dot pos"></span>
          <span class="tt-k">Einnahmen</span>
          <span class="tt-v">{fmtEur(f.inCents, { hide })}</span>
        </div>
        <div class="tt-row">
          <span class="tt-dot neg"></span>
          <span class="tt-k">Ausgaben</span>
          <span class="tt-v">{fmtEur(f.outCents, { hide })}</span>
        </div>
        <div class="tt-row net">
          <span class="tt-dot net-dot" class:pos={f.inCents - f.outCents >= 0} class:neg={f.inCents - f.outCents < 0}></span>
          <span class="tt-k">Netto</span>
          <span class="tt-v">{fmtEur(f.inCents - f.outCents, { hide })}</span>
        </div>
        <div class="tt-quote">
          <span class="tt-k">Sparquote</span>
          <span class="tt-q-v" class:pos={r >= 0} class:neg={r < 0}>
            {f.inCents > 0 ? `${r.toFixed(1)}%` : '—'}
          </span>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    width: 100%;
  }
  .empty {
    padding: 28px 0;
    text-align: center;
    color: var(--text-faint);
  }
  svg {
    display: block;
    max-width: 100%;
    touch-action: none;
  }
  .axis {
    font-size: 10px;
    fill: var(--text-faint);
    font-family: var(--font-mono, ui-monospace, monospace);
  }
  .x-label {
    transition: fill 0.15s;
  }
  .x-label.active {
    fill: var(--text);
    font-weight: 600;
  }
  .axis-line {
    stroke: var(--border-strong);
    stroke-width: 0.8;
  }
  .grid {
    stroke: var(--border);
    stroke-width: 0.6;
    stroke-dasharray: 2 4;
  }
  .grid.zero {
    stroke: var(--border-strong);
    stroke-dasharray: 0;
  }
  .crosshair {
    stroke: var(--text-faint);
    stroke-width: 0.8;
    stroke-dasharray: 3 3;
    opacity: 0.5;
    pointer-events: none;
  }
  .bar {
    transition: opacity 0.15s, y 0.25s cubic-bezier(0.22, 1, 0.36, 1), height 0.25s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .bar-pos { fill: var(--positive); opacity: 0.85; }
  .bar-neg { fill: var(--negative); opacity: 0.85; }
  .bar.dim { opacity: 0.35; }
  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .dot {
    fill: var(--surface);
    stroke: var(--accent);
    stroke-width: 1.8;
    transition: r 0.15s, opacity 0.15s;
  }
  .dot.dim {
    opacity: 0.4;
  }
  .avg-line {
    stroke: var(--text-muted);
    stroke-width: 1;
    stroke-dasharray: 5 3;
    opacity: 0.65;
    pointer-events: none;
  }
  .avg-label {
    font-size: 10px;
    fill: var(--text-muted);
    font-weight: 500;
    pointer-events: none;
  }

  .tooltip {
    position: absolute;
    top: 6px;
    transform: translateX(12px);
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 6px 22px rgba(0, 0, 0, 0.08), 0 1px 3px rgba(0, 0, 0, 0.06);
    padding: 9px 11px;
    min-width: 168px;
    font-size: 12px;
    color: var(--text);
    pointer-events: none;
    z-index: 2;
    animation: tt-in 0.12s ease-out;
  }
  .tooltip.right {
    transform: translateX(calc(-100% - 12px));
  }
  @keyframes tt-in {
    from { opacity: 0; transform: translateX(12px) translateY(-3px); }
    to   { opacity: 1; }
  }
  .tooltip.right {
    animation-name: tt-in-right;
  }
  @keyframes tt-in-right {
    from { opacity: 0; transform: translateX(calc(-100% - 12px)) translateY(-3px); }
    to   { opacity: 1; }
  }
  .tt-h {
    font-weight: 600;
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 6px;
  }
  .tt-row {
    display: grid;
    grid-template-columns: 10px 1fr auto;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
  }
  .tt-row.net {
    border-top: 1px dashed var(--border);
    margin-top: 4px;
    padding-top: 5px;
  }
  .tt-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .tt-dot.pos { background: var(--positive); }
  .tt-dot.neg { background: var(--negative); }
  .net-dot.pos { background: var(--positive); }
  .net-dot.neg { background: var(--negative); }
  .tt-k {
    color: var(--text-muted);
  }
  .tt-v {
    font-variant-numeric: tabular-nums;
    font-weight: 500;
  }
  .tt-quote {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-top: 6px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
  .tt-q-v {
    font-weight: 700;
    font-size: 14px;
    font-variant-numeric: tabular-nums;
  }
  .tt-q-v.pos { color: var(--positive); }
  .tt-q-v.neg { color: var(--negative); }
</style>
