<script lang="ts">
  import { goto } from '$app/navigation';
  import { api, fmtEur, listInstitutionsWithSummary, type Account, type InstitutionSummary, errMsg} from '$lib/api';
  import AccountEditModal from '$lib/components/AccountEditModal.svelte';
  import AccountTreeItem from '$lib/components/AccountTreeItem.svelte';
  import ImportStatementsModal from '$lib/components/ImportStatementsModal.svelte';
  import InstitutionCard from '$lib/components/InstitutionCard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import KPI from '$lib/components/KPI.svelte';
  import { settings, t } from '$lib/settings.svelte';

  // i18n-Lookup ohne typed Property-Access — fehlende Keys fallen via ?? auf den Default zurück.
  const tx = () => t().common as unknown as Record<string, string | undefined>;

  let allAccounts = $state<Account[]>([]);
  let balances = $state<Map<number, number>>(new Map());
  let trends = $state<Map<number, number[]>>(new Map());
  let loading = $state(true);
  let error = $state<string | null>(null);

  let creating = $state(false);
  let newName = $state('');
  let newKind = $state('bank');
  let newInstitutionId = $state<number | null>(null);

  /** Kinds, die kein Bank-Institut tragen sollen. */
  const NON_INSTITUTION_KINDS = new Set(['cash', 'loan']);
  // Wenn der Benutzer auf cash/loan wechselt, Institut-Auswahl leeren.
  $effect(() => {
    if (NON_INSTITUTION_KINDS.has(newKind)) newInstitutionId = null;
  });

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

  // Tree-Top-Level: alle nicht-archivierten Konten ohne Parent (oder mit Parent außerhalb der Tree-Auswahl).
  const activeIds = $derived(new Set(activeAccounts.map((a) => a.id)));
  const activeTopLevel = $derived(
    activeAccounts.filter((a) => a.parent_id == null || !activeIds.has(a.parent_id)),
  );
  const archivedTopLevel = $derived(
    archivedAccounts.filter(
      (a) => a.parent_id == null || !archivedAccounts.some((x) => x.id === a.parent_id),
    ),
  );

  /** Gruppiert active-Top-Level-Konten nach kind (bank/savings/broker/cash/credit/loan). */
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
    // Stabile Reihenfolge: known kinds first, dann alphabetisch.
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

      // Trends: 6-Monats-Cashflow-Netto pro Top-Level-Konto (depth=0 zeigt's nur dort).
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

  async function createAccount(e: SubmitEvent) {
    e.preventDefault();
    if (!newName.trim()) return;
    try {
      await api.createAccount(newName.trim(), newKind, undefined, null, null, newInstitutionId);
      newName = '';
      newInstitutionId = null;
      creating = false;
      await loadAll();
    } catch (e) {
      error = errMsg(e);
    }
  }

  // pickAndImport / pickAndImportFlatex wurden durch das einheitliche
  // ImportStatementsModal ersetzt. Trigger: openImportModal(accountId?).
</script>

<div class="topbar">
  <div>
    <h1>{t().nav.accounts}</h1>
    <div class="sub">{allAccounts.length} · {t().common.synced}</div>
  </div>
  <label class="group-toggle">
    {t().accounts.groupBy}:
    <select bind:value={groupBy}>
      <option value="institution">{t().accounts.groupByInstitution}</option>
      <option value="flat">{t().accounts.groupByFlat}</option>
    </select>
  </label>
  <button class="btn" onclick={() => openImportModal(null)}>
    <Icon name="download" size={13} />
    {t().common.importStatements}
  </button>
  <button class="btn accent" onclick={() => (creating = !creating)}>
    <Icon name="plus" size={13} />
    {t().common.addAccount}
  </button>
</div>

{#if error}
  <div class="card" style="color:var(--negative); margin-bottom: 14px;">Fehler: {error}</div>
{/if}

<div class="kpi-grid" style="grid-template-columns: repeat(2, 1fr);">
  <KPI label={t().common.assets} value={fmtEur(totalCents, { hide: settings.hide })} />
  <KPI label={t().nav.accounts} value={String(activeAccounts.length)} />
</div>

{#if creating}
  <div class="card card-pad-lg" style="margin-bottom: 14px;">
    <form onsubmit={createAccount} class="new-form">
      <div class="field">
        <div class="field-label">{t().common.name}</div>
        <input class="input" bind:value={newName} placeholder="z.B. TR Verrechnung" required />
      </div>
      <div class="field" style="width: 180px;">
        <div class="field-label">{t().common.type}</div>
        <select class="input" bind:value={newKind}>
          <option value="bank">{kindLabel('bank')}</option>
          <option value="broker">{kindLabel('broker')}</option>
          <option value="savings">{kindLabel('savings')}</option>
          <option value="credit">{kindLabel('credit')}</option>
          <option value="cash">{kindLabel('cash')}</option>
          <option value="loan">{kindLabel('loan')}</option>
        </select>
      </div>
      <div class="field" style="width: 200px;">
        <div class="field-label">{t().common.institution}</div>
        <select
          class="input"
          value={newInstitutionId === null ? '' : String(newInstitutionId)}
          onchange={(e) => {
            const v = (e.currentTarget as HTMLSelectElement).value;
            newInstitutionId = v === '' ? null : Number(v);
          }}
          disabled={NON_INSTITUTION_KINDS.has(newKind)}
          title={NON_INSTITUTION_KINDS.has(newKind) ? t().common.institutionNone : undefined}
        >
          <option value="">{t().common.institutionNone}</option>
          {#each institutionSummaries as inst (inst.id)}
            <option value={String(inst.id)}>{inst.name}</option>
          {/each}
        </select>
      </div>
      <div class="form-actions">
        <button type="button" class="btn" onclick={() => (creating = false)}>
          {t().common.cancel}
        </button>
        <button type="submit" class="btn accent">{t().common.save}</button>
      </div>
    </form>
  </div>
{/if}

<div class="card card-pad-lg">
  <div class="card-h">
    <h3>{t().nav.accounts}</h3>
  </div>

  {#if loading}
    <div class="empty">…</div>
  {:else if allAccounts.length === 0}
    <div class="empty">
      {t().common.soon} — {t().common.addAccount}
    </div>
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
  .new-form {
    display: flex;
    gap: 12px;
    align-items: flex-end;
    flex-wrap: wrap;
  }
  .field {
    display: flex;
    flex-direction: column;
    flex: 1 1 200px;
  }
  .field-label {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-muted);
    margin-bottom: 5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .form-actions {
    display: flex;
    gap: 8px;
  }
  .empty {
    padding: 32px 0;
    text-align: center;
    color: var(--text-faint);
  }
  .tree {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .account-group { margin-bottom: 14px; }
  .account-group:last-child { margin-bottom: 0; }
  .group-h {
    margin: 0 0 6px 0;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    display: flex; align-items: center; gap: 8px;
  }
  .group-count {
    font-size: 10px; font-weight: 400; padding: 1px 6px;
    background: var(--surface-2); border-radius: 999px; color: var(--text-muted);
    text-transform: none; letter-spacing: 0;
  }
  .archive-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: 0;
    color: var(--text-muted);
    font-size: 13px;
    cursor: pointer;
    padding: 4px 0;
  }
  .group-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .group-toggle select {
    font-size: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
    padding: 3px 6px;
    cursor: pointer;
  }
  .unassigned-group {
    margin-top: 8px;
  }

  @media (max-width: 599px) {
    /* new-form already flex-wraps; ensure inputs have touch target */
    .new-form .input,
    .new-form select,
    .form-actions button {
      min-height: var(--tap, 44px);
    }
  }
</style>
