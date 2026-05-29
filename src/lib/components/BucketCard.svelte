<script lang="ts">
  import Icon from './Icon.svelte';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';
  import { type Bucket, type BucketProgress } from '$lib/api';
  import { fmtEur } from '$lib/format';

  interface Props {
    bucket: Bucket;
    progress?: BucketProgress;
    securitiesValueCents?: number;
    trend?: number[];  // 6-month net cents
    onEdit: () => void;
    onAssign: () => void;
  }
  let { bucket, progress, securitiesValueCents = 0, trend, onEdit, onAssign }: Props = $props();

  const tb = $derived(t().buckets);

  const cashCents = $derived(progress?.currentCents ?? 0);
  const txCount = $derived(progress?.txCount ?? 0);
  const isOverspent = $derived(cashCents < 0);
  const target = $derived(bucket.targetCents);

  /** Progress bar is measured against cash saldo only. */
  const ratio = $derived(
    target && target > 0
      ? Math.max(0, Math.min(1, cashCents / target))
      : null,
  );

  /** Funded = target set AND cash saldo reaches target. */
  const isFunded = $derived(target !== null && target > 0 && cashCents >= target);

  const iconName = $derived(bucket.icon ?? 'wallet');
  const color = $derived(bucket.color ?? 'var(--accent)');

  const totalCents = $derived(cashCents + securitiesValueCents);
</script>

