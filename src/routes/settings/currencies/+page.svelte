<script lang="ts">
  import { goto } from '$app/navigation';
  import { listen } from '@tauri-apps/api/event';
  import { api, type CurrencyStatus, errMsg } from '$lib/api';
  import { parseEur } from '$lib/format';
  import AddCurrencyModal from '$lib/components/AddCurrencyModal.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { t } from '$lib/settings.svelte';

  const tc = $derived(t().currencies);

  let currencies = $state<CurrencyStatus[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let addOpen = $state(false);
  let refreshingAll = $state(false);

  let editingCode = $state<string | null>(null);
  let editInput = $state('');
  let editError = $state<string | null>(null);
  let savingEdit = $state(false);
  let refreshingCodes = $state<Set<string>>(new Set());
  // Per-row errors (e.g. Yahoo fetch failed) — shown below the rate cell
  // instead of the global top banner.
  let rowErrors = $state<Record<string, string>>({});

  $effect(() => { void load(); });

  // After an import (or startup refresh) the backend emits
  // 'price_refresh_status'. On 'completed' we refresh the list —
  // without a reload, new currencies from imports would only become visible
  // after a restart or manual navigation.
  $effect(() => {
    type Status = { stage: 'started' | 'completed' | 'failed' };
    const unlisten = listen<Status>('price_refresh_status', (e) => {
      if (e.payload.stage === 'completed') void load();
    });
    return () => { unlisten.then((u) => u()); };
  });

  async function load() {
    loading = true;
    error = null;
    try {
      currencies = await api.listCurrencies();
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  function startEdit(c: CurrencyStatus) {
    editingCode = c.code;
    editInput = c.rateMicro != null
      ? (c.rateMicro / 1_000_000).toString().replace('.', ',')
      : '';
    editError = null;
  }
  function cancelEdit() {
    editingCode = null;
    editInput = '';
    editError = null;
  }
  async function saveEdit(code: string) {
    editError = null;
    const n = parseEur(editInput);
    if (!Number.isFinite(n) || n <= 0) {
      editError = tc.errSaveFailed;
      return;
    }
    const micro = Math.round(n * 1_000_000);
    savingEdit = true;
    try {
      const updated = await api.updateCurrencyRate(code, micro);
      const idx = currencies.findIndex((c) => c.code === code);
      if (idx >= 0) currencies[idx] = updated;
      cancelEdit();
    } catch (e) {
      editError = `${tc.errSaveFailed}: ${errMsg(e)}`;
    } finally {
      savingEdit = false;
    }
  }

  async function refreshOne(code: string) {
    refreshingCodes.add(code);
    refreshingCodes = new Set(refreshingCodes);
    // Clear the old row error for this code
    const { [code]: _, ...rest } = rowErrors;
    rowErrors = rest;
    try {
      const updated = await api.refreshCurrencyRate(code);
      const idx = currencies.findIndex((c) => c.code === code);
      if (idx >= 0) currencies[idx] = updated;
    } catch (e) {
      rowErrors = { ...rowErrors, [code]: errMsg(e) };
    } finally {
      refreshingCodes.delete(code);
      refreshingCodes = new Set(refreshingCodes);
    }
  }
  async function refreshAll() {
    refreshingAll = true;
    error = null;
    rowErrors = {};
    try {
      const results = await Promise.allSettled(
        currencies.map((c) => api.refreshCurrencyRate(c.code))
      );
      // Extract per-row errors from allSettled results
      const newRowErrors: Record<string, string> = {};
      results.forEach((r, i) => {
        if (r.status === 'rejected') {
          newRowErrors[currencies[i].code] = errMsg(r.reason);
        }
      });
      rowErrors = newRowErrors;
      await load();
    } catch (e) {
      error = errMsg(e);
    } finally {
      refreshingAll = false;
    }
  }

  function onAdded(_status: CurrencyStatus) {
    void load();
  }

  function fmtRate(micro: number | null): string {
    if (micro == null) return '—';
    return (micro / 1_000_000).toFixed(6).replace('.', ',');
  }
</script>

<button class="btn back" onclick={() => goto('/settings')}>
  <Icon name="chevron-right" size={12} /> {t().nav.settings}
</button>

<div class="header">
  <h1>{tc.title}</h1>
  <div class="actions">
    <button class="btn ghost" onclick={refreshAll} disabled={refreshingAll || loading}>
      <Icon name="refresh" size={13} />
      {refreshingAll ? '…' : tc.refreshAllBtn}
    </button>
    <button class="btn primary" onclick={() => (addOpen = true)}>
      <Icon name="plus" size={13} /> {tc.addBtn}
    </button>
  </div>
</div>

{#if error}
  <div class="err-banner">{error}</div>
{/if}

<div class="card card-pad-lg">
  {#if loading}
    <div class="empty">…</div>
  {:else if currencies.length === 0}
    <div class="empty">{tc.noRateYet}</div>
  {:else}
    <table>
      <thead>
        <tr>
          <th>{tc.code}</th>
          <th>{tc.rate}</th>
          <th>{tc.lastUpdate}</th>
          <th>{tc.source}</th>
          <th class="right"></th>
        </tr>
      </thead>
      <tbody>
        {#each currencies as c (c.code)}
          <tr>
            <td class="mono">
              {c.code}
              {#if !c.inUse}<span class="badge">{tc.notInUseBadge}</span>{/if}
            </td>
            <td class="rate">
              {#if editingCode === c.code}
                <input
                  type="text"
                  inputmode="decimal"
                  bind:value={editInput}
                  onkeydown={(e) => {
                    if (e.key === 'Enter') void saveEdit(c.code);
                    if (e.key === 'Escape') cancelEdit();
                  }}
                />
                {#if editError}<small class="err">{editError}</small>{/if}
              {:else}
                <span class="mono">{fmtRate(c.rateMicro)}</span>
                {#if rowErrors[c.code]}
                  <small class="row-err">{rowErrors[c.code]}</small>
                {/if}
              {/if}
            </td>
            <td>{c.date ?? '—'}</td>
            <td>
              {#if c.source === 'manual'}{tc.sourceManual}
              {:else if c.source === 'yahoo'}{tc.sourceYahoo}
              {:else}—{/if}
            </td>
            <td class="actions-cell">
              {#if editingCode === c.code}
                <button class="iconbtn" onclick={() => saveEdit(c.code)} disabled={savingEdit} aria-label="save">
                  <Icon name="check" size={14} />
                </button>
                <button class="iconbtn" onclick={cancelEdit} disabled={savingEdit} aria-label="cancel">
                  <Icon name="x" size={14} />
                </button>
              {:else}
                <button
                  class="iconbtn"
                  onclick={() => startEdit(c)}
                  title={tc.editTooltip}
                  aria-label="edit"
                >
                  <Icon name="edit" size={14} />
                </button>
                <button
                  class="iconbtn"
                  onclick={() => refreshOne(c.code)}
                  disabled={refreshingCodes.has(c.code)}
                  title={tc.refreshTooltip}
                  aria-label="refresh"
                >
                  <Icon name="refresh" size={14} />
                </button>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if addOpen}
  <AddCurrencyModal onClose={() => (addOpen = false)} {onAdded} />
{/if}

<style>
  .btn.back { margin-bottom: 14px; transform: rotate(180deg); display: inline-flex; gap: 4px; }
  .header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; }
  .header h1 { margin: 0; font-size: 22px; }
  .actions { display: flex; gap: 8px; }
  .btn { padding: 6px 12px; border-radius: 6px; border: 0; cursor: pointer; font-size: 13px; display: inline-flex; align-items: center; gap: 4px; }
  .btn.primary { background: var(--accent, var(--positive)); color: white; }
  .btn.ghost { background: transparent; border: 1px solid var(--border); color: var(--text); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .err-banner {
    background: var(--negative-soft); color: var(--negative);
    padding: 10px 14px; border-radius: 6px; margin-bottom: 12px; font-size: 13px;
  }
  .empty { padding: 24px; text-align: center; color: var(--text-faint); }

  table { width: 100%; border-collapse: collapse; font-size: 13px; }
  th, td { padding: 8px 10px; border-bottom: 1px solid var(--border); text-align: left; }
  th { font-weight: 500; color: var(--text-faint); font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; }
  td.mono, .mono { font-family: ui-monospace, monospace; }
  .rate input { width: 130px; padding: 4px 6px; background: var(--surface-2); border: 1px solid var(--accent); border-radius: 4px; font-family: ui-monospace, monospace; }
  .rate .err { color: var(--negative); display: block; font-size: 11px; }
  .row-err { color: var(--negative); display: block; font-size: 11px; margin-top: 2px; }
  .actions-cell { display: flex; gap: 4px; justify-content: flex-end; }
  .iconbtn { background: transparent; border: 0; cursor: pointer; padding: 4px; border-radius: 4px; color: var(--text-muted); }
  .iconbtn:hover { color: var(--text); background: var(--surface-2); }
  .iconbtn:disabled { opacity: 0.4; cursor: not-allowed; }
  .badge {
    margin-left: 6px; padding: 1px 6px;
    background: var(--surface-2); color: var(--text-faint);
    border-radius: 999px; font-size: 10px;
  }
</style>
