<script lang="ts">
  interface Slice {
    name: string;
    v: number;
    color: string;
  }
  interface Props {
    data: Slice[];
    size?: number;
    thick?: number;
    hide?: boolean;
    /** Bindable: lets parent read/write hover index */
    hoverIdx?: number | null;
  }

  let { data, size = 180, thick = 26, hide = false, hoverIdx = $bindable(null) }: Props = $props();

  function fmtPlain(n: number): string {
    return n.toLocaleString('de-DE', { maximumFractionDigits: 0 });
  }
  function compact(n: number): string {
    if (Math.abs(n) >= 1000)
      return (n / 1000).toLocaleString('de-DE', { maximumFractionDigits: 1 }) + 'k';
    return fmtPlain(n);
  }

  const r = $derived(size / 2 - 4);
  const ir = $derived(r - thick);
  const cx = $derived(size / 2);
  const cy = $derived(size / 2);
  const total = $derived(data.reduce((s, d) => s + d.v, 0));

  const arcs = $derived.by(() => {
    let a = -Math.PI / 2;
    return data.map((d) => {
      const frac = total > 0 ? d.v / total : 0;
      const a2 = a + frac * Math.PI * 2;
      const large = frac > 0.5 ? 1 : 0;
      const x1 = cx + r * Math.cos(a);
      const y1 = cy + r * Math.sin(a);
      const x2 = cx + r * Math.cos(a2);
      const y2 = cy + r * Math.sin(a2);
      const x3 = cx + ir * Math.cos(a2);
      const y3 = cy + ir * Math.sin(a2);
      const x4 = cx + ir * Math.cos(a);
      const y4 = cy + ir * Math.sin(a);
      const path = `M ${x1} ${y1} A ${r} ${r} 0 ${large} 1 ${x2} ${y2} L ${x3} ${y3} A ${ir} ${ir} 0 ${large} 0 ${x4} ${y4} Z`;
      a = a2;
      return { path, color: d.color, frac, name: d.name, v: d.v };
    });
  });

  const centerLabel = $derived.by(() => {
    if (hoverIdx !== null && arcs[hoverIdx]) {
      return {
        name: arcs[hoverIdx].name,
        value: (hide ? '••••' : compact(arcs[hoverIdx].v)) + ' €',
        pct: `${(arcs[hoverIdx].frac * 100).toFixed(1)}%`,
      };
    }
    return {
      name: 'Total',
      value: (hide ? '••••' : compact(total)) + ' €',
      pct: null,
    };
  });
</script>

<div class="donut">
  <svg viewBox="0 0 {size} {size}" width="100%" style="height: auto; max-width: {size}px; flex-shrink: 0;">
    {#each arcs as a, i (i)}
      <path
        d={a.path}
        fill={a.color}
        style="cursor: pointer;
               opacity: {hoverIdx === null || hoverIdx === i ? 1 : 0.5};
               transition: opacity 120ms, transform 120ms;
               transform-origin: {cx}px {cy}px;
               transform: {hoverIdx === i ? 'scale(1.04)' : 'scale(1)'};"
        onpointerenter={() => hoverIdx = i}
        onpointerleave={() => hoverIdx = null}
      />
    {/each}
    <text x={cx} y={cy - 14} text-anchor="middle" font-size="11" fill="var(--text-faint)">{centerLabel.name}</text>
    <text x={cx} y={cy + 4} text-anchor="middle" font-size="13" font-weight="600" fill="var(--text)" font-family="var(--font-mono)">{centerLabel.value}</text>
    {#if centerLabel.pct}
      <text x={cx} y={cy + 18} text-anchor="middle" font-size="10" fill="var(--text-faint)">{centerLabel.pct}</text>
    {/if}
  </svg>
  <div class="legend">
    {#each arcs as a, i (i)}
      <div
        class="legend-row"
        class:hover={hoverIdx === i}
        onpointerenter={() => hoverIdx = i}
        onpointerleave={() => hoverIdx = null}
        role="button"
        tabindex="0"
      >
        <span class="swatch" style:background={a.color}></span>
        <span class="legend-name">{a.name}</span>
        <span class="num legend-val">{hide ? '••••' : fmtPlain(a.v) + ' €'}</span>
        <span class="num legend-pct">{(a.frac * 100).toFixed(0)}%</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .donut {
    display: flex;
    align-items: center;
    gap: 24px;
  }
  .legend {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .legend-row {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12.5px;
    border-radius: 4px;
    padding: 2px 4px;
    cursor: default;
    transition: background 120ms;
  }
  .legend-row.hover {
    background: var(--surface-2);
  }
  .swatch {
    width: 9px;
    height: 9px;
    border-radius: 2px;
  }
  .legend-name {
    flex: 1;
    color: var(--text-muted);
  }
  .legend-val {
    font-weight: 500;
  }
  .legend-pct {
    color: var(--text-faint);
    width: 38px;
    text-align: right;
  }
</style>