<!-- Card is a div to allow separate action buttons without nesting. -->
<div class="bucket-card" style:--accent={color}>
  <!-- Clickable area for edit — covers header, value, trend and progress -->
  <div class="card-body" role="button" tabindex="0" onclick={onEdit} onkeydown={(e) => e.key === 'Enter' && onEdit()}>
    <header>
      <span class="icon" style:color>
        <Icon name={iconName} size={16} />
      </span>
      <h4>{bucket.name}</h4>
      {#if bucket.archived}
        <span class="badge">{tb.archived}</span>
      {:else if isFunded}
        <span class="badge funded">✓ {tb.statusFunded ?? 'Funded'}</span>
      {:else if target !== null && target > 0}
        <span class="badge funding">{tb.statusFunding ?? 'Funding'}</span>
      {/if}
    </header>

    <!-- Primary value = cash saldo -->
    <div class="value num" class:overspent={isOverspent}>
      {fmtEur(cashCents, { hide: settings.hide, decimals: 2 })}
    </div>

    {#if isOverspent}
      <div class="overspent-note">
        {tb.overspentBy.replace('{amount}', fmtEur(-cashCents, { decimals: 2 }))}
      </div>
    {/if}

    {#if securitiesValueCents > 0}
      <div class="securities-row">
        <span class="in-sec">
          {tb.inSecurities ?? 'In Wertpapieren'}:
          {fmtEur(securitiesValueCents, { hide: settings.hide, decimals: eurDecimals() })}
        </span>
        <span class="sep">·</span>
        <span class="total-label">
          Gesamt {fmtEur(totalCents, { hide: settings.hide, decimals: eurDecimals() })}
        </span>
      </div>
    {/if}

    {#if trend && trend.length >= 2 && trend.some((v) => v !== 0)}
      {@const max = Math.max(...trend.map(Math.abs), 1)}
      <svg class="bucket-trend" viewBox="0 0 100 24" preserveAspectRatio="none" aria-hidden="true">
        {#each trend as v, i (i)}
          {@const x = (100 / Math.max(trend.length - 1, 1)) * i}
          {@const y = 12 - (v / max) * 10}
          {#if i > 0}
            {@const px = (100 / Math.max(trend.length - 1, 1)) * (i - 1)}
            {@const py = 12 - (trend[i - 1] / max) * 10}
            <line x1={px} y1={py} x2={x} y2={y}
              stroke={color} stroke-width="1.2" fill="none" opacity="0.6"
              vector-effect="non-scaling-stroke" />
          {/if}
          <circle cx={x} cy={y} r="1.3" fill={v >= 0 ? 'var(--positive)' : 'var(--negative)'} />
        {/each}
      </svg>
    {/if}

    {#if target && target > 0}
      <div
        class="bar"
        role="progressbar"
        aria-valuemin="0"
        aria-valuemax={target}
        aria-valuenow={cashCents}
        aria-valuetext={`${fmtEur(cashCents, { hide: settings.hide, decimals: eurDecimals() })} / ${fmtEur(target, { hide: settings.hide, decimals: eurDecimals() })}`}
      >
        <div class="fill" style:width={`${(ratio ?? 0) * 100}%`} style:background={color}></div>
      </div>
      <div class="meta num">
        {fmtEur(cashCents, { hide: settings.hide, decimals: eurDecimals() })}
        <span class="sep">/</span>
        <span class="target">{fmtEur(target, { hide: settings.hide, decimals: eurDecimals() })}</span>
      </div>
    {/if}

    <footer>
      <span class="tx-count">
        {txCount}
        {txCount === 1 ? 'Buchung' : 'Buchungen'}
      </span>
    </footer>
  </div>

  <!-- Card actions row — separate from card-body to avoid nested interactive elements -->
  <div class="card-actions">
    <button type="button" class="btn-assign" onclick={onAssign}>
      {tb.assign}
    </button>
    <button type="button" class="btn-edit" onclick={onEdit} aria-label={tb.edit ?? 'Bearbeiten'}>
      <Icon name="pencil" size={12} />
    </button>
  </div>
</div>

<style>
  .bucket-card {
    display: flex;
    flex-direction: column;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
  }
  .bucket-card:hover { border-color: var(--accent); }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px 10px;
    cursor: pointer;
    text-align: left;
    color: var(--text);
    outline: none;
    flex: 1;
  }
  .card-body:focus-visible { box-shadow: inset 0 0 0 2px var(--accent); }

  header { display: flex; align-items: center; gap: 10px; }
  .icon {
    width: 28px; height: 28px;
    border-radius: 8px;
    background: var(--surface-2);
    display: grid; place-items: center;
  }
  h4 { flex: 1; margin: 0; font-size: 14px; font-weight: 500; }
  .badge {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--surface);
    color: var(--text-muted);
  }
  .badge.funded {
    background: color-mix(in srgb, var(--positive) 15%, transparent);
    color: var(--positive);
  }
  .badge.funding {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
  }
  .value { font-size: 18px; }
  .value.overspent { color: var(--negative); }
  .overspent-note {
    font-size: 11px;
    color: var(--negative);
    margin-top: -4px;
  }
  .securities-row {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-muted);
    flex-wrap: wrap;
  }
  .in-sec { color: var(--accent); font-variant-numeric: tabular-nums; }
  .total-label { font-variant-numeric: tabular-nums; }
  .sep { color: var(--text-faint); }
  .bucket-trend { width: 100%; height: 24px; opacity: 0.9; }
  .bar {
    height: 8px;
    background: var(--surface);
    border-radius: 999px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 999px;
    transition: width .25s ease;
  }
  .meta { font-size: 12px; color: var(--text-muted); }
  .meta .sep { color: var(--text-faint); margin: 0 4px; }
  .meta .target { color: var(--text-muted); }
  footer {
    display: flex; justify-content: space-between; flex-wrap: wrap; gap: 4px 8px;
    font-size: 12px; color: var(--text-muted);
  }

  /* Action row at the bottom of the card */
  .card-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    background: var(--surface);
  }
  .btn-assign {
    flex: 1;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    border-radius: 6px;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    text-align: center;
  }
  .btn-assign:hover { background: color-mix(in srgb, var(--accent) 20%, transparent); }
  .btn-edit {
    padding: 6px 8px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .btn-edit:hover { border-color: var(--accent); color: var(--accent); }
</style>
