<script lang="ts">
  interface Point {
    m: string;
    in: number;
    out: number;
    year?: number;
    month?: number;
  }
  interface Props {
    data: Point[];
    height?: number;
    hide?: boolean;
    onPointClick?: (year: number, month: number) => void;
  }
  let { data, height = 200, hide = false, onPointClick }: Props = $props();

  const w = 600;
  const padL = 40;
  const padR = 8;
  const padT = 12;
  const padB = 26;

  function compact(n: number): string {
    if (hide) return '••';
    const abs = Math.abs(n);
    if (abs >= 1000) return (n / 1000).toLocaleString('de-DE', { maximumFractionDigits: 1 }) + 'k';
    return n.toLocaleString('de-DE', { maximumFractionDigits: 0 });
  }

  function fmtEurFull(n: number): string {
    if (hide) return '•••• €';
    return n.toLocaleString('de-DE', { maximumFractionDigits: 0 }) + ' €';
  }

  // Zoom state
  let zoomedRange = $state<{ startIdx: number; endIdx: number } | null>(null);
  let dragging = $state<{ startX: number; currentX: number } | null>(null);

  const visibleData = $derived(
    zoomedRange ? data.slice(zoomedRange.startIdx, zoomedRange.endIdx + 1) : data
  );

  const max = $derived(Math.max(...visibleData.map((d) => Math.max(d.in, d.out))) * 1.15);
  const bw = $derived((w - padL - padR) / Math.max(visibleData.length, 1));
  const barW = $derived(Math.min(18, bw * 0.35));

  // Hover state
  interface HoverState {
    idx: number;
    x: number;
    label: string;
    inVal: number;
    outVal: number;
  }
  let hover = $state<HoverState | null>(null);
  let svgEl = $state<SVGSVGElement | undefined>(undefined);

  let pressStart = $state<{ x: number; t: number } | null>(null);

  function mapClientXToViewport(clientX: number): number {
    if (!svgEl) return 0;
    const rect = svgEl.getBoundingClientRect();
    return ((clientX - rect.left) / rect.width) * w;
  }

  function idxFromViewportX(viewportX: number): number {
    const n = visibleData.length;
    if (n === 0) return 0;
    let best = 0;
    let bestD = Infinity;
    for (let i = 0; i < n; i++) {
      const cx = padL + bw * i + bw / 2;
      const d = Math.abs(cx - viewportX);
      if (d < bestD) { bestD = d; best = i; }
    }
    return best;
  }

  function handlePointerMove(ev: PointerEvent) {
    if (visibleData.length === 0 || !svgEl) { hover = null; return; }
    const rect = svgEl.getBoundingClientRect();
    const xPx = ev.clientX - rect.left;
    const xViewport = (xPx / rect.width) * w;
    const best = idxFromViewportX(xViewport);
    const cx = padL + bw * best + bw / 2;
    hover = {
      idx: best,
      x: cx,
      label: visibleData[best].m,
      inVal: visibleData[best].in,
      outVal: visibleData[best].out,
    };
    if (dragging) {
      dragging = { ...dragging, currentX: mapClientXToViewport(ev.clientX) };
    }
  }
  function handlePointerLeave() { hover = null; }

  function handlePointerDown(ev: PointerEvent) {
    pressStart = { x: ev.clientX, t: Date.now() };
    const x = mapClientXToViewport(ev.clientX);
    dragging = { startX: x, currentX: x };
    (ev.currentTarget as Element).setPointerCapture(ev.pointerId);
  }

  function handlePointerUp(ev: PointerEvent) {
    const wasClick =
      pressStart !== null &&
      Math.abs(ev.clientX - pressStart.x) < 4 &&
      Date.now() - pressStart.t < 500;

    if (wasClick) {
      if (hover && onPointClick) {
        const pt = visibleData[hover.idx];
        if (pt?.year != null && pt?.month != null) {
          onPointClick(pt.year, pt.month);
        }
      }
    } else if (dragging && Math.abs(dragging.currentX - dragging.startX) > 8) {
      const x1 = Math.min(dragging.startX, dragging.currentX);
      const x2 = Math.max(dragging.startX, dragging.currentX);

      const baseStart = zoomedRange?.startIdx ?? 0;
      const baseEnd = zoomedRange?.endIdx ?? data.length - 1;
      const baseLen = baseEnd - baseStart + 1;

      function viewportXToAbsIdx(vx: number): number {
        const localIdx = idxFromViewportX(vx);
        return baseStart + Math.round((localIdx / Math.max(1, visibleData.length - 1)) * (baseLen - 1));
      }

      const absIdx1 = Math.max(0, viewportXToAbsIdx(x1));
      const absIdx2 = Math.min(data.length - 1, viewportXToAbsIdx(x2));

      if (absIdx2 - absIdx1 >= 1) {
        zoomedRange = { startIdx: absIdx1, endIdx: absIdx2 };
      }
    }

    dragging = null;
    pressStart = null;
  }

  const tooltipLeft = $derived(hover ? (hover.x / w) * 100 : 0);
  // Position tooltip near top of chart area
  const tooltipTop = $derived(padT / height * 100 + 4);

  // Brush overlay
  const brushX = $derived(dragging ? Math.min(dragging.startX, dragging.currentX) : 0);
  const brushW = $derived(dragging ? Math.abs(dragging.currentX - dragging.startX) : 0);
  const showBrush = $derived(dragging !== null && brushW > 4);
