<script lang="ts">
  import {
    api,
    type DetectedRecurring, type Account, errMsg} from '$lib/api';
  import { fmtEur } from '$lib/format';
  import { settings, t } from '$lib/settings.svelte';
  import Sheet from './Sheet.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    accounts: Account[];
    onClose: () => void;
    onCreated: () => void;
  }
  let { accounts, onClose, onCreated }: Props = $props();

  let suggestions = $state<DetectedRecurring[]>([]);
  let selected = $state<Set<number>>(new Set());
  let loading = $state(true);
  let saving = $state(false);
  let error = $state<string | null>(null);

  const tr = $derived(t().recurring);

  function accountName(id: number): string {
    return accounts.find((a) => a.id === id)?.name ?? '?';
  }

  function freqLabel(f: string): string {
    switch (f) {
      case 'weekly': return tr.weekly;
      case 'monthly': return tr.monthly;
      case 'quarterly': return tr.quarterly;
      case 'yearly': return tr.yearly;
      default: return f;
    }
  }

  async function load() {
    loading = true;
    try {
      suggestions = await api.detectRecurring();
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }
  load();

  function toggle(i: number) {
    if (selected.has(i)) {
      selected.delete(i);
    } else {
      selected.add(i);
    }
    selected = new Set(selected);
  }

  async function applySelected() {
    saving = true;
    error = null;
    try {
      for (const i of selected) {
        const s = suggestions[i];
        await api.createRecurring({
          name: s.counterparty,
          accountId: s.accountId,
          categoryId: null,
          amountCents: s.amountCents,
          frequency: s.frequency,
          anchorDate: s.anchorDate,
          counterparty: s.counterparty,
          note: null,
        });
      }
      onCreated();
    } catch (e) {
      error = errMsg(e);
    } finally {
      saving = false;
    }
  }


</script>

<Sheet open={true} {onClose} title={tr.detectTitle}>
  {#snippet footer()}
    <div class="footer-actions">
      <button type="button" onclick={onClose}>{t().common.cancel}</button>
      <button type="button" class="primary" disabled={saving || selected.size === 0} onclick={applySelected}>
        {tr.detectAccept} ({selected.size})
      </button>
    </div>
  {/snippet}

  {#if loading}
    <p class="muted">…</p>
  {:else if suggestions.length === 0}
    <p class="muted">{tr.detectEmpty}</p>
  {:else}
    <ul class="list">
      {#each suggestions as s, i (i)}
        <li>
          <input type="checkbox" checked={selected.has(i)} onchange={() => toggle(i)} />
          <span class="cp">{s.counterparty}</span>
          <span class="acc">{accountName(s.accountId)}</span>
          <span class="amt" class:neg={s.amountCents < 0}>
            {fmtEur(s.amountCents, { hide: settings.hide, signed: true, decimals: 2 })}
          </span>
          <span class="freq">{freqLabel(s.frequency)}</span>
          <span class="samples">{tr.detectSamples.replace('{n}', String(s.sampleCount))}</span>
        </li>
      {/each}
    </ul>
  {/if}

  <p class="err" aria-live="polite">{#if error}<Icon name="warning" size={14} aria-hidden="true" /> {error}{/if}</p>
</Sheet>

<style>
  .muted { color: var(--text-muted); font-size: 13px; }
  .list { list-style: none; padding: 0; margin: 0; display: grid; gap: 4px; max-height: 60vh; overflow-y: auto; }
  .list li {
    display: grid;
    grid-template-columns: 20px 1fr 120px 120px 100px 70px;
    gap: 8px;
    padding: 8px 12px;
    align-items: center;
    background: var(--surface-2);
    border-radius: 6px;
    font-size: 12px;
  }
  .cp { font-weight: 500; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .acc, .freq, .samples { color: var(--text-muted); min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .amt { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .amt.neg { color: var(--negative); }
  .err { display: flex; align-items: center; gap: 6px; color: var(--negative); font-size: 12px; margin: 0; }
  .err:empty { display: none; }
  @media (max-width: 599px) {
    .list li {
      grid-template-columns: 20px 1fr auto;
      grid-template-rows: auto auto;
    }
    .list li input[type="checkbox"] { grid-row: 1; grid-column: 1; }
    .cp { grid-row: 1; grid-column: 2; }
    .amt { grid-row: 1; grid-column: 3; }
    .acc { grid-row: 2; grid-column: 2; }
    .freq { grid-row: 2; grid-column: 2; display: none; }
    .samples { grid-row: 2; grid-column: 3; text-align: right; }
  }
  .footer-actions { display: flex; gap: 8px; justify-content: flex-end; flex-wrap: nowrap; }
  @media (max-width: 600px) {
    .footer-actions button { flex: 1 1 0; min-width: 0; }
  }
  .footer-actions button {
    background: var(--surface-2); border: 1px solid var(--border);
    border-radius: 6px; padding: 6px 12px; cursor: pointer;
    font: inherit; color: var(--text);
  }
  .footer-actions .primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); }
  .footer-actions .primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
