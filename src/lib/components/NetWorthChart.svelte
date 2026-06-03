<script lang="ts">
  interface HistoryPoint {
    m: string;
    v: number;
    year?: number;
    month?: number;
  }
  interface ForecastPoint {
    m: string;
    mid: number;
    lo: number;
    hi: number;
  }
  interface Props {
    history: HistoryPoint[];
    forecast: ForecastPoint[];
    height?: number;
    hide?: boolean;
    onPointClick?: (year: number, month: number) => void;
  }

  let { history, forecast, height = 220, hide = false, onPointClick }: Props = $props();

  let containerWidth = $state(700);
  const w = $derived(Math.max(360, containerWidth));
  const padL = 36;
  const padR = 12;
  const padT = 16;
  const padB = 26;

  function compact(n: number): string {
    if (hide) return '••';
    const abs = Math.abs(n);
    if (abs >= 1000) {
      return (n / 1000).toLocaleString('de-DE', { maximumFractionDigits: 1 }) + 'k';
    }
    return n.toLocaleString('de-DE', { maximumFractionDigits: 0 });
  }

  function fmtEurCompact(n: number): string {
    if (hide) return '•••• €';
    const abs = Math.abs(n);
    const sign = n < 0 ? '−' : '';
    if (abs >= 1_000_000) return `${sign}${(abs / 1_000_000).toFixed(1).replace('.', ',')} M €`;
    if (abs >= 1000) return `${sign}${(abs / 1000).toFixed(1).replace('.', ',')}k €`;
    return `${sign}${Math.round(abs)} €`;
  }

  // Zoom state
  let zoomedRange = $state<{ startIdx: number; endIdx: number } | null>(null);
  let dragging = $state<{ startX: number; currentX: number } | null>(null);

  // Visible data sliced when zoomed
  const visibleHistory = $derived(
    zoomedRange ? history.slice(zoomedRange.startIdx, zoomedRange.endIdx + 1) : history
  );

  // When zoomed, forecast is shown only if last visible history point is still the real last history point
  const visibleForecast = $derived.by(() => {
    if (!zoomedRange) return forecast;
    // Only show forecast if zoom includes the last history point
    if (zoomedRange.endIdx >= history.length - 1) return forecast;
    return [];
  });

  const total = $derived(visibleHistory.length + visibleForecast.length);

  const allMid = $derived([...visibleHistory.map((d) => d.v), ...visibleForecast.map((d) => d.mid)]);
  const allLo = $derived(visibleForecast.map((d) => d.lo));
  const allHi = $derived(visibleForecast.map((d) => d.hi));
  const min = $derived(Math.min(...allMid, ...allLo) * 0.95);
  const max = $derived(Math.max(...allMid, ...allHi) * 1.02);

  const xAt = (i: number) => padL + ((w - padL - padR) * i) / (total - 1);
  const yAt = (v: number) =>
    padT + (height - padT - padB) * (1 - (v - min) / (max - min));

  const histPts = $derived(
    visibleHistory.map((d, i) => [xAt(i), yAt(d.v)] as [number, number])
  );
  const fcMidPts = $derived(
    visibleForecast.map((d, i) => [xAt(visibleHistory.length + i), yAt(d.mid)] as [number, number])
  );
  const fcLoPts = $derived(
    visibleForecast.map((d, i) => [xAt(visibleHistory.length + i), yAt(d.lo)] as [number, number])
  );
  const fcHiPts = $derived(
    visibleForecast.map((d, i) => [xAt(visibleHistory.length + i), yAt(d.hi)] as [number, number])
  );
  const last = $derived(histPts[histPts.length - 1]);
  const fcMidFull = $derived([last, ...fcMidPts]);
  const fcLoFull = $derived([last, ...fcLoPts]);
  const fcHiFull = $derived([last, ...fcHiPts]);

  const ticks = $derived(
    Array.from({ length: 5 }, (_, i) => min + ((max - min) * i) / 4)
  );
  const allLabels = $derived([
    ...visibleHistory.map((d) => d.m),
    ...visibleForecast.map((d) => d.m),
  ]);

  function polyline(pts: [number, number][]): string {
    return pts.map(([x, y]) => `${x},${y}`).join(' ');
  }

  const histPath = $derived('M ' + histPts.map(([x, y]) => `${x},${y}`).join(' L '));
  const histArea = $derived(
    `M ${histPts[0][0]},${height - padB} L ${histPts
      .map(([x, y]) => `${x},${y}`)
      .join(' L ')} L ${histPts[histPts.length - 1][0]},${height - padB} Z`
  );
  const conePath = $derived(
    visibleForecast.length > 0
      ? 'M ' +
        [...fcHiFull, ...fcLoFull.slice().reverse()]
          .map(([x, y]) => `${x},${y}`)
          .join(' L ') +
        ' Z'
      : ''
  );

  // Hover state
  interface HoverState {
    idx: number;
    x: number;
    isForecast: boolean;
    histVal: number | null;
    fcMid: number | null;
    fcLo: number | null;
    fcHi: number | null;
    label: string;
  }
  let hover = $state<HoverState | null>(null);

  // Press tracking for click vs drag distinction
  let pressStart = $state<{ x: number; t: number } | null>(null);

  function mapClientXToViewport(clientX: number, el: Element): number {
    const rect = el.getBoundingClientRect();
    return ((clientX - rect.left) / rect.width) * w;
  }

  function handlePointerMove(ev: PointerEvent) {
    if (total < 2) { hover = null; return; }
    const rect = (ev.currentTarget as Element).getBoundingClientRect();
    const xPx = ev.clientX - rect.left;
    const xViewport = (xPx / rect.width) * w;
    const rel = Math.max(0, Math.min(1, (xViewport - padL) / (w - padL - padR)));
    const idx = Math.round(rel * (total - 1));
    if (idx < 0 || idx >= total) { hover = null; return; }
    const isForecast = idx >= visibleHistory.length;
    const fcIdx = idx - visibleHistory.length;
    hover = {
      idx,
      x: xAt(idx),
      isForecast,
      histVal: isForecast ? null : visibleHistory[idx].v,
      fcMid: isForecast ? visibleForecast[fcIdx].mid : null,
      fcLo: isForecast ? visibleForecast[fcIdx].lo : null,
      fcHi: isForecast ? visibleForecast[fcIdx].hi : null,
      label: allLabels[idx] ?? '',
    };
    if (dragging) {
      const x = mapClientXToViewport(ev.clientX, ev.currentTarget as Element);
      dragging = { ...dragging, currentX: x };
    }
  }
  function handlePointerLeave() {
    hover = null;
    if (!dragging) return;
    // keep dragging if pointer leaves — pointer capture handles it
  }

  function handlePointerDown(ev: PointerEvent) {
    pressStart = { x: ev.clientX, t: Date.now() };
    const x = mapClientXToViewport(ev.clientX, ev.currentTarget as Element);
    dragging = { startX: x, currentX: x };
    (ev.currentTarget as Element).setPointerCapture(ev.pointerId);
  }

  function handlePointerUp(ev: PointerEvent) {
    const wasClick =
      pressStart !== null &&
      Math.abs(ev.clientX - pressStart.x) < 4 &&
      Date.now() - pressStart.t < 500;

    if (wasClick) {
      // Click-to-navigate: use hovered point
      if (hover && onPointClick) {
        const pt = visibleHistory[hover.idx];
        if (pt?.year != null && pt?.month != null) {
          onPointClick(pt.year, pt.month);
        }
      }
    } else if (dragging && Math.abs(dragging.currentX - dragging.startX) > 8) {
      // Brush-zoom
      const x1 = Math.min(dragging.startX, dragging.currentX);
      const x2 = Math.max(dragging.startX, dragging.currentX);

      // Map viewport coords back to absolute history indices
      const baseStart = zoomedRange?.startIdx ?? 0;
      const baseEnd = zoomedRange?.endIdx ?? history.length - 1;
      const baseLen = baseEnd - baseStart + 1;

      function viewportXToAbsIdx(vx: number): number {
        const rel = (vx - padL) / (w - padL - padR);
        const localIdx = Math.round(Math.max(0, Math.min(1, rel)) * (total - 1));
        // localIdx is within visibleHistory (+ forecast). Clamp to visibleHistory.
        const clampedLocal = Math.min(localIdx, visibleHistory.length - 1);
        return baseStart + Math.round((clampedLocal / Math.max(1, visibleHistory.length - 1)) * (baseLen - 1));
      }

      const absIdx1 = Math.max(0, viewportXToAbsIdx(x1));
      const absIdx2 = Math.min(history.length - 1, viewportXToAbsIdx(x2));

      if (absIdx2 - absIdx1 >= 1) {
        zoomedRange = { startIdx: absIdx1, endIdx: absIdx2 };
      }
    }

    dragging = null;
    pressStart = null;
  }

  const hoverY = $derived.by(() => {
    if (!hover) return 0;
    if (hover.isForecast && hover.fcMid !== null) return yAt(hover.fcMid);
    if (!hover.isForecast && hover.histVal !== null) return yAt(hover.histVal);
    return 0;
  });
  const tooltipLeft = $derived(hover ? (hover.x / w) * 100 : 0);
  const tooltipTop = $derived(hover ? (hoverY / height) * 100 : 0);

  // Brush overlay dimensions
  const brushX = $derived(dragging ? Math.min(dragging.startX, dragging.currentX) : 0);
  const brushW = $derived(dragging ? Math.abs(dragging.currentX - dragging.startX) : 0);
  const showBrush = $derived(dragging !== null && brushW > 4);
