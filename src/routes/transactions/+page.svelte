<script lang="ts">
  import { api, type Account, type Bucket, type Category, type Transaction, type TxFilter, type TxAggregate, errMsg, listInstitutions, isTradeTx } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import DepotTxModal from '$lib/components/DepotTxModal.svelte';
  import type { ExportFilter, Institution } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import KPI from '$lib/components/KPI.svelte';
  import TxRow from '$lib/components/TxRow.svelte';
  import TxModal from '$lib/components/TxModal.svelte';
  import TradeModal from '$lib/components/TradeModal.svelte';
  import ExportButton from '$lib/components/ExportButton.svelte';
  import OverflowMenu from '$lib/components/OverflowMenu.svelte';
  import ImportStatementsModal from '$lib/components/ImportStatementsModal.svelte';
  import AccountCreateModal from '$lib/components/AccountCreateModal.svelte';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import DateField from '$lib/components/DateField.svelte';
  import { settings, t } from '$lib/settings.svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { groupByDay } from '$lib/tx-grouping';

  let transactions = $state<Transaction[]>([]);
  let accounts = $state<Account[]>([]);
  let categories = $state<Category[]>([]);
  let buckets = $state<Bucket[]>([]);
  let institutions = $state<Institution[]>([]);
  const bucketsById = $derived(new Map(buckets.map((b) => [b.id, b])));
  const institutionsById = $derived(new Map(institutions.map((i) => [i.id, i])));
  let loading = $state(true);
  let error = $state<string | null>(null);

  let search = $state('');
  let filterCat = $state<'all' | number>('all');
  let filterAcc = $state<'all' | number>('all');
  let filterInstitution = $state<'all' | number>('all');
  let filterBucket = $state<'all' | number>('all');
  let filterFrom = $state<string>('');  // YYYY-MM-DD inclusive
  let filterTo = $state<string>('');    // YYYY-MM-DD inclusive
  let filterUncategorized = $state(false);
  let filterMinAmount = $state<number | null>(null);  // in cents, abs

  let showImportModal = $state(false);
  let showCreateAccount = $state(false);
  const hasAccounts = $derived(accounts.filter((a) => !a.archived).length > 0);
  let showFilterPanel = $state(false);

  const activeFilterCount = $derived.by(() => {
    let n = 0;
    if (filterCat !== 'all') n++;
    if (filterAcc !== 'all') n++;
    if (filterInstitution !== 'all') n++;
    if (filterBucket !== 'all') n++;
    if (filterFrom || filterTo) n++;
    if (filterUncategorized) n++;
    if (filterMinAmount !== null) n++;
    return n;
  });

  // ─── Multi-select / bulk operations ────────────────────────────────────────
  let selectedIds = $state<Set<number>>(new Set());
  let bulkBusy = $state(false);
  let bulkError = $state<string | null>(null);

  function setIndeterminate(node: HTMLInputElement, value: boolean) {
    node.indeterminate = value;
    return {
      update(newValue: boolean) {
        node.indeterminate = newValue;
      }
    };
  }

  function toggleSelect(id: number) {
    if (selectedIds.has(id)) selectedIds.delete(id);
    else selectedIds.add(id);
    selectedIds = new Set(selectedIds);
  }
  function selectAllVisible() {
    selectedIds = new Set(transactions.map((t) => t.id));
  }
  function clearSelection() {
    selectedIds = new Set();
  }

  async function bulkAssignCategory(catId: number | null) {
    if (selectedIds.size === 0) return;
    bulkBusy = true; bulkError = null;
    try {
      for (const id of selectedIds) {
        await api.assignCategory(id, catId);
      }
      // Update local cache
      const newCat = catId;
      transactions = transactions.map((tx) =>
        selectedIds.has(tx.id) ? { ...tx, category_id: newCat } : tx
      );
      clearSelection();
    } catch (e) {
      bulkError = errMsg(e);
    } finally {
      bulkBusy = false;
    }
  }

  async function bulkAssignBucket(bucketId: number | null) {
    if (selectedIds.size === 0) return;
    bulkBusy = true; bulkError = null;
    try {
      for (const id of selectedIds) {
        await api.assignBucket(id, bucketId);
      }
      transactions = transactions.map((tx) =>
        selectedIds.has(tx.id) ? { ...tx, bucket_id: bucketId } : tx
      );
      clearSelection();
    } catch (e) {
      bulkError = errMsg(e);
    } finally {
      bulkBusy = false;
    }
  }

  async function bulkDelete() {
    if (selectedIds.size === 0) return;
    if (!confirm(`Really delete ${selectedIds.size} transactions?`)) return;
    bulkBusy = true; bulkError = null;
    try {
      const tradeKinds = ['buy', 'sell', 'dividend', 'corporate_action'];
      const txById = new Map(transactions.map((tx) => [tx.id, tx]));
      for (const id of selectedIds) {
        const tx = txById.get(id);
        if (tx && tradeKinds.includes(tx.kind)) {
          await api.deleteTrade(id);
        } else {
          await api.deleteTransaction(id);
        }
      }
      transactions = transactions.filter((tx) => !selectedIds.has(tx.id));
      clearSelection();
    } catch (e) {
      bulkError = errMsg(e);
    } finally {
      bulkBusy = false;
    }
  }

  let bulkCatPick = $state<'' | number>('');
  let bulkBucketPick = $state<'' | number | 'none'>('');

  $effect(() => {
    if (bulkCatPick !== '') {
      const v = bulkCatPick;
      bulkCatPick = '';
      void bulkAssignCategory(v as number);
    }
  });
  $effect(() => {
    if (bulkBucketPick !== '') {
      const v = bulkBucketPick;
      bulkBucketPick = '';
      void bulkAssignBucket(v === 'none' ? null : (v as number));
    }
  });

  function pillThisMonth() {
    const now = new Date();
    filterFrom = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-01`;
    const end = new Date(now.getFullYear(), now.getMonth() + 1, 0);
    filterTo = end.toISOString().slice(0, 10);
  }
  function pillLastMonth() {
    const now = new Date();
    const y = now.getMonth() === 0 ? now.getFullYear() - 1 : now.getFullYear();
    const m = now.getMonth() === 0 ? 12 : now.getMonth();
    filterFrom = `${y}-${String(m).padStart(2, '0')}-01`;
    const end = new Date(y, m, 0);
    filterTo = end.toISOString().slice(0, 10);
  }
  function pillUncategorized() {
    filterUncategorized = !filterUncategorized;
  }
  function pillBig() {
    filterMinAmount = filterMinAmount === 10000 ? null : 10000;
  }

  // Read URL params (e.g. ?categoryId=1&from=2026-05-01&to=2026-05-31 from the Sankey diagram)
  $effect(() => {
    const url = page.url;
    const cat = url.searchParams.get('categoryId');
    const from = url.searchParams.get('from');
    const to = url.searchParams.get('to');
    if (cat !== null) {
      const n = Number(cat);
      if (Number.isFinite(n)) filterCat = n;
    }
    if (from !== null) filterFrom = from;
    if (to !== null) filterTo = to;
  });

  // FAB wiring (variant B): ?new=1 opens the "New Transaction" form
  $effect(() => {
    const newFlag = page.url.searchParams.get('new');
    if (newFlag === '1') {
      modalTx = null;
      modalOpen = true;
      // Clean up the URL so a reload doesn't reopen the modal
      goto('/transactions', { replaceState: true, noScroll: true });
    }
  });

  let modalOpen = $state(false);
  let modalTx = $state<Transaction | null>(null);
  let tradeModalOpen = $state(false);
  let newMenuOpen = $state(false);

  function openNewCash() {
    newMenuOpen = false;
    modalTx = null;
    modalOpen = true;
  }
  function openNewTrade() {
    newMenuOpen = false;
    tradeModalOpen = true;
  }
  function openEdit(tx: Transaction) {
    modalTx = tx;
    modalOpen = true;
  }
  function closeModal() {
    modalOpen = false;
  }
  function onSaved(saved: Transaction) {
    const idx = transactions.findIndex((t) => t.id === saved.id);
    if (idx >= 0) transactions[idx] = saved;
    else transactions = [saved, ...transactions];
  }
  function onDeleted(id: number) {
    transactions = transactions.filter((t) => t.id !== id);
  }

  // ── Server-side paginated loading ──────────────────────────────────────
  const PAGE_SIZE = 200;
  let cursor = $state<string | null>(null);
  let hasMore = $state(false);
  let loadingMore = $state(false);
  let aggregate = $state<TxAggregate>({ inCents: 0, outCents: 0, count: 0 });
  /** Local display of category diversity, built up from loaded pages. */
  let usedCategoryIds = $state<Set<number>>(new Set());

  function buildServerFilter(): TxFilter {
    const f: TxFilter = { limit: PAGE_SIZE };
    if (search.trim() !== '') f.search = search.trim();
    if (filterCat !== 'all') f.categoryId = filterCat as number;
    if (filterAcc !== 'all') f.accountId = filterAcc as number;
    if (filterInstitution !== 'all') f.institutionId = filterInstitution as number;
    if (filterBucket !== 'all') f.bucketId = filterBucket as number;
    if (filterFrom) f.from = filterFrom;
    if (filterTo) f.to = filterTo;
    if (filterUncategorized) f.uncategorized = true;
    if (filterMinAmount !== null) f.minAmountCents = filterMinAmount;
    return f;
  }

  async function loadMetadata() {
    const [accs, cats, bks, insts] = await Promise.all([
      api.listAccounts(),
      api.listCategories(),
      api.listBuckets(true),
      listInstitutions(false),
    ]);
    accounts = accs;
    categories = cats;
    buckets = bks;
    institutions = insts;
  }

  async function loadFirstPage() {
    loading = true;
    error = null;
    cursor = null;
    transactions = [];
    try {
      const filter = buildServerFilter();
      const [page, agg] = await Promise.all([
        api.listTransactions(filter),
        api.aggregateTransactions(filter),
      ]);
      transactions = page.rows;
      cursor = page.nextCursor;
      hasMore = page.hasMore;
      aggregate = agg;
      const ids = new Set<number>();
      for (const tx of page.rows) if (tx.category_id) ids.add(tx.category_id);
      usedCategoryIds = ids;
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    if (loadingMore || !hasMore || cursor === null) return;
    loadingMore = true;
    try {
      const filter = { ...buildServerFilter(), cursor };
      const page = await api.listTransactions(filter);
      transactions = [...transactions, ...page.rows];
      cursor = page.nextCursor;
      hasMore = page.hasMore;
      const ids = new Set(usedCategoryIds);
      for (const tx of page.rows) if (tx.category_id) ids.add(tx.category_id);
      usedCategoryIds = ids;
    } catch (e) {
      error = errMsg(e);
    } finally {
      loadingMore = false;
    }
  }

  // Initial metadata + first page load.
  $effect(() => {
    void loadMetadata();
  });

  // Debounced reload on filter change.
  const filterKey = $derived(JSON.stringify({
    search, filterCat, filterAcc, filterInstitution, filterBucket,
    filterFrom, filterTo, filterUncategorized, filterMinAmount,
  }));
  let reloadTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    void filterKey;
    if (reloadTimer) clearTimeout(reloadTimer);
    reloadTimer = setTimeout(() => { void loadFirstPage(); }, 250);
  });

  const inSum = $derived(aggregate.inCents);
  const outSum = $derived(aggregate.outCents);
  const total = $derived(inSum - outSum);

  const groupedFiltered = $derived(groupByDay(transactions));

  function clearFilters() {
    search = '';
    filterCat = 'all';
    filterAcc = 'all';
    filterInstitution = 'all';
    filterBucket = 'all';
    filterFrom = '';
    filterTo = '';
    filterUncategorized = false;
    filterMinAmount = null;
  }

  const hasFilter = $derived(
    search !== '' || filterCat !== 'all' || filterAcc !== 'all' || filterInstitution !== 'all'
      || filterBucket !== 'all' || filterFrom !== '' || filterTo !== ''
      || filterUncategorized || filterMinAmount !== null,
  );

  const activeAccounts = $derived(accounts.filter((a) => !a.archived));

  // Account picker: when an institution is filtered, show only accounts of that institution.
  const filteredAccountsForPicker = $derived(
    filterInstitution === 'all'
      ? activeAccounts
      : activeAccounts.filter((a) => a.institution_id === filterInstitution)
  );

  // Reset the account filter if the selected account no longer belongs to the institution.
  $effect(() => {
    if (filterAcc !== 'all'
        && filterInstitution !== 'all'
        && !filteredAccountsForPicker.some((a) => a.id === filterAcc)) {
      filterAcc = 'all';
    }
  });

  function buildExportFilter(): ExportFilter {
    return {
      accountId: filterAcc === 'all' ? undefined : (filterAcc as number),
      institutionId: filterInstitution === 'all' ? undefined : (filterInstitution as number),
      categoryId: filterCat === 'all' ? undefined : (filterCat as number),
      from: filterFrom || undefined,
      to: filterTo || undefined,
      search: search.trim() === '' ? undefined : search.trim(),
    };
  }
</script>

<div class="topbar">
  <div>
    <h1>{t().nav.transactions}</h1>
    <div class="sub">
      {aggregate.count} ·
      <span class="num">{t().common.net}: {fmtEur(total, { hide: settings.hide, signed: true })}</span>
    </div>
  </div>
  <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap; justify-content: flex-end;">
    {#if hasAccounts}
      <button class="btn" onclick={() => (showImportModal = true)}>
        <Icon name="arrow-up" size={13} />
        {t().common.importStatements}
      </button>
    {:else}
      <button class="btn accent" onclick={() => (showCreateAccount = true)}>
        <Icon name="plus" size={13} />
        {t().common.addAccount}
      </button>
    {/if}
    <OverflowMenu>
      {#snippet children()}
        <ExportButton getFilter={buildExportFilter} />
      {/snippet}
    </OverflowMenu>
    <div class="new-dropdown" data-tour="new-tx">
      <button
        class="btn primary"
        type="button"
        onclick={() => (newMenuOpen = !newMenuOpen)}
        disabled={activeAccounts.length === 0}
      >
        <Icon name="plus" size={13} />
        {t().common.newTx}
        <span class="chev" class:open={newMenuOpen}>▾</span>
      </button>
      {#if newMenuOpen}
        <ul class="new-menu" role="menu">
          <li>
            <button type="button" role="menuitem" onclick={openNewCash}>
              <Icon name="arrow-down" size={12} />
              {t().common.newTxCash}
            </button>
          </li>
          <li>
            <button type="button" role="menuitem" onclick={openNewTrade}>
              <Icon name="trending-up" size={12} />
              {t().common.newTxTrade}
            </button>
          </li>
        </ul>
      {/if}
    </div>
  </div>
</div>

{#if error}
  <div class="card" style="color:var(--negative); margin-bottom: 14px;">Fehler: {error}</div>
{/if}

<div class="card card-pad-lg tx-summary">
  <div class="tx-summary-col">
    <span class="tx-summary-label">{t().common.income}</span>
    <span class="tx-summary-value pos">{fmtEur(inSum, { hide: settings.hide })}</span>
  </div>
  <div class="tx-summary-divider" aria-hidden="true"></div>
  <div class="tx-summary-col">
    <span class="tx-summary-label">{t().common.expenses}</span>
    <span class="tx-summary-value neg">{fmtEur(outSum, { hide: settings.hide })}</span>
  </div>
  <div class="tx-summary-divider" aria-hidden="true"></div>
  <div class="tx-summary-col">
    <span class="tx-summary-label">{t().common.net}</span>
    <span class="tx-summary-value" class:pos={total >= 0} class:neg={total < 0}>
      {fmtEur(total, { hide: settings.hide, signed: true })}
    </span>
  </div>
</div>

<div class="card card-pad-lg">
  <div class="filter-bar">
    <div class="filter-bar-row">
      <div class="search-wrap">
        <span class="search-icon"><Icon name="search" size={14} /></span>
        <input class="input" placeholder={t().common.search} bind:value={search} style="padding-left: 34px;" />
      </div>
      <button
        type="button"
        class="btn filter-toggle"
        class:active={hasFilter}
        onclick={() => (showFilterPanel = !showFilterPanel)}
        aria-expanded={showFilterPanel}
        aria-controls="tx-filter-panel"
      >
        <Icon name="filter" size={13} />
        Filter
        {#if activeFilterCount > 0}
          <span class="filter-badge">{activeFilterCount}</span>
        {/if}
        <span class="chev" class:open={showFilterPanel}>▾</span>
      </button>
      {#if hasFilter}
        <button class="btn" onclick={clearFilters} aria-label="Filter zurücksetzen" title="Alle Filter zurücksetzen">
          <Icon name="x" size={13} />
        </button>
      {/if}
    </div>
    {#if showFilterPanel}
      <div id="tx-filter-panel" class="filter-panel-inline" role="region" aria-label="Filter">

          <div class="filter-section">
            <div class="filter-section-h">Schnellzugriff</div>
            <div class="pill-group">
              <button class="pill" class:on={filterUncategorized} type="button" onclick={pillUncategorized}>
                ⚠ Unkategorisiert
              </button>
              <button class="pill" type="button" onclick={pillThisMonth}>Diesen Monat</button>
              <button class="pill" type="button" onclick={pillLastMonth}>Letzter Monat</button>
              <button class="pill" class:on={filterMinAmount !== null} type="button" onclick={pillBig}>≥ 100 €</button>
            </div>
          </div>

          <div class="filter-section">
            <div class="filter-section-h">
              <span>Datum</span>
              {#if !filterFrom && !filterTo}
                <span class="filter-status muted">kein Filter aktiv</span>
              {:else}
                <button
                  type="button"
                  class="filter-status-reset"
                  onclick={() => { filterFrom = ''; filterTo = ''; }}
                  title="Datumsfilter zurücksetzen"
                >
                  Filter zurücksetzen
                </button>
              {/if}
            </div>
            <div class="filter-section-body">
              <label class="date-row">
                <span class="date-label">{t().common.from}</span>
                <DateField bind:value={filterFrom} />
              </label>
              <label class="date-row">
                <span class="date-label">{t().common.to}</span>
                <DateField bind:value={filterTo} />
              </label>
            </div>
          </div>

          <div class="filter-section">
            <div class="filter-section-h">Kategorie</div>
            <select class="input" bind:value={filterCat}>
              <option value="all">{t().common.all}</option>
              {#each categories.filter((c) => usedCategoryIds.has(c.id)) as c (c.id)}
                <option value={c.id}>{c.name}</option>
              {/each}
            </select>
          </div>

          <div class="filter-section">
            <div class="filter-section-h">Institution</div>
            <select class="input" bind:value={filterInstitution}>
              <option value="all">{t().common.all}</option>
              {#each institutions as inst (inst.id)}
                <option value={inst.id}>{inst.name}</option>
              {/each}
            </select>
          </div>

          <div class="filter-section">
            <div class="filter-section-h">Konto</div>
            <select class="input" bind:value={filterAcc}>
              <option value="all">{t().common.all}</option>
              {#each filteredAccountsForPicker as a (a.id)}
                <option value={a.id}>{a.name}</option>
              {/each}
            </select>
          </div>

          <div class="filter-section">
            <div class="filter-section-h">Topf</div>
            <select class="input" bind:value={filterBucket}>
              <option value="all">{(t().common as unknown as Record<string, string | undefined>).bucketAll ?? 'Alle Töpfe'}</option>
              {#each buckets as b (b.id)}
                <option value={b.id}>{b.name}</option>
              {/each}
            </select>
          </div>

        <div class="filter-panel-actions">
          <button type="button" class="btn ghost" onclick={clearFilters} disabled={!hasFilter}>
            Alle zurücksetzen
          </button>
          <button type="button" class="btn" onclick={() => (showFilterPanel = false)}>
            Schließen
          </button>
        </div>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="skel-list">
      {#each Array(8) as _, i (i)}
        <div class="skel-row">
          <Skeleton width="32" height="32" radius="50%" />
          <div class="skel-meta">
            <Skeleton width="60%" height="14" />
            <Skeleton width="40%" height="11" marginTop="6" />
          </div>
          <Skeleton width="80" height="14" />
        </div>
      {/each}
    </div>
  {:else if groupedFiltered.length === 0}
    <div class="empty">—</div>
  {:else}
    {#if selectedIds.size > 0}
      <div class="bulk-bar">
        <span class="bulk-count">{selectedIds.size} markiert</span>
        <select class="input" bind:value={bulkCatPick} disabled={bulkBusy} style="width: auto;">
          <option value="">→ Kategorie setzen…</option>
          {#each categories as c (c.id)}
            <option value={c.id}>{c.name}</option>
          {/each}
        </select>
        <select class="input" bind:value={bulkBucketPick} disabled={bulkBusy} style="width: auto;">
          <option value="">→ Topf setzen…</option>
          <option value="none">— kein Topf —</option>
          {#each buckets as b (b.id)}
            <option value={b.id}>{b.name}</option>
          {/each}
        </select>
        <button class="btn warn" type="button" onclick={bulkDelete} disabled={bulkBusy}>
          Löschen
        </button>
        <button class="btn ghost" type="button" onclick={clearSelection} disabled={bulkBusy}>
          Abwählen
        </button>
        {#if bulkError}
          <span class="bulk-err">{bulkError}</span>
        {/if}
      </div>
    {/if}

    {#each groupedFiltered as group (group.date)}
      <div class="date-group">
        <div class="tx-list-day-header">
          <label class="select-all" title="Alle Transaktionen dieses Tages auswählen / abwählen">
            <input
              type="checkbox"
              checked={group.txs.every((t) => selectedIds.has(t.id))}
              use:setIndeterminate={group.txs.some((t) => selectedIds.has(t.id)) && !group.txs.every((t) => selectedIds.has(t.id))}
              onchange={(e) => {
                const checked = (e.target as HTMLInputElement).checked;
                for (const t of group.txs) {
                  if (checked) selectedIds.add(t.id);
                  else selectedIds.delete(t.id);
                }
                selectedIds = new Set(selectedIds);
              }}
            />
          </label>
          <span>{new Intl.DateTimeFormat(settings.lang === 'de' ? 'de-DE' : 'en-US', { day: '2-digit', month: 'long' }).format(new Date(group.date + 'T12:00:00'))}</span>
          <span class="day-sum num" class:neg={group.totalCents < 0} class:pos={group.totalCents > 0}>
            {fmtEur(group.totalCents, { hide: settings.hide, signed: true })}
          </span>
        </div>
        {#each group.txs as tx (tx.id)}
          <div class="sel-row">
            <label class="select-cell" title="Diese Transaktion auswählen">
              <input
                type="checkbox"
                checked={selectedIds.has(tx.id)}
                onchange={() => toggleSelect(tx.id)}
              />
            </label>
            <div class="sel-row-tx">
              <TxRow
                {tx}
                {accounts}
                {categories}
                {bucketsById}
                {institutionsById}
                lang={settings.lang}
                hide={settings.hide}
                onclick={() => openEdit(tx)}
                viewAccountId={filterAcc !== 'all' ? (filterAcc as number) : undefined}
              />
            </div>
          </div>
        {/each}
      </div>
    {/each}

    {#if hasMore}
      <div class="load-more">
        <button class="btn" onclick={() => void loadMore()} disabled={loadingMore}>
          {loadingMore ? '…' : `${transactions.length} / ${aggregate.count} · weitere laden`}
        </button>
      </div>
    {/if}
  {/if}
</div>

{#if modalOpen}
  {#if modalTx && isTradeTx(modalTx)}
    <DepotTxModal
      tx={modalTx}
      {accounts}
      {categories}
      {bucketsById}
      onClose={closeModal}
      {onSaved}
      {onDeleted}
    />
  {:else}
    <TxModal
      tx={modalTx}
      {accounts}
      {categories}
      onClose={closeModal}
      {onSaved}
      {onDeleted}
      onCategoryCreated={(c) => (categories = [...categories, c])}
    />
  {/if}
{/if}

{#if tradeModalOpen}
  <TradeModal
    onClose={() => (tradeModalOpen = false)}
    onSaved={() => { tradeModalOpen = false; void loadFirstPage(); }}
  />
{/if}

{#if showImportModal}
  <ImportStatementsModal
    {accounts}
    {institutions}
    defaultAccountId={filterAcc === 'all' ? null : (filterAcc as number)}
    onClose={() => { showImportModal = false; }}
    onImported={() => { void loadFirstPage(); }}
  />
{/if}

<AccountCreateModal
  open={showCreateAccount}
  onClose={() => (showCreateAccount = false)}
  onCreated={() => { void loadMetadata(); void loadFirstPage(); }}
/>

<svelte:window onclick={(e) => {
  if (!newMenuOpen) return;
  const t = e.target as HTMLElement | null;
  if (t && !t.closest('.new-dropdown')) newMenuOpen = false;
}} />

<style>
  .load-more {
    display: flex;
    justify-content: center;
    padding: 16px 0;
  }
  .new-dropdown { position: relative; display: inline-block; }
  .chev { font-size: 10px; opacity: 0.7; margin-left: 4px; transition: transform 0.15s; display: inline-block; }
  .chev.open { transform: rotate(180deg); }
  .new-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    margin: 0; padding: 4px;
    list-style: none;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.15);
    min-width: 200px;
    z-index: 50;
  }
  .new-menu li { display: block; }
  .new-menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: 0;
    border-radius: 6px;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
  }
  .new-menu button:hover { background: var(--surface-2); }

  .pill {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-muted);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .pill:hover { color: var(--text); border-color: var(--text-faint); }
  .pill.on {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
    border-color: var(--accent);
  }
  .bulk-bar {
    display: flex; align-items: center; flex-wrap: wrap; gap: 8px;
    padding: 8px 12px; margin-bottom: 10px;
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    border: 1px solid var(--accent);
    border-radius: 6px;
    font-size: 12px;
  }
  .bulk-count { font-weight: 500; color: var(--accent); margin-right: 4px; }
  .bulk-bar .btn {
    padding: 4px 10px; border-radius: 4px; border: 1px solid var(--border);
    background: var(--surface-2); color: var(--text); cursor: pointer; font: inherit; font-size: 12px;
  }
  .bulk-bar .btn.warn { border-color: var(--negative); color: var(--negative); }
  .bulk-bar .btn.ghost { border: 0; background: transparent; color: var(--text-muted); }
  .bulk-err { color: var(--negative); margin-left: 4px; }
  .skel-list { display: grid; gap: 8px; padding: 8px 0; }
  .skel-row { display: grid; grid-template-columns: 32px 1fr 80px; gap: 12px; align-items: center; }
  .skel-meta { display: grid; min-width: 0; }
  .sel-row { display: grid; grid-template-columns: 32px 1fr; align-items: center; gap: 4px; }
  .select-cell, .select-all { display: flex; align-items: center; justify-content: center; }

  .select-cell input[type="checkbox"],
  .select-all input[type="checkbox"] {
    appearance: none;
    -webkit-appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1.5px solid var(--border);
    background: var(--surface-2);
    cursor: pointer;
    margin: 0;
    position: relative;
    transition: background 120ms, border-color 120ms, box-shadow 120ms;
  }
  .select-cell input[type="checkbox"]:hover,
  .select-all input[type="checkbox"]:hover {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 18%, transparent);
  }
  .select-cell input[type="checkbox"]:focus-visible,
  .select-all input[type="checkbox"]:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .select-cell input[type="checkbox"]:checked,
  .select-all input[type="checkbox"]:checked {
    background: var(--accent);
    border-color: var(--accent);
  }
  .select-cell input[type="checkbox"]:checked::after,
  .select-all input[type="checkbox"]:checked::after {
    content: "";
    position: absolute;
    inset: 0;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16' fill='none' stroke='white' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='3.5 8.5 7 12 12.5 5'/%3E%3C/svg%3E");
    background-size: 100% 100%;
    background-repeat: no-repeat;
  }
  .select-all input[type="checkbox"]:indeterminate {
    background: var(--accent);
    border-color: var(--accent);
  }
  .select-all input[type="checkbox"]:indeterminate::after {
    content: "";
    position: absolute;
    left: 25%;
    right: 25%;
    top: 50%;
    height: 2px;
    background: white;
    border-radius: 1px;
    transform: translateY(-50%);
  }
  .sel-row-tx { min-width: 0; }
  .filter-bar {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 16px;
  }
  .filter-bar-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }
  .search-wrap {
    position: relative;
    flex: 1 1 200px;
    min-width: 0;
  }
  .search-icon {
    position: absolute;
    left: 11px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-faint);
    display: inline-flex;
  }
  .empty {
    padding: 40px 0;
    text-align: center;
    color: var(--text-faint);
  }
  .date-group {
    margin-bottom: 12px;
  }
  .tx-summary {
    display: grid;
    grid-template-columns: 1fr 1px 1fr 1px 1fr;
    align-items: stretch;
    gap: 0;
    margin-bottom: 14px;
  }
  .tx-summary-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 4px 12px;
  }
  .tx-summary-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-faint);
  }
  .tx-summary-value {
    font-size: 22px;
    font-weight: 600;
    font-family: var(--font-mono);
    color: var(--text);
  }
  .tx-summary-value.pos { color: var(--positive); }
  .tx-summary-value.neg { color: var(--negative); }
  .tx-summary-divider {
    background: var(--border);
    margin: 8px 0;
  }
  /* Phone: three large currency values can't sit side by side — stack them
     into rows (label left, value right) and swap the vertical rules for
     horizontal ones. */
  @media (max-width: 599px) {
    .tx-summary {
      grid-template-columns: 1fr;
    }
    .tx-summary-col {
      flex-direction: row;
      justify-content: space-between;
      align-items: baseline;
      padding: 8px 4px;
    }
    .tx-summary-value {
      font-size: 18px;
    }
    .tx-summary-divider {
      height: 1px;
      margin: 0;
    }
  }
  .filter-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .filter-toggle.active {
    border-color: var(--accent);
    color: var(--accent);
  }
  .filter-panel-inline {
    width: 100%;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .filter-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .filter-section-h {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 8px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-faint);
  }
  .filter-status {
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.02em;
    text-transform: none;
  }
  .filter-status.muted { color: var(--text-faint); font-style: italic; }
  .filter-status-reset {
    border: 0;
    background: transparent;
    padding: 0;
    color: var(--accent);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: none;
    cursor: pointer;
  }
  .filter-status-reset:hover { text-decoration: underline; }
  .filter-section-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .pill-group {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .filter-panel-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }
  .filter-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    background: var(--accent);
    color: var(--accent-fg, white);
    font-size: 11px;
    font-weight: 600;
    border-radius: 9px;
    margin-left: 2px;
  }
  .date-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .date-label {
    font-size: 11px;
    color: var(--text-faint);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
</style>
