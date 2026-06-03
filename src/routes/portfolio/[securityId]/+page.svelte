<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { listen } from '@tauri-apps/api/event';
  import Icon from '$lib/components/Icon.svelte';
  import SecurityPriceChart from '$lib/components/SecurityPriceChart.svelte';
  import DividendHistoryList from '$lib/components/DividendHistoryList.svelte';
  import SecurityForm from '$lib/components/SecurityForm.svelte';
  import DateField from '$lib/components/DateField.svelte';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';
  import {
    api,
    type Security, type Holding, type DividendEntry,
    type TradeWithTx, type SecurityBreakdown,
    type Bucket, type SecurityBucketAllocation, type AllocationItem, errMsg} from '$lib/api';
  import { fmtEur, parseEur } from '$lib/format';

  const securityId = $derived(Number(page.params.securityId));

  let security = $state<Security | null>(null);
  let holding = $state<Holding | null>(null);
  let trades = $state<TradeWithTx[]>([]);
  let dividends = $state<DividendEntry[]>([]);
  let breakdownCountry = $state<SecurityBreakdown[]>([]);
  let breakdownSector = $state<SecurityBreakdown[]>([]);
  let loading = $state(true);
  let editing = $state(false);

  let fetchingHistory = $state(false);
  let fetchHistoryToast = $state<string | null>(null);
  let chartReloadKey = $state(0);

  type DetailTab = 'trades' | 'dividends' | 'breakdown' | 'buckets';
  let activeTab = $state<DetailTab>('trades');

  const tp = $derived(t().portfolio);

  async function loadAll() {
    loading = true;
    try {
      const [sec, holdings, t_, div_, bdC, bdS] = await Promise.all([
        api.getSecurity(securityId),
        api.listHoldings(),
        api.listTrades(securityId),
        api.dividendHistory(),
        api.getBreakdown(securityId, 'country'),
        api.getBreakdown(securityId, 'sector'),
      ]);
      security = sec;
      holding = holdings.find((h) => h.securityId === securityId) ?? null;
      trades = t_;
      dividends = div_.filter((d) => d.securityId === securityId);
      breakdownCountry = bdC;
      breakdownSector = bdS;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void securityId;
    void loadAll();
  });

  // After a background price/FX refresh: reload holding, trades, dividends
  // so EUR-converted values and new price data points become visible.
  // Force chart re-render via chartReloadKey++.
  $effect(() => {
    type Status = { stage: 'started' | 'completed' | 'failed' };
    const unlisten = listen<Status>('price_refresh_status', (e) => {
      if (e.payload.stage === 'completed') {
        void loadAll();
        chartReloadKey++;
      }
    });
    return () => { unlisten.then((u) => u()); };
  });

  async function fetchHistory() {
    if (security === null) return;
    fetchingHistory = true;
    fetchHistoryToast = null;
    try {
      const n = await api.fetchSecurityHistory(security.id, 5);
      fetchHistoryToast = tp.fetchHistorySuccess(n);
      await loadAll();
      chartReloadKey++;
      setTimeout(() => (fetchHistoryToast = null), 3000);
    } catch (e) {
      fetchHistoryToast = `${tp.fetchHistoryFailed}: ${e}`;
      setTimeout(() => (fetchHistoryToast = null), 5000);
    } finally {
      fetchingHistory = false;
    }
  }

  // Manual price editor
  let manualOpen = $state(false);
  let manualDate = $state<string>(new Date().toISOString().slice(0, 10));
  let manualPrice = $state<string>('');
  let manualBusy = $state(false);
  let manualMsg = $state<string | null>(null);

  async function saveManualPrice() {
    if (security === null) return;
    const eur = parseEur(manualPrice);
    if (!Number.isFinite(eur) || eur < 0) {
      manualMsg = 'Bitte einen gültigen Preis eingeben';
      return;
    }
    const micro = Math.round(eur * 1_000_000);
    manualBusy = true;
    manualMsg = null;
    try {
      await api.setManualPrice(security.id, manualDate, micro);
      manualMsg = '✓ Gespeichert';
      chartReloadKey++;
      setTimeout(() => {
        manualMsg = null;
        manualOpen = false;
        manualPrice = '';
      }, 1500);
    } catch (e) {
      manualMsg = errMsg(e);
    } finally {
      manualBusy = false;
    }
  }

  function fmtShares(micro: number): string {
    const n = micro / 1_000_000;
    return new Intl.NumberFormat(settings.lang === 'en' ? 'en' : 'de', {
      minimumFractionDigits: 0,
      maximumFractionDigits: 4,
    }).format(n);
  }

  function fmtAvgCost(micro: number): string {
    // micro = millionths of a currency unit; fmtEur expects cents (1 cent = 10_000 micro).
    return fmtEur(Math.round(micro / 10_000), { hide: settings.hide, decimals: 2 });
  }

  function fmtUnitPrice(micro: number | null): string {
    if (micro == null) return '—';
    return (micro / 1_000_000).toLocaleString(settings.lang === 'en' ? 'en' : 'de', {
      minimumFractionDigits: 2, maximumFractionDigits: 4,
    });
  }

  // ─── Bucket allocation state ──────────────────────────────────────────────
  let allBuckets = $state<Bucket[]>([]);
  let allocDraft = $state<Array<{ bucketId: number; sharesMicroInput: string }>>([]);
  let allocSaving = $state(false);
  let allocError = $state<string | null>(null);
  let allocToast = $state<string | null>(null);
  let bucketCashMap = $state<Map<number, number>>(new Map());
  let bucketSecMap = $state<Map<number, number>>(new Map());

  async function loadAllocations() {
    const [b, a, progress] = await Promise.all([
      api.listBuckets(false),
      api.listSecurityAllocations(securityId),
      api.listBucketProgress(),
    ]);
    allBuckets = b;
    allocDraft = a.map((row: SecurityBucketAllocation) => ({
      bucketId: row.bucketId,
      sharesMicroInput: (row.sharesMicro / 1_000_000).toString(),
    }));
    bucketCashMap = new Map(progress.map((p) => [p.bucketId, p.currentCents]));
    const secEntries = await Promise.all(
      b.map(async (bk) => {
        const rows = await api.bucketHoldings(bk.id);
        return [bk.id, rows.reduce((s, r) => s + r.valueCents, 0)] as const;
      })
    );
    bucketSecMap = new Map(secEntries);
  }

  function bucketTotal(bid: number): number {
    return (bucketCashMap.get(bid) ?? 0) + (bucketSecMap.get(bid) ?? 0);
  }
  function bucketTarget(bid: number): number | null {
    return allBuckets.find((b) => b.id === bid)?.targetCents ?? null;
  }

  $effect(() => {
    if (activeTab === 'buckets') {
      void securityId;
      void loadAllocations();
    }
  });

  const heldSharesMicro = $derived(holding?.sharesMicro ?? 0);
  const heldMarketValueCents = $derived(holding?.marketValueCents ?? 0);
  /** Cents per micro-share. 0 if no holding/market value. */
  const valuePerMicro = $derived(
    heldSharesMicro > 0 ? heldMarketValueCents / heldSharesMicro : 0
  );

  function sharesInputToMicro(s: string): number {
    const n = parseEur(s) || 0;
    return Math.round(n * 1_000_000);
  }

  function rowValueCents(s: string): number {
    return Math.round(sharesInputToMicro(s) * valuePerMicro);
  }

  const draftSumMicro = $derived(
    allocDraft.reduce((sum, row) => {
      const n = parseEur(row.sharesMicroInput) || 0;
      return sum + Math.round(n * 1_000_000);
    }, 0),
  );

  const unallocatedMicro = $derived(heldSharesMicro - draftSumMicro);
  const draftValueCents = $derived(Math.round(draftSumMicro * valuePerMicro));
  const unallocatedValueCents = $derived(Math.round(unallocatedMicro * valuePerMicro));

  function addAllocationRow() {
    const used = new Set(allocDraft.map((r) => r.bucketId));
    const firstFree = allBuckets.find((b) => !used.has(b.id));
    if (!firstFree) return;
    allocDraft = [...allocDraft, { bucketId: firstFree.id, sharesMicroInput: '0' }];
  }

  function removeAllocationRow(index: number) {
    allocDraft = allocDraft.filter((_, i) => i !== index);
  }

  async function saveAllocations() {
    if (allocSaving) return;
    allocSaving = true;
    allocError = null;
    try {
      const items: AllocationItem[] = allocDraft
        .map((row) => {
          const n = parseEur(row.sharesMicroInput) || 0;
          return { bucketId: row.bucketId, sharesMicro: Math.round(n * 1_000_000) };
        })
        .filter((i) => i.sharesMicro > 0);

      await api.setSecurityAllocations(securityId, items);
      allocToast = tp.bucketAllocSaved;
      await loadAllocations();
      setTimeout(() => (allocToast = null), 3000);
    } catch (e) {
      allocError = errMsg(e);
    } finally {
      allocSaving = false;
    }
  }
