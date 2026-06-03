<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, listInstitutionsWithSummary, type Account, type InstitutionSummary, errMsg} from '$lib/api';
  import { fmtEur } from '$lib/format';
  import AccountEditModal from '$lib/components/AccountEditModal.svelte';
  import AccountCreateModal from '$lib/components/AccountCreateModal.svelte';
  import AccountTreeItem from '$lib/components/AccountTreeItem.svelte';
  import ImportStatementsModal from '$lib/components/ImportStatementsModal.svelte';
  import InstitutionCard from '$lib/components/InstitutionCard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import KPI from '$lib/components/KPI.svelte';
  import OverflowMenu from '$lib/components/OverflowMenu.svelte';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import { settings, t } from '$lib/settings.svelte';

  // i18n lookup without typed property access — missing keys fall back to the default via ??.
  const tx = () => t().common as unknown as Record<string, string | undefined>;

  let allAccounts = $state<Account[]>([]);
  let balances = $state<Map<number, number>>(new Map());
  let trends = $state<Map<number, number[]>>(new Map());
  let loading = $state(true);
  let error = $state<string | null>(null);

  let creating = $state(false);

  let showImportModal = $state(false);
  let importPrefilledAccountId = $state<number | null>(null);

  function openImportModal(accountId: number | null = null) {
    importPrefilledAccountId = accountId;
    showImportModal = true;
  }

  let editing = $state<Account | null>(null);
  let showArchived = $state(false);

  const STORAGE_KEY = 'accountsGroupBy';
  let groupBy = $state<'institution' | 'flat'>(
    typeof localStorage !== 'undefined'
      ? ((localStorage.getItem(STORAGE_KEY) as 'institution' | 'flat' | null) ?? 'institution')
      : 'institution',
  );
  let institutionSummaries = $state<InstitutionSummary[]>([]);

  $effect(() => {
    if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_KEY, groupBy);
  });

  const childrenByParent = $derived.by(() => {
    const map = new Map<number | null, Account[]>();
    for (const a of allAccounts) {
      const key = a.parent_id;
      const list = map.get(key) ?? [];
      list.push(a);
      map.set(key, list);
    }
    return map;
  });

  const activeAccounts = $derived(allAccounts.filter((a) => !a.archived));
  const archivedAccounts = $derived(allAccounts.filter((a) => a.archived));

  // Tree top level: all non-archived accounts without a parent (or whose parent is outside the tree selection).
  const activeIds = $derived(new Set(activeAccounts.map((a) => a.id)));
  const activeTopLevel = $derived(
    activeAccounts.filter((a) => a.parent_id == null || !activeIds.has(a.parent_id)),
  );
  const archivedTopLevel = $derived(
    archivedAccounts.filter(
      (a) => a.parent_id == null || !archivedAccounts.some((x) => x.id === a.parent_id),
    ),
  );

  /** Groups active top-level accounts by kind (bank/savings/broker/cash/credit/loan). */
  const KIND_ORDER: ReadonlyArray<string> = [
    'bank', 'savings', 'broker', 'cash', 'credit', 'loan',
  ];
  function kindLabel(kind: string): string {
    const tx = t().common as unknown as Record<string, string | undefined>;
    const key = `kind${kind.charAt(0).toUpperCase() + kind.slice(1)}`;
    return tx[key] ?? kind;
  }
  const activeGrouped = $derived.by(() => {
    const map = new Map<string, typeof activeTopLevel>();
    for (const a of activeTopLevel) {
      const arr = map.get(a.kind) ?? [];
      arr.push(a);
      map.set(a.kind, arr);
    }
    // Stable order: known kinds first, then alphabetically.
    const known = KIND_ORDER.filter((k) => map.has(k));
    const unknown = [...map.keys()].filter((k) => !KIND_ORDER.includes(k)).sort();
    return [...known, ...unknown].map((k) => ({
      kind: k,
      label: kindLabel(k),
      items: map.get(k) ?? [],
    }));
  });

  const totalCents = $derived(
    activeTopLevel.reduce((s, a) => s + (balances.get(a.id) ?? 0), 0),
  );

  async function loadInstitutions() {
    institutionSummaries = await listInstitutionsWithSummary();
  }

  async function loadAll() {
    loading = true;
    try {
      allAccounts = await api.listAccounts();
      await loadInstitutions();
      const entries = await Promise.all(
        allAccounts.map(async (a) => [a.id, await api.accountBalance(a.id)] as const),
      );
      balances = new Map(entries);

      // Trends: 6-month net cashflow per top-level account (depth=0 shows it there only).
      const now = new Date();
      const trendEntries = await Promise.all(
        allAccounts
          .filter((a) => !a.archived && a.parent_id == null)
          .map(async (a) => {
            const flow = await api.accountMonthlyCashflow(a.id, now.getFullYear(), now.getMonth() + 1, 6);
            const series = flow.map((f) => f.inCents - f.outCents);
            return [a.id, series] as const;
          }),
      );
      trends = new Map(trendEntries);
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadAll();
  });

  // pickAndImport / pickAndImportFlatex were replaced by the unified
  // ImportStatementsModal. Trigger: openImportModal(accountId?).
