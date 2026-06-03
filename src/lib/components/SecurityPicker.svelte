<script lang="ts">
  import Icon from './Icon.svelte';
  import SecurityForm from './SecurityForm.svelte';
  import { t } from '$lib/settings.svelte';
  import { api, type Security } from '$lib/api';

  interface Props {
    value: Security | null;
    onSelect: (security: Security) => void;
  }
  let { value, onSelect }: Props = $props();

  const tt = $derived(t().trade);

  let searchOpen = $state(false);
  let query = $state('');
  let securities = $state<Security[]>([]);
  let loading = $state(false);
  let addingNew = $state(false);

  $effect(() => {
    if (searchOpen) void load();
  });

  async function load() {
    loading = true;
    try {
      securities = await api.listSecurities(false);
    } finally {
      loading = false;
    }
  }

  const filtered = $derived(() => {
    const q = query.trim().toLowerCase();
    if (!q) return securities;
    return securities.filter((s) =>
      s.name.toLowerCase().includes(q) ||
      s.isin.toLowerCase().includes(q) ||
      (s.symbol?.toLowerCase().includes(q) ?? false),
    );
  });

  function pick(s: Security) {
    onSelect(s);
    searchOpen = false;
    query = '';
  }

  function onCreated() {
    addingNew = false;
    void load();
  }
</script>

<div class="picker">
  {#if value}
    <button type="button" class="selected" onclick={() => (searchOpen = !searchOpen)}>
      <strong>{value.name}</strong>
      <small class="muted">{value.isin}</small>
    </button>
  {:else}
    <button type="button" class="placeholder" onclick={() => (searchOpen = !searchOpen)}>
      {tt.pickSecurity}
    </button>
  {/if}

  {#if searchOpen}
    <div class="dropdown">
      <input
        type="text"
        placeholder={tt.pickSecurityHint}
        bind:value={query}
      />
      {#if loading}
        <p class="muted">…</p>
      {:else}
        <ul>
          {#each filtered() as s (s.id)}
            <li>
              <button type="button" onclick={() => pick(s)}>
                <strong>{s.name}</strong>
                <small class="muted">{s.isin} · {s.symbol ?? '—'}</small>
              </button>
            </li>
          {/each}
          {#if filtered().length === 0}
            <li class="empty"><span class="muted">—</span></li>
          {/if}
        </ul>
      {/if}
      <button type="button" class="add-new" onclick={() => (addingNew = true)}>
        <Icon name="plus" size={12} /> {tt.newSecurity}
      </button>
    </div>
  {/if}

  {#if addingNew}
    <SecurityForm
      security={null}
      onClose={() => (addingNew = false)}
      onSaved={onCreated}
    />
  {/if}
</div>

<style>
  .picker { position: relative; }
  .picker button.selected, .picker button.placeholder {
    width: 100%; text-align: left;
    padding: 8px;
    border: 1px solid var(--border); border-radius: 8px;
    background: var(--surface-2); cursor: pointer; color: var(--text);
    font: inherit;
  }
  .picker button.selected:hover, .picker button.placeholder:hover {
    background: var(--surface-hover);
  }
  .picker button.placeholder { color: var(--text-muted); }
  .picker .selected strong { display: block; font-size: 13px; }
  .picker .selected small { display: block; font-size: 11px; color: var(--text-muted); margin-top: 4px; }
  .dropdown {
    position: absolute; top: 100%; left: 0; right: 0;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: var(--r-md); padding: 8px; z-index: 20;
    max-height: 280px; overflow: auto;
    margin-top: 4px;
    box-shadow: var(--shadow-md);
  }
  .dropdown input {
    width: 100%; box-sizing: border-box; padding: 8px;
    border: 1px solid var(--border); border-radius: 8px;
    background: var(--surface-2); color: var(--text);
    margin-bottom: 8px;
    font: inherit;
  }
  .dropdown input:focus { outline: none; border-color: var(--accent); }
  .dropdown ul { list-style: none; padding: 0; margin: 0 0 8px; }
  .dropdown li button {
    width: 100%; text-align: left;
    padding: 8px; background: transparent; border: none;
    cursor: pointer; color: var(--text);
    border-radius: var(--r-sm);
    font: inherit;
  }
  .dropdown li button:hover { background: var(--surface-hover); }
  .dropdown li button strong { display: block; font-size: 13px; }
  .dropdown li button small { display: block; font-size: 11px; color: var(--text-muted); margin-top: 4px; }
  .dropdown li.empty { padding: 8px; text-align: center; color: var(--text-muted); }
  .add-new {
    width: 100%;
    padding: 8px;
    background: transparent;
    border: 1px dashed var(--border); border-radius: 8px;
    cursor: pointer; color: var(--accent);
    display: flex; align-items: center; justify-content: center; gap: 8px;
    font-size: 12px;
    font: inherit;
  }
  .add-new:hover { background: var(--accent-soft); border-color: var(--accent); }
</style>
