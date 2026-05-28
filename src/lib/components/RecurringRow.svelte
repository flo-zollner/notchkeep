<script lang="ts">
  import Icon from './Icon.svelte';
  import { type RecurringPayment } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import { settings, t } from '$lib/settings.svelte';

  interface Props {
    recurring: RecurringPayment;
    onEdit?: (r: RecurringPayment) => void;
    onArchiveToggle?: (r: RecurringPayment) => void;
  }
  let { recurring, onEdit, onArchiveToggle }: Props = $props();

  const tr = $derived(t().recurring);

  const freqLabel = $derived.by(() => {
    switch (recurring.frequency) {
      case 'weekly': return tr.weekly;
      case 'monthly': return tr.monthly;
      case 'quarterly': return tr.quarterly;
      case 'yearly': return tr.yearly;
    }
  });
</script>

<div class="row" class:archived={recurring.archived}>
  <span class="name">{recurring.name}</span>
  <span class="cp">{recurring.counterparty ?? '—'}</span>
  <span class="amount" class:neg={recurring.amountCents < 0}>
    {fmtEur(recurring.amountCents, { hide: settings.hide, signed: true, decimals: 2 })}
  </span>
  <span class="freq">{freqLabel}</span>
  <span class="anchor">{recurring.anchorDate}</span>
  <span class="mobile-meta">{freqLabel} · {recurring.anchorDate}</span>
  <div class="actions">
    <button type="button" onclick={() => onEdit?.(recurring)} title={t().common.edit}>
      <Icon name="pencil" size={12} />
    </button>
    <button type="button" onclick={() => onArchiveToggle?.(recurring)} title={recurring.archived ? tr.unarchive : tr.archive}>
      <Icon name="x" size={12} />
    </button>
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) 110px 110px 110px auto;
    gap: 10px;
    align-items: center;
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
  }
  .row.archived { opacity: 0.5; }
  /* min-width:0 lets the flexible columns shrink below their text's
     intrinsic width (long names) instead of forcing the row to overflow
     in the narrow tablet band where the Rail eats horizontal space. */
  .name { font-weight: 500; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cp { color: var(--text-muted); font-size: 12px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .amount { font-variant-numeric: tabular-nums; text-align: right; }
  .amount.neg { color: var(--negative); }
  .freq, .anchor { color: var(--text-muted); font-size: 11px; }
  .mobile-meta { display: none; }
  .actions { display: flex; gap: 4px; }
  .actions button {
    background: transparent; border: 0; color: var(--text-muted);
    padding: 4px; cursor: pointer; border-radius: 4px;
  }
  .actions button:hover { color: var(--text); background: var(--surface-2); }

  @media (max-width: 599px) {
    .row {
      grid-template-columns: 1fr auto;
      grid-template-rows: auto auto auto;
      gap: 2px 8px;
      padding: 10px 10px;
    }
    .name {
      grid-column: 1 / 2;
      grid-row: 1;
    }
    .amount {
      grid-column: 2 / 3;
      grid-row: 1;
      align-self: start;
    }
    .cp {
      grid-column: 1 / 2;
      grid-row: 2;
      font-size: 11px;
    }
    .actions {
      grid-column: 2 / 3;
      grid-row: 2;
      align-self: end;
      justify-content: flex-end;
    }
    .freq, .anchor { display: none; }
    .mobile-meta {
      display: block;
      grid-column: 1 / -1;
      grid-row: 3;
      font-size: 11px;
      color: var(--text-muted);
      margin-top: 2px;
    }
  }
</style>