</script>

<div class="wrap" bind:clientWidth={containerWidth}>
  {#if zoomedRange}
    <button class="zoom-reset" type="button" onclick={() => (zoomedRange = null)}>
      ↺ Zoom zurücksetzen
    </button>
  {/if}
  <svg viewBox="0 0 {w} {height}" width="100%" {height}>
    <defs>
      <linearGradient id="nwArea" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="var(--accent)" stop-opacity="0.18" />
        <stop offset="100%" stop-color="var(--accent)" stop-opacity="0" />
      </linearGradient>
      <linearGradient id="conebg" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="var(--accent)" stop-opacity="0.13" />
        <stop offset="100%" stop-color="var(--accent)" stop-opacity="0.03" />
      </linearGradient>
    </defs>
    {#each ticks as t, i (i)}
      <g>
        <line
          x1={padL}
          x2={w - padR}
          y1={yAt(t)}
          y2={yAt(t)}
          stroke="var(--border)"
          stroke-dasharray="2 4"
        />
        <text
          x={padL - 8}
          y={yAt(t) + 3}
          text-anchor="end"
          font-size="10"
          fill="var(--text-faint)"
          font-family="var(--font-mono)">{compact(t)}</text
        >
      </g>
    {/each}
    <path d={histArea} fill="url(#nwArea)" />
    <path d={histPath} fill="none" stroke="var(--accent)" stroke-width="2" />
    {#if conePath}
      <path d={conePath} fill="url(#conebg)" stroke="none" />
      <polyline
        points={polyline(fcMidFull)}
        fill="none"
        stroke="var(--accent)"
        stroke-width="1.5"
        stroke-dasharray="5 4"
        opacity="0.85"
      />
      <polyline
        points={polyline(fcHiFull)}
        fill="none"
        stroke="var(--accent)"
        stroke-width="1"
        stroke-dasharray="2 3"
        opacity="0.4"
      />
      <polyline
        points={polyline(fcLoFull)}
        fill="none"
        stroke="var(--accent)"
        stroke-width="1"
        stroke-dasharray="2 3"
        opacity="0.4"
      />
    {/if}
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
    {#each allLabels as label, i (i)}
      {#if i % 2 === 0}
        <text
          x={xAt(i)}
          y={height - 6}
          text-anchor="middle"
          font-size="10"
          fill="var(--text-faint)"
          font-family="var(--font-mono)">{label}</text
        >
      {/if}
    {/each}
    {#if hover}
      <line
        x1={hover.x} x2={hover.x}
        y1={padT} y2={height - padB}
        stroke="var(--accent)" stroke-width="1" stroke-opacity="0.6"
        stroke-dasharray="3 3" pointer-events="none"
      />
      <circle
        cx={hover.x} cy={hoverY} r="4"
        fill="var(--accent)" stroke="var(--surface)" stroke-width="2"
        pointer-events="none"
      />
    {/if}
    {#if showBrush}
      <rect
        x={brushX} y={padT}
        width={brushW} height={height - padT - padB}
        fill="var(--accent)" fill-opacity="0.15"
        stroke="var(--accent)" stroke-opacity="0.5" stroke-width="1"
        pointer-events="none"
      />
    {/if}
    <rect
      x={padL} y={padT}
      width={w - padL - padR} height={height - padT - padB}
      fill="transparent"
      style="cursor: {onPointClick ? 'pointer' : 'crosshair'};"
      onpointermove={handlePointerMove}
      onpointerleave={handlePointerLeave}
      onpointerdown={handlePointerDown}
      onpointerup={handlePointerUp}
    />
  </svg>
  {#if hover}
    <div class="chart-tooltip" style="left: {tooltipLeft}%; top: {tooltipTop}%;">
      <span class="label">{hover.label}</span>
      {#if hover.isForecast}
        <span class="value">Mid: {fmtEurCompact(hover.fcMid!)}</span>
        <span class="label">Lo: {fmtEurCompact(hover.fcLo!)} · Hi: {fmtEurCompact(hover.fcHi!)}</span>
      {:else}
        <span class="value">{fmtEurCompact(hover.histVal!)}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .wrap {
    width: 100%;
    position: relative;
  }
  svg {
    display: block;
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
  .zoom-reset {
    position: absolute;
    top: 8px;
    right: 8px;
    font-size: 11px;
    padding: 4px 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    z-index: 5;
    color: var(--text-muted);
  }
  .zoom-reset:hover {
    color: var(--text);
    border-color: var(--accent);
  }
</style>