</script>

<div class="topbar">
  <div>
    <h1>{t().nav.accounts}</h1>
    <div class="sub">{allAccounts.length} · {t().common.synced}</div>
  </div>
  <OverflowMenu>
    <div class="overflow-group-select">
      <span class="overflow-group-label">{t().accounts.groupBy}</span>
      <select bind:value={groupBy}>
        <option value="institution">{t().accounts.groupByInstitution}</option>
        <option value="flat">{t().accounts.groupByFlat}</option>
      </select>
    </div>
    <button onclick={() => { openImportModal(null); }}>
      <Icon name="download" size={16} />
      {t().common.importStatements}
    </button>
  </OverflowMenu>
  <button class="btn accent" onclick={() => (creating = true)}>
    <Icon name="plus" size={13} />
    {t().common.addAccount}
  </button>
</div>

<div aria-live="polite" aria-atomic="true">
  {#if error}
    <div class="card error-banner"><Icon name="alert-circle" size={16} /> Fehler: {error}</div>
  {/if}
</div>

<div class="kpi-grid" style="grid-template-columns: repeat(2, 1fr);">
  <KPI label={t().common.assets} value={fmtEur(totalCents, { hide: settings.hide })} />
  <KPI label={t().nav.accounts} value={String(activeAccounts.length)} />
</div>

<div class="card card-pad-lg">
  <div class="card-h">
    <h3>{t().nav.accounts}</h3>
  </div>

  {#if loading}
    <div class="skeleton-list">
      <Skeleton height={18} marginTop={0} />
      <Skeleton height={18} marginTop={8} />
      <Skeleton height={18} marginTop={8} />
      <Skeleton height={18} marginTop={8} />
    </div>
  {:else if allAccounts.length === 0}
    <EmptyState
      icon="accounts"
      title="Noch keine Konten"
      description="Lege dein erstes Konto an, um Buchungen zu erfassen."
      actionLabel={t().common.addAccount}
      onAction={() => (creating = true)}
    />
  {:else if groupBy === 'institution'}
    {#each institutionSummaries as inst (inst.id)}
      {@const accountsInGroup = activeTopLevel.filter((a) => a.institution_id === inst.id)}
      {#if accountsInGroup.length > 0}
        <div class="account-group">
          <InstitutionCard institution={inst}>
            <div class="tree">
              {#each accountsInGroup as node (node.id)}
                <AccountTreeItem
                  {node}
                  children={childrenByParent.get(node.id) ?? []}
                  {childrenByParent}
                  {balances}
                  {trends}
                  onOpen={(id) => goto(`/accounts/${id}`)}
                  onEdit={(a) => (editing = a)}
                  onImport={openImportModal}
                />
              {/each}
            </div>
          </InstitutionCard>
        </div>
      {/if}
    {/each}
    {@const unassigned = activeTopLevel.filter((a) => a.institution_id === null || !institutionSummaries.some((s) => s.id === a.institution_id))}
    {#if unassigned.length > 0}
      <div class="account-group unassigned-group">
        <h4 class="group-h">{t().accounts.withoutInstitution}</h4>
        <div class="tree">
          {#each unassigned as node (node.id)}
            <AccountTreeItem
              {node}
              children={childrenByParent.get(node.id) ?? []}
              {childrenByParent}
              {balances}
              {trends}
              onOpen={(id) => goto(`/accounts/${id}`)}
              onEdit={(a) => (editing = a)}
              onImport={openImportModal}
            />
          {/each}
        </div>
      </div>
    {/if}
  {:else}
    {#each activeGrouped as group (group.kind)}
      {#if group.items.length > 0}
        <div class="account-group">
          <h4 class="group-h">{group.label} <span class="group-count">{group.items.length}</span></h4>
          <div class="tree">
            {#each group.items as node (node.id)}
              <AccountTreeItem
                {node}
                children={childrenByParent.get(node.id) ?? []}
                {childrenByParent}
                {balances}
                {trends}
                onOpen={(id) => goto(`/accounts/${id}`)}
                onEdit={(a) => (editing = a)}
                onImport={openImportModal}
              />
            {/each}
          </div>
        </div>
      {/if}
    {/each}
  {/if}
</div>

{#if archivedTopLevel.length > 0}
  <div class="card card-pad-lg" style="margin-top: 14px;">
    <button class="archive-toggle" onclick={() => (showArchived = !showArchived)} type="button">
      <Icon name={showArchived ? 'chevron-down' : 'chevron-right'} size={12} />
      {tx().archived ?? 'Archiviert'} ({archivedAccounts.length})
    </button>
    {#if showArchived}
      <div class="tree" style="margin-top: 8px;">
        {#each archivedTopLevel as node (node.id)}
          <AccountTreeItem
            {node}
            children={childrenByParent.get(node.id) ?? []}
            {childrenByParent}
            {balances}
            onOpen={(id) => goto(`/accounts/${id}`)}
          onEdit={(a) => (editing = a)}
          onImport={openImportModal}
          />
        {/each}
      </div>
    {/if}
  </div>
{/if}

{#if editing}
  <AccountEditModal
    account={editing}
    onClose={() => (editing = null)}
    onSaved={(updated) => {
      allAccounts = allAccounts.map((a) => (a.id === updated.id ? updated : a));
    }}
  />
{/if}

<AccountCreateModal
  open={creating}
  onClose={() => (creating = false)}
  onCreated={() => loadAll()}
/>

{#if showImportModal}
  <ImportStatementsModal
    accounts={allAccounts}
    institutions={institutionSummaries}
    defaultAccountId={importPrefilledAccountId}
    onClose={() => { showImportModal = false; }}
    onImported={() => { loadAll(); }}
  />
{/if}

<style>
  .error-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--negative);
    margin-bottom: 14px;
  }
  .skeleton-list {
    padding: 16px 0;
  }
  .empty {
    padding: 32px 0;
    text-align: center;
    color: var(--text-faint);
  }
  .empty-cta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
  }
  .empty-cta p {
    margin: 0;
  }
  .tree {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .account-group { margin-bottom: 14px; }
  .account-group:last-child { margin-bottom: 0; }
  .group-h {
    margin: 0 0 8px 0;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    display: flex; align-items: center; gap: 8px;
  }
  .group-count {
    font-size: 10px; font-weight: 400; padding: 4px 8px;
    background: var(--surface-2); border-radius: 999px; color: var(--text-muted);
    text-transform: none; letter-spacing: 0;
  }
  .archive-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: 0;
    color: var(--text-muted);
    font-size: 13px;
    cursor: pointer;
    padding: 4px 0;
  }
  .unassigned-group {
    margin-top: 8px;
  }
  .overflow-group-select {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px;
    min-height: 48px;
    justify-content: center;
  }
  .overflow-group-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-faint);
  }
  .overflow-group-select select {
    font-size: 13px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    color: var(--text);
    padding: 4px 8px;
    cursor: pointer;
    width: 100%;
  }
</style>
