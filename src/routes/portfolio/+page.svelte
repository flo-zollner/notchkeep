<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import SecurityForm from '$lib/components/SecurityForm.svelte';
  import TradeModal from '$lib/components/TradeModal.svelte';
  import HoldingRow from '$lib/components/HoldingRow.svelte';
  import KursRefreshButton from '$lib/components/KursRefreshButton.svelte';
  import PerformanceChart from '$lib/components/PerformanceChart.svelte';
  import DividendHistoryList from '$lib/components/DividendHistoryList.svelte';
  import AllocationDonut from '$lib/components/AllocationDonut.svelte';
  import { goto } from '$app/navigation';
  import { listen } from '@tauri-apps/api/event';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';
  import {
    api,
    type Security, type Holding, type DividendEntry, type PortfolioKpis,
    type AllocationSlice,
  } from '$lib/api';
  import { fmtEur } from '$lib/format';

  type Tab = 'positions' | 'dividends' | 'allocation' | 'securities';
  let activeTab = $state<Tab>('positions');

  // Securities (for the "Securities" tab and editing)
  let securities = $state<Security[]>([]);
  let showArchived = $state(false);
  let editing = $state<Security | null>(null);
  let addingNew = $state(false);
  let addingTrade = $state(false);

  // Portfolio data
  let kpis = $state<PortfolioKpis | null>(null);
  let holdings = $state<Holding[]>([]);
  let dividends = $state<DividendEntry[]>([]);
  let loading = $state(true);

  // Allocation data (lazy)
  let allocAssetType = $state<AllocationSlice[]>([]);
  let allocCountry = $state<AllocationSlice[]>([]);
  let allocSector = $state<AllocationSlice[]>([]);
  let allocLoaded = $state(false);

  const tp = $derived(t().portfolio);
  const ts = $derived(t().security);

  const now = new Date();
  const currentYear = now.getFullYear();

  async function loadAll() {
    loading = true;
    try {
      const [k, h, d, s] = await Promise.all([
        api.portfolioKpis(currentYear),
        api.listHoldings(),
        api.dividendHistory(),
        api.listSecurities(showArchived),
      ]);
      kpis = k;
      holdings = h;
      dividends = d;
      securities = s;
    } finally {
      loading = false;
    }
    allocLoaded = false;
  }

  $effect(() => {
    void showArchived;
    void loadAll();
  });

  async function loadAllocations() {
    if (allocLoaded) return;
    try {
      const [byType, byCountry, bySector] = await Promise.all([
        api.assetAllocation('asset_type'),
        api.assetAllocation('country'),
        api.assetAllocation('sector'),
      ]);
      allocAssetType = byType;
      allocCountry = byCountry;
      allocSector = bySector;
      allocLoaded = true;
    } catch (e) {
      console.error('loadAllocations', e);
    }
  }

  $effect(() => {
    if (activeTab === 'allocation') {
      void loadAllocations();
    }
  });

  // After a background FX/price refresh: reload holdings + KPIs, since their
  // EUR values are FX-dependent. The active allocation tab is also refreshed.
  $effect(() => {
    type Status = { stage: 'started' | 'completed' | 'failed' };
    const unlisten = listen<Status>('price_refresh_status', (e) => {
      if (e.payload.stage !== 'completed') return;
      void loadAll();
      if (activeTab === 'allocation') void loadAllocations();
    });
    return () => { unlisten.then((u) => u()); };
  });

  function onSaved() {
    void loadAll();
  }

  function typeLabel(at: string): string {
    const key = at as keyof typeof ts.types;
    return ts.types[key] ?? at;
  }
</script>

<header class="page-h">
  <h1>{tp.title}</h1>
  <div class="actions">
    <KursRefreshButton onRefreshed={loadAll} />
    <button class="primary" type="button" onclick={() => (addingTrade = true)}>
      <Icon name="plus" size={14} /> {t().trade.addTrade}
    </button>
    <button class="primary" type="button" onclick={() => (addingNew = true)}>
      <Icon name="plus" size={14} /> {tp.new}
    </button>
  </div>
</header>

<div class="kpi-row">
  <div class="kpi">
    <span class="lbl">{tp.kpiMarketValue}</span>
    <span class="val">{fmtEur(kpis?.marketValueCents ?? 0, { hide: settings.hide, decimals: eurDecimals() })}</span>
  </div>
  <div class="kpi">
    <span class="lbl">{tp.kpiCostBasis}</span>
    <span class="val">{fmtEur(kpis?.costBasisCents ?? 0, { hide: settings.hide, decimals: eurDecimals() })}</span>
  </div>
  <div class="kpi">
    <span class="lbl">{tp.kpiUnrealized}</span>
    <span class="val">{fmtEur(kpis?.unrealizedCents ?? 0, { hide: settings.hide, signed: true, decimals: eurDecimals() })}</span>
  </div>
  <div class="kpi">
    <span class="lbl">{tp.kpiRealizedYtd}</span>
    <span class="val">{fmtEur(kpis?.realizedYtdCents ?? 0, { hide: settings.hide, signed: true, decimals: eurDecimals() })}</span>
  </div>
</div>

<div class="chart-card">
  <PerformanceChart />
</div>

<div class="tabs">
  <button class="tab" class:on={activeTab === 'positions'} onclick={() => (activeTab = 'positions')}>
    {tp.tabPositions}
  </button>
  <button class="tab" class:on={activeTab === 'dividends'} onclick={() => (activeTab = 'dividends')}>
    {tp.tabDividends}
  </button>
  <button class="tab" class:on={activeTab === 'allocation'} onclick={() => (activeTab = 'allocation')}>
    {tp.tabAllocation}
  </button>
  <button class="tab" class:on={activeTab === 'securities'} onclick={() => (activeTab = 'securities')}>
    {tp.tabSecurities}
  </button>
