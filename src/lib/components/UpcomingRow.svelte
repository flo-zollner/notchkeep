<script lang="ts">
  import { type Occurrence, type RecurringPayment } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import { settings, t } from '$lib/settings.svelte';

  interface Props {
    occurrence: Occurrence;
    recurring: RecurringPayment;
    onclick?: () => void;
  }
  let { occurrence, recurring, onclick }: Props = $props();

  const tr = $derived(t().recurring);

  function fmtDate(s: string): string {
    const d = new Date(s + 'T00:00:00');
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const tomorrow = new Date(today);
    tomorrow.setDate(today.getDate() + 1);
    const ds = settings.lang === 'en' ? 'en-GB' : 'de-DE';
    if (d.getTime() === today.getTime()) {
      return settings.lang === 'en' ? 'Today' : 'Heute';
    }
    if (d.getTime() === tomorrow.getTime()) {
      return settings.lang === 'en' ? 'Tomorrow' : 'Morgen';
    }
    return d.toLocaleDateString(ds, {
      weekday: 'short', day: '2-digit', month: '2-digit',
    });
  }

  const isPaid = $derived(occurrence.status === 'paid');
</script>

<button type="button" class="row" onclick={onclick}>
  <span class="date">{fmtDate(occurrence.dueDate)}</span>
  <span class="name">{recurring.name}</span>
  <span class="amount" class:neg={recurring.amountCents < 0}>
    {fmtEur(recurring.amountCents, { hide: settings.hide, signed: true, decimals: 2 })}
  </span>
  <span class="status" class:paid={isPaid}>
    {isPaid ? '✓ ' + tr.statusPaid : tr.statusPending}
  </span>
</button>

<style>
  .row {
    --date-col: 120px;
    --date-col-sm: 92px;
    --fs-row: 13px;
    --fs-status: 11px;
    display: grid;
    grid-template-columns: var(--date-col) 1fr auto auto;
    gap: 12px;
    align-items: center;
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: var(--fs-row);
    cursor: pointer;
    color: var(--text);
    text-align: left;
    font: inherit;
    width: 100%;
  }
  .row:hover { background: var(--surface-hover); }
  .date { color: var(--text-muted); }
  .name { font-weight: 500; }
  .amount { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }
  .amount.neg { color: var(--negative); }
  .status { font-size: var(--fs-status); padding: 4px 8px; border-radius: 4px;
            background: var(--surface-2); color: var(--text-muted); }
  .status.paid { color: var(--positive); background: var(--positive-soft, var(--surface-2)); }
  @media (max-width: 600px) {
    .row {
      grid-template-columns: var(--date-col-sm) minmax(0, 1fr) auto auto;
      gap: 8px;
    }
  }
</style>
