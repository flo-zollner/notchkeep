<script lang="ts">
  import Donut from './Donut.svelte';
  import { type AllocationSlice } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import { settings, eurDecimals } from '$lib/settings.svelte';

  interface Props {
    title: string;
    slices: AllocationSlice[];
    size?: number;
    maxSlices?: number;
  }
  let { title, slices, size = 180, maxSlices = 6 }: Props = $props();

  const PALETTE = [
    '#10b981', '#0ea5e9', '#6366f1', '#f59e0b',
    '#ef4444', '#a855f7', '#06b6d4', '#84cc16',
    '#fb7185', '#94a3b8',
  ];

  const groupedSlices = $derived.by(() => {
    if (slices.length <= maxSlices) {
      return slices.map((s, i) => ({
        name: s.key,
        v: s.valueCents,
        color: PALETTE[i % PALETTE.length],
      }));
    }
    const top = slices.slice(0, maxSlices - 1);
    const rest = slices.slice(maxSlices - 1);
    const restSum = rest.reduce((a, b) => a + b.valueCents, 0);
    const out = top.map((s, i) => ({
      name: s.key,
      v: s.valueCents,
      color: PALETTE[i % PALETTE.length],
    }));
    if (restSum > 0) {
      out.push({
        name: 'Sonstige',
        v: restSum,
        color: PALETTE[PALETTE.length - 1],
      });
    }
    return out;
  });

  const total = $derived(groupedSlices.reduce((a, b) => a + b.v, 0));

  // Donut expects euro values; groupedSlices carry cents (for fmtEur in the legend below).
  const donutData = $derived(
    groupedSlices.map((s) => ({ name: s.name, v: s.v / 100, color: s.color })),
  );

  let hoverIdx = $state<number | null>(null);
</script>

<div class="card">
  <h4>{title}</h4>
  {#if total === 0}
    <p class="muted">—</p>
  {:else}
    <div class="row">
      <Donut data={donutData} {size} hide={settings.hide} legend={false} bind:hoverIdx />
      <ul class="legend">
        {#each groupedSlices as s, i (s.name)}
          <li
            class:hover={hoverIdx === i}
            onpointerenter={() => hoverIdx = i}
            onpointerleave={() => hoverIdx = null}
          >
            <span class="dot" style:background={s.color}></span>
            <span class="name">{s.name}</span>
            <span class="val">{fmtEur(s.v, { hide: settings.hide, decimals: eurDecimals() })}</span>
            <span class="pct">{((s.v / total) * 100).toFixed(1)}%</span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px;
  }
  h4 {
    margin: 0 0 10px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }
  .muted { color: var(--text-faint); font-size: 12px; }
  .row {
    display: flex;
    gap: 14px;
    align-items: center;
  }
  .legend {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }
  .legend li {
    display: grid;
    grid-template-columns: 12px 1fr auto auto;
    gap: 8px;
    align-items: center;
    font-size: 11px;
    color: var(--text);
    border-radius: 4px;
    padding: 2px 4px;
    cursor: default;
    transition: background 120ms;
  }
  .legend li.hover {
    background: var(--surface-2);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .val, .pct {
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }
</style>
