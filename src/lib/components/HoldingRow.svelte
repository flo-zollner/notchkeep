<script lang="ts">
  import { type Holding } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';

  interface Props {
    holding: Holding;
    onclick?: (id: number) => void;
  }
  let { holding, onclick }: Props = $props();

  const tp = $derived(t().portfolio);

  function fmtShares(micro: number): string {
    const n = micro / 1_000_000;
    return new Intl.NumberFormat(settings.lang === 'en' ? 'en' : 'de', {
      minimumFractionDigits: 0,
      maximumFractionDigits: 4,
    }).format(n);
  }

  function unrealizedPct(holding: Holding): number {
    if (holding.costBasisCents === 0) return 0;
    return (holding.unrealizedCents / holding.costBasisCents) * 100;
  }
</script>

<button
  type="button"
  class="row"
  onclick={() => onclick?.(holding.securityId)}
>
  <div class="left">
    <strong>{holding.name}</strong>
    <small class="muted">
      {holding.isin}{#if holding.symbol} · {holding.symbol}{/if}
      {#if holding.lastPriceDate}
        · {tp.lastUpdate.replace('{date}', holding.lastPriceDate)}
      {:else}
        · {tp.pricesNotSet}
      {/if}
    </small>
  </div>
  <div class="cells">
    <div class="cell">
      <span class="lbl">{tp.shares}</span>
      <span class="num">{fmtShares(holding.sharesMicro)}</span>
    </div>
    <div class="cell">
      <span class="lbl">{tp.kpiMarketValue}</span>
      <span class="num">{fmtEur(holding.marketValueCents, { hide: settings.hide, decimals: eurDecimals() })}</span>
    </div>
    <div class="cell">
      <span class="lbl">{tp.kpiUnrealized}</span>
      <span class="num" class:pos={holding.unrealizedCents > 0} class:neg={holding.unrealizedCents < 0}>
        {fmtEur(holding.unrealizedCents, { hide: settings.hide, signed: true, decimals: eurDecimals() })}
        {#if holding.unrealizedCents !== 0}
          ({unrealizedPct(holding).toFixed(1)}%)
        {/if}
      </span>
    </div>
  </div>
</button>

<style>
  .row {
    width: 100%;
    text-align: left;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 12px 14px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    cursor: pointer;
    color: var(--text);
    font: inherit;
    transition: background 0.1s, border-color 0.1s;
  }
  .row:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
  }
  .left {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .left strong {
    font-size: 13px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted {
    color: var(--text-muted);
    font-size: 11px;
  }
  .cells {
    display: flex;
    gap: 24px;
  }
  .cell {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    min-width: 70px;
  }
  .lbl {
    font-size: 10px;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .num {
    font-size: 13px;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .pos { color: var(--positive); }
  .neg { color: var(--negative); }

  @media (max-width: 599px) {
    .row {
      align-items: start;
      gap: 8px;
      padding: 10px 10px;
    }
    .left {
      flex: 1 1 0;
      min-width: 0;
    }
    .cells {
      flex-direction: column;
      gap: 4px;
      align-items: flex-end;
      flex-shrink: 0;
    }
    .cell {
      min-width: 0;
      gap: 0;
    }
    .lbl { display: none; }
    /* hide shares cell on mobile, show only market-value + unrealized */
    .cell:first-child { display: none; }
    .num { font-size: 12px; }
  }
</style>
