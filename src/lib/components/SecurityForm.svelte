<script lang="ts">
  import Icon from './Icon.svelte';
  import BreakdownEditor from './BreakdownEditor.svelte';
  import { t } from '$lib/settings.svelte';
  import {
    api,
    type Security,
    type AssetType,
    type NewSecurityPayload,
    type UpdateSecurityPayload,
    type BreakdownDimension,
    type BreakdownRowInput,
  } from '$lib/api';

  interface Props {
    security: Security | null;
    onClose: () => void;
    onSaved: () => void;
    onDeleted?: (id: number) => void;
  }
  let { security, onClose, onSaved, onDeleted }: Props = $props();

  const ts = $derived(t().security);
  const tb = $derived(t().breakdown);

  const isEdit = $derived(security !== null);

  const ASSET_TYPES: AssetType[] = [
    'stock', 'etf_equity', 'etf_bond', 'etf_reit', 'bond', 'crypto', 'other',
  ];

  // Form state is initialised from the `security` prop once at mount. The
  // parent unmounts/remounts via `{#if}` for each open, so prop-change
  // reactivity is not desired here.
  /* svelte-ignore state_referenced_locally */
  let isin = $state(security?.isin ?? '');
  /* svelte-ignore state_referenced_locally */
  let symbol = $state(security?.symbol ?? '');
  /* svelte-ignore state_referenced_locally */
  let name = $state(security?.name ?? '');
  /* svelte-ignore state_referenced_locally */
  let currency = $state(security?.currency ?? 'EUR');
  /* svelte-ignore state_referenced_locally */
  let assetType = $state<AssetType>((security?.assetType as AssetType) ?? 'etf_equity');
  /* svelte-ignore state_referenced_locally */
  let country = $state(security?.country ?? '');
  /* svelte-ignore state_referenced_locally */
  let sector = $state(security?.sector ?? '');
  /* svelte-ignore state_referenced_locally */
  let note = $state(security?.note ?? '');
  /* svelte-ignore state_referenced_locally */
  let archived = $state(security?.archived ?? false);

  let countryBreakdown = $state<BreakdownRowInput[]>([]);
  let sectorBreakdown = $state<BreakdownRowInput[]>([]);

  let activeTab = $state<'main' | BreakdownDimension>('main');
  let saving = $state(false);
  let error = $state<string | null>(null);
  let confirmingDelete = $state(false);

  const isEtf = $derived(assetType.startsWith('etf_'));

  $effect(() => {
    if (security) {
      void loadBreakdowns(security.id);
    }
  });

  async function loadBreakdowns(id: number) {
    try {
      const [c, s] = await Promise.all([
        api.getBreakdown(id, 'country'),
        api.getBreakdown(id, 'sector'),
      ]);
      countryBreakdown = c.map((r) => ({ key: r.key, weightBps: r.weightBps }));
      sectorBreakdown = s.map((r) => ({ key: r.key, weightBps: r.weightBps }));
    } catch (e) {
      error = (e as Error).message ?? String(e);
    }
  }

  function isinLooksOk(raw: string): boolean {
    const cleaned = raw.replace(/\s/g, '').toUpperCase();
    return /^[A-Z]{2}[A-Z0-9]{9}[0-9]$/.test(cleaned);
  }

  async function save() {
    error = null;
    if (!name.trim()) { error = ts.errNameRequired; return; }
    if (!isinLooksOk(isin)) { error = ts.errIsinInvalid; return; }
    if (!ASSET_TYPES.includes(assetType)) { error = ts.errAssetTypeInvalid; return; }

    saving = true;
    try {
      let savedId: number;
      if (isEdit && security) {
        const payload: UpdateSecurityPayload = {
          isin: isin.replace(/\s/g, '').toUpperCase(),
          symbol: symbol.trim() || null,
          name: name.trim(),
          currency: currency.trim().toUpperCase() || 'EUR',
          assetType,
          country: country.trim() || null,
          sector: sector.trim() || null,
          note: note.trim() || null,
          archived,
        };
        const updated = await api.updateSecurity(security.id, payload);
        savedId = updated.id;
      } else {
        const payload: NewSecurityPayload = {
          isin: isin.replace(/\s/g, '').toUpperCase(),
          symbol: symbol.trim() || null,
          name: name.trim(),
          currency: currency.trim().toUpperCase() || null,
          assetType,
          country: country.trim() || null,
          sector: sector.trim() || null,
          note: note.trim() || null,
        };
        const created = await api.createSecurity(payload);
        savedId = created.id;
      }
      if (isEtf) {
        await api.setBreakdown(savedId, 'country', countryBreakdown);
        await api.setBreakdown(savedId, 'sector', sectorBreakdown);
      }
      onSaved();
      onClose();
    } catch (e) {
      error = (e as Error).message ?? String(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!security) return;
    saving = true;
    try {
      await api.deleteSecurity(security.id);
      onDeleted?.(security.id);
      onSaved();
      onClose();
    } catch (e) {
      error = (e as Error).message ?? String(e);
      confirmingDelete = false;
    } finally {
      saving = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onClose()} />

<div class="overlay">
  <div class="backdrop" role="presentation" onclick={onClose}></div>
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="sec-form-title" tabindex="-1">
    <header>
      <h2 id="sec-form-title">{isEdit ? ts.edit : t().portfolio.newSecurity}</h2>
      <button type="button" class="icon" aria-label="Schließen" onclick={onClose}>
        <Icon name="x" size={16} />
      </button>
    </header>

    <nav class="tabs">
      <button
        type="button"
        class:active={activeTab === 'main'}
        onclick={() => (activeTab = 'main')}
      >{ts.name}</button>
      {#if isEtf}
        <button
          type="button"
          class:active={activeTab === 'country'}
          onclick={() => (activeTab = 'country')}
        >{tb.country}</button>
        <button
          type="button"
          class:active={activeTab === 'sector'}
          onclick={() => (activeTab = 'sector')}
        >{tb.sector}</button>
      {/if}
    </nav>

    {#if activeTab === 'main'}
      <div class="grid">
        <label>{ts.isin}<input type="text" bind:value={isin} maxlength="14" /></label>
        <label>{ts.symbol}<input type="text" bind:value={symbol} /></label>
        <label class="span2">{ts.name}<input type="text" bind:value={name} /></label>
        <label>{ts.currency}<input type="text" bind:value={currency} maxlength="3" /></label>
        <label>{ts.assetType}
          <select bind:value={assetType}>
            {#each ASSET_TYPES as at (at)}
              <option value={at}>{ts.types[at]}</option>
            {/each}
          </select>
        </label>
        <label>{ts.country}<input type="text" bind:value={country} placeholder="DE / US / …" /></label>
        <label>{ts.sector}<input type="text" bind:value={sector} placeholder="Technology / …" /></label>
        <label class="span2">{ts.note}<textarea bind:value={note} rows="2"></textarea></label>
        {#if isEdit}
          <label class="span2 toggle">
            <input type="checkbox" bind:checked={archived} />
            <span>{ts.archived}</span>
          </label>
        {/if}
      </div>
    {:else if activeTab === 'country'}
      <BreakdownEditor
        rows={countryBreakdown}
        onChange={(r) => (countryBreakdown = r)}
      />
    {:else if activeTab === 'sector'}
      <BreakdownEditor
        rows={sectorBreakdown}
        onChange={(r) => (sectorBreakdown = r)}
      />
    {/if}

    <p class="err" aria-live="polite">{#if error}<Icon name="warning" size={14} aria-hidden="true" /> {error}{/if}</p>

    <footer>
      {#if isEdit}
        {#if confirmingDelete}
          <button type="button" class="danger" disabled={saving} onclick={remove}>
            {ts.confirmDelete}
          </button>
          <button type="button" onclick={() => (confirmingDelete = false)}>{ts.cancel}</button>
        {:else}
          <button type="button" class="danger" onclick={() => (confirmingDelete = true)}>
            {ts.delete}
          </button>
        {/if}
      {/if}
      <span style="flex:1"></span>
      <button type="button" onclick={onClose}>{ts.cancel}</button>
      <button type="button" class="primary" disabled={saving} onclick={save}>
        {ts.save}
      </button>
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0;
    display: grid; place-items: center;
    z-index: 100;
  }
  .backdrop {
    position: absolute; inset: 0;
    background: var(--scrim);
    border: 0; padding: 0; margin: 0;
    cursor: pointer;
  }
  .modal {
    position: relative;
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--border);
    padding: 18px;
    border-radius: var(--r-lg);
    width: min(520px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow: auto;
    box-shadow: var(--shadow-lg);
  }
  header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 14px; }
  header h2 { margin: 0; font-size: 16px; color: var(--text); }
  header button.icon {
    background: transparent; border: none; cursor: pointer;
    color: var(--text-muted);
    padding: 4px;
    border-radius: var(--r-sm);
  }
  header button.icon:hover { background: var(--surface-hover); color: var(--text); }
  .tabs { display: flex; gap: 4px; margin-bottom: 14px; border-bottom: 1px solid var(--border); }
  .tabs button {
    background: transparent; border: none; padding: 8px 12px;
    cursor: pointer; color: var(--text-muted);
    border-bottom: 2px solid transparent;
    font-size: 13px;
  }
  .tabs button:hover { color: var(--text); }
  .tabs button.active { color: var(--text); border-bottom-color: var(--accent); }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px 12px;
  }
  .grid label { display: flex; flex-direction: column; font-size: 12px; gap: 4px; color: var(--text-muted); }
  .grid input, .grid select, .grid textarea {
    padding: 8px 10px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--surface-2); color: var(--text);
    font: inherit;
  }
  .grid input:focus, .grid select:focus, .grid textarea:focus {
    outline: none; border-color: var(--accent);
  }
  .grid .span2 { grid-column: 1 / -1; }
  .grid .toggle { flex-direction: row; align-items: center; gap: 8px; color: var(--text); }
  .err { display: flex; align-items: center; gap: 6px; color: var(--negative); font-size: 12px; margin: 8px 0 0; }
  .err:empty { display: none; }
  footer {
    display: flex; gap: 8px; margin-top: 14px;
    align-items: center;
  }
  footer button {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 8px 12px;
    border-radius: 8px;
    cursor: pointer;
    font: inherit;
  }
  footer button:hover { background: var(--surface-hover); }
  footer button.primary {
    background: var(--accent); color: var(--accent-fg); border: 0;
  }
  footer button.primary:hover { background: var(--accent-hover); }
  footer button.primary:disabled { opacity: 0.5; cursor: not-allowed; }
  footer button.danger {
    background: var(--surface-2);
    color: var(--negative);
    border: 1px solid var(--border);
  }
  footer button.danger:hover { background: var(--negative-soft); }
</style>