</script>

<header class="page-h">
  <button class="back" onclick={() => goto('/portfolio')}>
    {tp.detailBack}
  </button>
  <div class="actions">
    {#if security}
      <button class="secondary" onclick={() => (editing = true)}>
        <Icon name="pencil" size={13} /> {tp.detailEdit}
      </button>
    {/if}
  </div>
</header>

{#if loading && !security}
  <p class="muted">…</p>
{:else if !security}
  <div class="empty">404</div>
{:else}
  <div class="title">
    <h1>{security.name}</h1>
    <p class="muted">
      {security.isin}{#if security.symbol} · {security.symbol}{/if} · {security.currency}
    </p>
  </div>

  <div class="kpi-row">
    <div class="kpi">
      <span class="lbl">{tp.shares}</span>
      <span class="val">{holding ? fmtShares(holding.sharesMicro) : '—'}</span>
    </div>
    <div class="kpi">
      <span class="lbl">{tp.avgCost}</span>
      <span class="val">{holding ? fmtAvgCost(holding.avgCostPerShareMicro) : '—'}</span>
    </div>
    <div class="kpi">
      <span class="lbl">{tp.kpiMarketValue}</span>
      <span class="val">{holding ? fmtEur(holding.marketValueCents, { hide: settings.hide, decimals: eurDecimals() }) : '—'}</span>
    </div>
    <div class="kpi">
      <span class="lbl">{tp.kpiUnrealized}</span>
      <span class="val" class:pos={holding && holding.unrealizedCents > 0} class:neg={holding && holding.unrealizedCents < 0}>
        {holding ? fmtEur(holding.unrealizedCents, { hide: settings.hide, signed: true, decimals: eurDecimals() }) : '—'}
      </span>
    </div>
  </div>

  <div class="chart-card">
    <div class="chart-actions">
      <button class="btn-fetch" onclick={fetchHistory} disabled={fetchingHistory}>
        <Icon name="refresh" size={12} />
        {fetchingHistory ? tp.fetchHistoryBusy : tp.fetchHistoryButton}
      </button>
      <button class="btn-fetch" type="button" onclick={() => (manualOpen = !manualOpen)}>
        <Icon name="pencil" size={12} />
        Preis manuell setzen
      </button>
      <span class="toast-inline" aria-live="polite" aria-atomic="true">{fetchHistoryToast ?? ''}</span>
    </div>
    {#if manualOpen}
      <div class="manual-price-row">
        <label>
          Datum
          <DateField bind:value={manualDate} disabled={manualBusy} />
        </label>
        <label>
          Preis (€)
          <input
            type="text"
            inputmode="decimal"
            bind:value={manualPrice}
            disabled={manualBusy}
            placeholder="z.B. 149,12"
          />
        </label>
        <button class="btn-fetch" type="button" onclick={saveManualPrice} disabled={manualBusy}>
          {manualBusy ? 'Speichert…' : 'Speichern'}
        </button>
        <span class="toast-inline" aria-live="polite" aria-atomic="true">{manualMsg ?? ''}</span>
      </div>
    {/if}
    <SecurityPriceChart {securityId} reloadKey={chartReloadKey} />
  </div>

  <div class="tabs">
    <button class="tab" class:on={activeTab === 'trades'} onclick={() => (activeTab = 'trades')}>
      {tp.detailTabTrades}
    </button>
    <button class="tab" class:on={activeTab === 'dividends'} onclick={() => (activeTab = 'dividends')}>
      {tp.detailTabDividends}
    </button>
    <button class="tab" class:on={activeTab === 'breakdown'} onclick={() => (activeTab = 'breakdown')}>
      {tp.detailTabBreakdown}
    </button>
    <button class="tab" class:on={activeTab === 'buckets'} onclick={() => (activeTab = 'buckets')}>
      {tp.tabBuckets}
    </button>
  </div>

  {#if activeTab === 'trades'}
    {#if trades.length === 0}
      <div class="empty">—</div>
    {:else}
      <ul class="trades-list">
        {#each trades as t (t.trade.txId)}
          <li>
            <span class="date">{t.tx.booking_date}</span>
            <span class="side {t.trade.side}">{t.trade.side}</span>
            <span class="num">{fmtShares(Math.abs(t.trade.sharesMicro))}</span>
            <span class="muted">@ {fmtUnitPrice(t.trade.unitPriceMicro)}</span>
            <span class="num">{fmtEur(t.tx.amount_cents, { hide: settings.hide, signed: true, decimals: 2 })}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {:else if activeTab === 'dividends'}
    <DividendHistoryList entries={dividends} />
  {:else if activeTab === 'breakdown'}
    <div class="breakdown-grid">
      <div class="bd-card">
        <h4>{tp.allocByCountry}</h4>
        {#if breakdownCountry.length === 0}
          <p class="muted">{tp.noBreakdown}</p>
        {:else}
          <ul>
            {#each breakdownCountry as b (b.key)}
              <li>
                <span>{b.key}</span>
                <span class="num">{(b.weightBps / 100).toFixed(2)}%</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
      <div class="bd-card">
        <h4>{tp.allocBySector}</h4>
        {#if breakdownSector.length === 0}
          <p class="muted">{tp.noBreakdown}</p>
        {:else}
          <ul>
            {#each breakdownSector as b (b.key)}
              <li>
                <span>{b.key}</span>
                <span class="num">{(b.weightBps / 100).toFixed(2)}%</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  {:else if activeTab === 'buckets'}
    <div class="bucket-alloc">
      <p class="muted">{tp.bucketAllocateHint}</p>
      {#if allBuckets.length === 0}
        <p class="empty">{tp.bucketAllocNoBuckets}</p>
      {:else}
        <ul class="alloc-list">
          {#each allocDraft as row, i (i)}
            {@const total = bucketTotal(row.bucketId)}
            {@const target = bucketTarget(row.bucketId)}
            {@const remaining = target !== null && target > 0 ? Math.max(0, target - total) : null}
            <li class="alloc-row-wrap">
              <div class="alloc-row">
                <select bind:value={row.bucketId}>
                  {#each allBuckets as b (b.id)}
                    <option value={b.id}>{b.name}</option>
                  {/each}
                </select>
                <input
                  type="text"
                  inputmode="decimal"
                  bind:value={row.sharesMicroInput}
                  placeholder="0"
                />
                <span class="muted">{tp.bucketAllocShares}</span>
                <span class="row-value num">
                  ≈ {fmtEur(rowValueCents(row.sharesMicroInput), { hide: settings.hide, decimals: eurDecimals() })}
                </span>
                <button class="btn-remove" type="button" onclick={() => removeAllocationRow(i)} aria-label={tp.bucketAllocRemove ?? 'Zeile entfernen'}>
                  <Icon name="x" size={12} />
                </button>
              </div>
              <div class="bucket-state">
                <span class="bs-item">
                  <span class="bs-lbl">{t().common.bucketCurrent ?? 'Bereits'}:</span>
                  <span class="bs-num">{fmtEur(total, { hide: settings.hide, decimals: eurDecimals() })}</span>
                </span>
                {#if target !== null && target > 0}
                  <span class="bs-sep">·</span>
                  <span class="bs-item">
                    <span class="bs-lbl">{t().common.bucketTarget ?? 'Ziel'}:</span>
                    <span class="bs-num">{fmtEur(target, { hide: settings.hide, decimals: eurDecimals() })}</span>
                  </span>
                  <span class="bs-sep">·</span>
                  <span class="bs-item" class:done={remaining === 0}>
                    <span class="bs-lbl">{remaining === 0 ? (t().common.bucketReached ?? 'erreicht ✓') : (t().common.bucketRemaining ?? 'Noch')}:</span>
                    {#if remaining !== 0}
                      <span class="bs-num">{fmtEur(remaining ?? 0, { hide: settings.hide, decimals: eurDecimals() })}</span>
                    {/if}
                  </span>
                {/if}
              </div>
            </li>
          {/each}
        </ul>

        <button
          class="btn-add"
          type="button"
          onclick={addAllocationRow}
          disabled={allocDraft.length >= allBuckets.length}
        >
          {tp.bucketAllocAddRow}
        </button>

        <div class="alloc-summary">
          <div>
            <span class="muted">{tp.bucketAllocTotal}:</span>
            <span class="num">{fmtShares(draftSumMicro)} / {fmtShares(heldSharesMicro)}</span>
            <span class="muted">·</span>
            <span class="num accent">≈ {fmtEur(draftValueCents, { hide: settings.hide, decimals: eurDecimals() })}</span>
          </div>
          <div class:warn={unallocatedMicro < 0}>
            <span class="muted">{tp.bucketAllocUnallocated}:</span>
            <span class="num">{fmtShares(unallocatedMicro)}</span>
            <span class="muted">·</span>
            <span class="num">≈ {fmtEur(unallocatedValueCents, { hide: settings.hide, decimals: eurDecimals() })}</span>
          </div>
        </div>

        <p class="error" role="alert" aria-live="assertive" aria-atomic="true">
          {#if allocError}<Icon name="alert-circle" size={12} /> {allocError}{/if}
        </p>
        <p class="info" aria-live="polite" aria-atomic="true">{allocToast ?? ''}</p>

        <button class="btn-save" type="button" onclick={saveAllocations} disabled={allocSaving}>
          {tp.bucketAllocSave}
        </button>
      {/if}
    </div>
  {/if}
{/if}

{#if editing && security}
  <SecurityForm
    security={security}
    onClose={() => (editing = false)}
    onSaved={() => { editing = false; loadAll(); }}
    onDeleted={() => goto('/portfolio')}
  />
{/if}

<style>
  .page-h {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 16px;
  }
  .back {
    background: transparent;
    border: 0;
    color: var(--text-muted);
    font-size: 13px;
    cursor: pointer;
    font: inherit;
  }
  .back:hover { color: var(--text); }
  .actions { display: flex; gap: 8px; }
  button.secondary {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 6px 10px;
    cursor: pointer;
    display: inline-flex;
    gap: 6px;
    align-items: center;
    font: inherit;
    color: var(--text);
    font-size: 13px;
  }
  button.secondary:hover { background: var(--surface-hover); }
  .title h1 { margin: 0; font-size: 22px; letter-spacing: -0.02em; }
  .title .muted { margin: 2px 0 16px; color: var(--text-muted); font-size: 12px; }

  .kpi-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 16px;
  }
  .kpi {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 12px 14px;
    display: flex; flex-direction: column; gap: 6px;
  }
  .kpi .lbl {
    font-size: 11px; color: var(--text-muted);
    text-transform: uppercase; letter-spacing: 0.04em;
  }
  .kpi .val { font-size: 18px; font-weight: 500; font-variant-numeric: tabular-nums; font-family: var(--font-mono); }
  .kpi .pos { color: var(--positive); }
  .kpi .neg { color: var(--negative); }

  .chart-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 14px;
    margin-bottom: 16px;
  }
  .manual-price-row {
    display: flex; flex-wrap: wrap; gap: 10px; align-items: end;
    margin: 8px 0; padding: 8px 10px;
    background: var(--surface-2); border: 1px solid var(--border); border-radius: var(--r-sm);
    font-size: 12px;
  }
  .manual-price-row label {
    display: grid; gap: 3px; color: var(--text-muted);
  }
  .manual-price-row input {
    background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-sm);
    padding: 4px 8px; font: inherit; color: var(--text);
  }
  .chart-actions {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }
  .btn-fetch {
    padding: 4px 10px;
    border-radius: var(--r-sm);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .btn-fetch:hover:not(:disabled) { background: var(--surface-hover); }
  .btn-fetch:disabled { opacity: 0.5; cursor: wait; }
  .toast-inline { font-size: 12px; color: var(--text-muted); }

  .tabs {
    display: flex; gap: 0; margin-bottom: 16px;
    border-bottom: 1px solid var(--border);
  }
  .tab {
    background: transparent; border: 0;
    padding: 10px 18px; font-size: 13px; font-weight: 500;
    color: var(--text-muted); cursor: pointer;
    border-bottom: 2px solid transparent;
    font: inherit;
  }
  .tab.on { color: var(--text); border-bottom-color: var(--accent); }

  .trades-list { list-style: none; padding: 0; margin: 0; display: grid; gap: 4px; overflow-x: auto; -webkit-overflow-scrolling: touch; }
  .trades-list li {
    display: grid;
    grid-template-columns: 100px 100px 1fr 1fr 1fr;
    gap: 12px;
    padding: 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    font-size: 12px;
    align-items: center;
    min-width: 480px;
  }
  .date { color: var(--text-muted); font-variant-numeric: tabular-nums; }
  .side {
    text-transform: uppercase;
    font-size: 11px;
    font-weight: 500;
    padding: 2px 6px;
    border-radius: var(--r-sm);
    background: var(--surface-2);
    text-align: center;
  }
  .side.buy { color: var(--positive); }
  .side.sell { color: var(--negative); }
  .side.dividend { color: var(--accent); }
  .num { font-variant-numeric: tabular-nums; text-align: right; }

  .breakdown-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 12px;
  }
  .bd-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 14px;
  }
  .bd-card h4 { margin: 0 0 8px; font-size: 13px; font-weight: 500; }
  .bd-card ul { list-style: none; padding: 0; margin: 0; display: grid; gap: 4px; }
  .bd-card li {
    display: flex; justify-content: space-between;
    font-size: 12px;
    padding: 4px 0;
    border-bottom: 1px solid var(--border);
  }

  .muted { color: var(--text-muted); font-size: 12px; }
  .empty { padding: 32px 16px; text-align: center; color: var(--text-faint); }

  .bucket-alloc { display: grid; gap: 12px; max-width: 600px; }
  .bucket-alloc .muted { color: var(--text-muted); font-size: 12px; margin: 0; }
  .alloc-list { list-style: none; padding: 0; margin: 0; display: grid; gap: 6px; }
  .alloc-row-wrap { display: grid; gap: 4px; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .alloc-row-wrap:last-child { border-bottom: 0; }
  .alloc-row { display: grid; grid-template-columns: 1fr 120px auto auto auto; gap: 8px; align-items: center; }
  .alloc-row .row-value { font-size: 12px; color: var(--text-muted); white-space: nowrap; }
  .alloc-summary .accent { color: var(--accent); }
  .alloc-row-wrap .bucket-state {
    display: flex; flex-wrap: wrap; align-items: baseline;
    gap: 4px 6px; padding-left: 4px;
    font-size: 11px; color: var(--text-muted);
  }
  .alloc-row-wrap .bucket-state .bs-num { font-variant-numeric: tabular-nums; color: var(--text); }
  .alloc-row-wrap .bucket-state .bs-sep { color: var(--text-faint); }
  .alloc-row-wrap .bucket-state .done { color: var(--positive); }
  .alloc-row-wrap .bucket-state .done .bs-lbl { color: var(--positive); }
  .alloc-row select, .alloc-row input {
    padding: 4px 8px; border: 1px solid var(--border); border-radius: var(--r-sm);
    background: var(--surface-2); color: var(--text); font: inherit; font-size: 13px;
  }
  .btn-remove {
    background: transparent; border: 0; color: var(--text-muted); cursor: pointer; padding: 4px;
  }
  .btn-add, .btn-save {
    padding: 6px 14px; border-radius: var(--r-sm); border: 1px solid var(--border);
    background: var(--surface-2); color: var(--text); cursor: pointer; font: inherit; font-size: 12px;
    align-self: flex-start;
  }
  .btn-save { border-color: var(--accent); color: var(--accent); }
  .alloc-summary { display: grid; gap: 4px; font-size: 13px; }
  .alloc-summary .warn { color: var(--negative); }
  .alloc-summary .warn .num { color: var(--negative); font-weight: 500; }
  .error { display: flex; align-items: center; gap: 6px; color: var(--negative); font-size: 12px; margin: 0; }
  .info { color: var(--text-muted); font-size: 12px; margin: 0; }
</style>