</script>

<div class="wrap">
  {#if zoomedRange}
    <button class="zoom-reset" type="button" onclick={() => (zoomedRange = null)}>
      ↺ Zoom zurücksetzen
    </button>
  {/if}
  <svg
    bind:this={svgEl}
    viewBox="0 0 {w} {height}" width="100%" {height} preserveAspectRatio="none"
    style="cursor: {onPointClick ? 'pointer' : 'default'};"
    onpointermove={handlePointerMove}
    onpointerleave={handlePointerLeave}
    onpointerdown={handlePointerDown}
    onpointerup={handlePointerUp}
  >
    {#each [0, 0.25, 0.5, 0.75, 1] as p, i (i)}
      {@const y = padT + (height - padT - padB) * (1 - p)}
      <g>
        <line x1={padL} x2={w - padR} y1={y} y2={y} stroke="var(--border)" stroke-dasharray="2 4" />
        <text
          x={padL - 6}
          {y}
          text-anchor="end"
          font-size="10"
          fill="var(--text-faint)"
          font-family="var(--font-mono)">{compact(p * max)}</text
        >
      </g>
    {/each}
    {#if hover}
      <line
        x1={hover.x} x2={hover.x}
        y1={padT} y2={height - padB}
        stroke="var(--accent)" stroke-width="1" stroke-opacity="0.5"
        stroke-dasharray="3 3" pointer-events="none"
      />
    {/if}
    {#each visibleData as d, i (i)}
      {@const cx = padL + bw * i + bw / 2}
      {@const inH = (height - padT - padB) * (d.in / max)}
      {@const outH = (height - padT - padB) * (d.out / max)}
      {@const isHover = hover?.idx === i}
      <g>
        <rect
          x={cx - barW - 2}
          y={height - padB - inH}
          width={barW}
          height={inH}
          rx="3"
          fill="var(--positive)"
          opacity={hover !== null && !isHover ? 0.4 : 1}
        />
        <rect
          x={cx + 2}
          y={height - padB - outH}
          width={barW}
          height={outH}
          rx="3"
          fill="var(--negative)"
          opacity={hover !== null && !isHover ? 0.35 : 0.85}
        />
        <text
          x={cx}
          y={height - 8}
          text-anchor="middle"
          font-size="10"
          fill={isHover ? 'var(--text)' : 'var(--text-faint)'}
          font-family="var(--font-mono)">{d.m}</text
        >
      </g>
    {/each}
    {#if showBrush}
      <rect
        x={brushX} y={padT}
        width={brushW} height={height - padT - padB}
        fill="var(--accent)" fill-opacity="0.15"
        stroke="var(--accent)" stroke-opacity="0.5" stroke-width="1"
        pointer-events="none"
      />
    {/if}
  </svg>
  {#if hover}
    <div class="chart-tooltip" style="left: {tooltipLeft}%; top: {tooltipTop}%;">
      <span class="label">{hover.label}</span>
      <span class="value-row">
        <span class="dot pos"></span>
        <span>{fmtEurFull(hover.inVal)}</span>
      </span>
      <span class="value-row">
        <span class="dot neg"></span>
        <span>{fmtEurFull(hover.outVal)}</span>
      </span>
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    width: 100%;
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
    transform: translate(-50%, 0);
  }
  .chart-tooltip .label { color: var(--text-faint); font-size: 11px; display: block; margin-bottom: 2px; }
  .value-row {
    display: flex;
    align-items: center;
    gap: 5px;
    font-family: var(--font-mono);
    font-weight: 600;
    font-size: 12px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.pos { background: var(--positive); }
  .dot.neg { background: var(--negative); }
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