</div>

{#if activeTab === 'positions'}
  {#if loading && holdings.length === 0}
    <p class="muted">…</p>
  {:else if holdings.length === 0}
    <div class="empty">{tp.emptyPositions}</div>
  {:else}
    <ul class="hold-list">
      {#each holdings as h (h.securityId)}
        <li>
          <HoldingRow holding={h} onclick={(id) => goto(`/portfolio/${id}`)} />
        </li>
      {/each}
    </ul>
  {/if}
{:else if activeTab === 'dividends'}
  <DividendHistoryList entries={dividends} />
{:else if activeTab === 'allocation'}
  <div class="alloc-grid">
    <AllocationDonut title={tp.allocByAssetType} slices={allocAssetType} />
    <AllocationDonut title={tp.allocByCountry} slices={allocCountry} />
    <AllocationDonut title={tp.allocBySector} slices={allocSector} />
  </div>
{:else}
  <div class="sec-head">
    <label class="toggle">
      <input type="checkbox" bind:checked={showArchived} />
      <span>{tp.showArchived}</span>
    </label>
  </div>
  {#if loading && securities.length === 0}
    <p class="muted">…</p>
  {:else if securities.length === 0}
    <div class="empty">
      <p>{tp.empty}</p>
      <button class="primary" type="button" onclick={() => (addingNew = true)}>
        <Icon name="plus" size={14} /> {tp.newSecurity}
      </button>
    </div>
  {:else}
    <ul class="sec-list">
      {#each securities as sec (sec.id)}
        <li class:archived={sec.archived}>
          <button type="button" class="row" onclick={() => (editing = sec)}>
            <div class="left">
              <strong>{sec.name}</strong>
              <small class="muted">{sec.isin} · {sec.symbol ?? '—'}</small>
            </div>
            <div class="right">
              <span class="badge">{typeLabel(sec.assetType)}</span>
              <span class="muted">{sec.currency}</span>
            </div>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
{/if}

{#if addingNew}
  <SecurityForm security={null} onClose={() => (addingNew = false)} onSaved={onSaved} />
{/if}
{#if editing}
  <SecurityForm
    security={editing}
    onClose={() => (editing = null)}
    onSaved={onSaved}
    onDeleted={() => (editing = null)}
  />
{/if}
{#if addingTrade}
  <TradeModal onClose={() => (addingTrade = false)} onSaved={() => { addingTrade = false; loadAll(); }} />
{/if}

<style>
  .page-h {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 20px;
  }
  .page-h h1 { margin: 0; font-size: 22px; letter-spacing: -0.02em; }
  .actions { display: flex; gap: 12px; align-items: center; }

  .kpi-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 16px;
  }
  .kpi {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .kpi .lbl {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .kpi .val {
    font-size: 18px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }

  .chart-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px;
    margin-bottom: 16px;
  }

  .tabs {
    display: flex;
    gap: 0;
    margin-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }
  .tab {
    background: transparent;
    border: 0;
    padding: 10px 18px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    font: inherit;
  }
  .tab.on {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .tab:hover:not(.on) {
    color: var(--text);
  }

  .hold-list,
  .sec-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 6px;
  }

  .sec-head { margin-bottom: 8px; }
  .toggle { display: flex; gap: 6px; align-items: center; font-size: 13px; color: var(--text-muted); }
  .empty { text-align: center; padding: 32px 16px; color: var(--text-muted); }
  .empty p { margin: 0 0 16px; }
  .sec-list li button.row {
    width: 100%; text-align: left;
    background: var(--surface);
    border: 1px solid var(--border); border-radius: 10px;
    padding: 12px 14px;
    display: flex; align-items: center; justify-content: space-between;
    cursor: pointer; color: var(--text);
    font: inherit;
    transition: background 0.1s, border-color 0.1s;
  }
  .sec-list li button.row:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
  }
  .sec-list li.archived { opacity: 0.55; }
  .left { display: flex; flex-direction: column; gap: 2px; }
  .left strong { font-size: 13px; font-weight: 500; }
  .right { display: flex; gap: 12px; align-items: center; }
  .badge {
    background: var(--accent-soft);
    color: var(--accent);
    border-radius: 6px;
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 500;
  }
  .muted { color: var(--text-muted); font-size: 11px; }

  button.primary {
    background: var(--accent); color: var(--accent-fg); border: 0;
    padding: 8px 12px; border-radius: 8px; cursor: pointer;
    display: inline-flex; align-items: center; gap: 6px;
    font: inherit;
  }
  button.primary:hover { background: var(--accent-hover); }

  .alloc-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 12px;
  }

  @media (max-width: 599px) {
    .page-h { flex-wrap: wrap; gap: 8px; }
    .page-h h1 { font-size: 20px; }

    /* KPI row (4 tiles) → horizontal scroll strip */
    .kpi-row {
      grid-template-columns: none;
      display: flex;
      gap: 10px;
      overflow-x: auto;
      scroll-snap-type: x mandatory;
      margin: 0 -16px 14px;
      padding: 0 16px 6px;
    }
    .kpi-row > * {
      flex: 0 0 60%;
      max-width: 220px;
      scroll-snap-align: start;
    }

    /* Holdings/allocation grid → 1 column */
    .alloc-grid { grid-template-columns: 1fr; gap: 8px; }
  }
</style>
