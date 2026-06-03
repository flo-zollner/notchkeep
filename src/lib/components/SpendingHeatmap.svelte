<script lang="ts">
  import { t } from '$lib/settings.svelte';

  interface Props {
    daily: number[];
    hide?: boolean;
    /** First weekday of the month (0=Mo, …, 6=Su). Default: 4 = May 2026 starts on Friday. */
    offset?: number;
    /** Displayed in each day's tooltip, e.g. "May 2026". */
    monthLabel?: string;
  }
  let { daily, hide = false, offset = 4, monthLabel = '' }: Props = $props();

  const max = $derived(daily.length > 0 ? Math.max(...daily) : 0);

  const cells = $derived.by(() => {
    const out: Array<{ empty: true } | { empty: false; day: number; v: number }> = [];
    for (let i = 0; i < offset; i++) out.push({ empty: true });
    daily.forEach((v, i) => out.push({ empty: false, day: i + 1, v }));
    return out;
  });

  function color(v: number): string {
    if (v === 0 || max === 0) return 'var(--surface-2)';
    const p = Math.min(1, v / max);
    return `oklch(from var(--accent) calc(l + 0.05 - 0.2 * ${p}) calc(c * ${0.6 + p * 0.4}) h)`;
  }

  function fmtEUR(v: number): string {
    if (hide) return '•• €';
    return v.toLocaleString('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + ' €';
  }

  const sum = $derived(daily.reduce((s, v) => s + v, 0));
</script>

<figure aria-describedby="heatmap-long-desc">
  <figcaption class="heatmap-caption">
    {monthLabel ? monthLabel + ': ' : ''}{hide ? '•• €' : fmtEUR(sum)}
  </figcaption>
  <div id="heatmap-long-desc" class="sr-only">
    {#if hide}
      {t().common.spent}: ••
    {:else}
      {t().common.spent}: {fmtEUR(sum)}.
      {#each daily as v, i (i)}
        {i + 1}.: {fmtEUR(v)}{i < daily.length - 1 ? ' ' : ''}
      {/each}
    {/if}
  </div>
  <div class="weekdays">
    {#each t().weekdays as w (w)}
      <div>{w}</div>
    {/each}
  </div>
  <div class="grid">
    {#each cells as c, i (i)}
      {#if c.empty}
        <div></div>
      {:else}
        <div
          class="cell"
          title={`${c.day}.${monthLabel ? ' ' + monthLabel : ''}: ${fmtEUR(c.v)}`}
          style:background={color(c.v)}
          style:color={c.v > max * 0.3 ? 'var(--accent-fg)' : 'var(--text-faint)'}
        >
          {c.day}
        </div>
      {/if}
    {/each}
  </div>
  <div class="footer">
    <span>{t().common.spent}: <span class="num muted">{fmtEUR(sum)}</span></span>
    <div class="legend">
      <span>—</span>
      {#each [0, 0.3, 0.6, 1] as p, i (i)}
        <span class="legend-cell" style:background={color(max * p)}></span>
      {/each}
      <span>+</span>
    </div>
  </div>
</figure>

<style>
  .heatmap-caption {
    font-size: 11px;
    color: var(--text-faint);
    margin-bottom: 4px;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  .weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 4px;
    font-size: 10px;
    color: var(--text-faint);
    margin-bottom: 4px;
    text-align: center;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 4px;
  }
  .cell {
    aspect-ratio: 1;
    border-radius: 4px;
    display: grid;
    place-items: center;
    font-size: 10px;
    font-family: var(--font-mono);
    font-weight: 500;
  }
  .footer {
    display: flex;
    justify-content: space-between;
    margin-top: 8px;
    font-size: 11px;
    color: var(--text-faint);
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .legend-cell {
    width: 8px;
    height: 8px;
    border-radius: 4px;
  }
  .muted {
    color: var(--text-muted);
  }
</style>
