<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import {
    api,
    type Account,
    type Bucket,
    type MonthlyFlow,
    type Transaction,
    type Category,
    errMsg,
    isTradeTx} from '$lib/api';
  import { fmtEur } from '$lib/format';
  import AccountSettingsForm from '$lib/components/AccountSettingsForm.svelte';
  import CashflowChart from '$lib/components/CashflowChart.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import KPI from '$lib/components/KPI.svelte';
  import TxRow from '$lib/components/TxRow.svelte';
  import TxModal from '$lib/components/TxModal.svelte';
  import DepotTxModal from '$lib/components/DepotTxModal.svelte';
  import { settings, t } from '$lib/settings.svelte';

  // i18n lookup without typed property access — missing keys fall back to the default via ??.
  const tx = () => t().common as unknown as Record<string, string | undefined>;

  function kindLabel(k: string): string {
    const c = tx();
    const map: Record<string, string> = {
      bank: c.kindBank ?? 'Bank',
      broker: c.kindBroker ?? 'Depot',
      savings: c.kindSavings ?? 'Tagesgeld',
      credit: c.kindCredit ?? 'Kreditkarte',
      cash: c.kindCash ?? 'Bargeld',
      loan: c.kindLoan ?? 'Schuld / Darlehen',
    };
    return map[k] ?? k;
  }

  const accountId = $derived(parseInt(page.params.id ?? '', 10));

  let account = $state<Account | null>(null);
  let balance = $state(0);
  let txs = $state<Transaction[]>([]);
  let cashflow = $state<MonthlyFlow[]>([]);
  let categories = $state<Category[]>([]);
  let buckets = $state<Bucket[]>([]);
  const bucketsById = $derived(new Map(buckets.map((b) => [b.id, b])));
  let modalOpen = $state(false);
  let editingTx = $state<Transaction | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let search = $state('');
  let allAccounts = $state<Account[]>([]);
  let childBalances = $state<Map<number, number>>(new Map());

  const now = new Date();
  const curYear = now.getFullYear();
  const curMonth = now.getMonth() + 1;

  async function loadAll(id: number) {
    loading = true;
    error = null;
    try {
      const [acc, bal, list, cf, cats, bks] = await Promise.all([
        api.getAccount(id),
        api.accountBalance(id),
        api.listTransactions({ accountId: id, limit: 5000 }),
        api.accountMonthlyCashflow(id, curYear, curMonth, 6),
        api.listCategories(),
        api.listBuckets(true),
      ]);
      account = acc;
      balance = bal;
      txs = list.rows;
      cashflow = cf;
      categories = cats;
      buckets = bks;
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const id = accountId;
    if (!Number.isFinite(id)) return;
    void loadAll(id);
  });

  $effect(() => {
    if (!account) return;
    const parentId = account.id;
    api.listAccounts().then(async (list) => {
      allAccounts = list;
      const kids = list.filter((a) => a.parent_id === parentId);
      const entries = await Promise.all(
        kids.map(async (k) => [k.id, await api.accountBalance(k.id)] as const),
      );
      childBalances = new Map(entries);
    });
  });

  const children = $derived(allAccounts.filter((a) => account && a.parent_id === account.id));

  const thisMonth = $derived(cashflow[cashflow.length - 1]);
  const inCents = $derived(thisMonth?.inCents ?? 0);
  const outCents = $derived(thisMonth?.outCents ?? 0);
  const netCents = $derived(inCents - outCents);

  const filteredTxs = $derived.by(() => {
    const q = search.trim().toLowerCase();
    if (!q) return txs;
    return txs.filter(
      (t) =>
        (t.counterparty?.toLowerCase().includes(q) ?? false) ||
        (t.purpose?.toLowerCase().includes(q) ?? false) ||
        (t.manual_note?.toLowerCase().includes(q) ?? false),
    );
  });

  const chartData = $derived(
    cashflow.map((f) => ({
      m: `${String(f.month).padStart(2, '0')}/${String(f.year).slice(2)}`,
      in: f.inCents / 100,
      out: f.outCents / 100,
    })),
  );

  async function handleSave(draft: Account) {
    await api.updateAccount(draft);
    account = draft;
  }

  function openEdit(tx: Transaction) {
    editingTx = tx;
    modalOpen = true;
  }
  function openNew() {
    editingTx = null;
    modalOpen = true;
  }
  function closeModal() {
    modalOpen = false;
    editingTx = null;
  }
  function onSaved(updated: Transaction) {
    const idx = txs.findIndex((t) => t.id === updated.id);
    if (idx >= 0) txs[idx] = updated;
    closeModal();
    void loadAll(accountId);
  }
  function onDeleted(id: number) {
    txs = txs.filter((t) => t.id !== id);
    closeModal();
    void loadAll(accountId);
  }
</script>

<button class="btn back" onclick={() => goto('/accounts')}>
  <span class="back-arrow"><Icon name="chevron-right" size={12} /></span> {t().nav.accounts}
</button>

{#if error}
  <div class="card" style="color: var(--negative); margin-bottom: 14px;">Fehler: {error}</div>
{/if}

{#if loading || !account}
  <div class="empty">…</div>
{:else}
  {@const bg = account.color ?? 'oklch(0.55 0.13 230)'}
  <div class="card card-pad-lg header">
    <div class="acc-logo" style:background={bg} style:color="#fff">
      {#if account.icon}
        <Icon name={account.icon} size={20} />
      {:else}
        {account.name.slice(0, 2).toUpperCase()}
      {/if}
    </div>
    <div class="header-body">
      <h1>
        {account.name}
        {#if account.archived}
          <span class="pill archived">{tx().archived ?? 'Archiviert'}</span>
        {/if}
      </h1>
      <div class="sub">
        {kindLabel(account.kind)} · {account.currency}
        {#if account.last4} · ·· {account.last4}{/if}
        · {txs.length} {t().nav.transactions}
      </div>
      {#if account.iban}
        <div class="iban-block">
          <span class="label">{t().common.iban}</span>
          <span class="iban mono">{account.iban}</span>
          <button
            class="copy"
            type="button"
            onclick={() => account?.iban && navigator.clipboard.writeText(account.iban)}
            aria-label="copy"
          >
            <Icon name="card" size={14} />
          </button>
        </div>
      {/if}
      {#if account.note}
        <div class="note">{account.note}</div>
      {/if}
    </div>
    <div class="balance">
      <div class="num bal">{fmtEur(balance, { hide: settings.hide })}</div>
      <div class="balance-label">{t().common.balance}</div>
    </div>
  </div>

  <div class="kpi-grid" style="grid-template-columns: repeat(3, 1fr); margin-top: 14px;">
    <KPI label={t().common.income} value={fmtEur(inCents, { hide: settings.hide })} />
    <KPI label={t().common.expenses} value={fmtEur(outCents, { hide: settings.hide })} />
    <KPI label={t().common.net} value={fmtEur(netCents, { hide: settings.hide, signed: true })} />
  </div>

  <div class="card card-pad-lg" style="margin-top: 14px;">
    <div class="card-h">
      <h3>{tx().trend ?? 'Trend (6 Monate)'}</h3>
    </div>
    <CashflowChart data={chartData} height={160} hide={settings.hide} />
  </div>

  {#if children.length > 0}
    <section class="card card-pad-lg" style="margin-top: 14px;">
      <div class="card-h">
        <h3>{tx().subAccounts ?? 'Subkonten'}</h3>
      </div>
      {#each children as c (c.id)}
        <button class="subaccount-row" type="button" onclick={() => goto(`/accounts/${c.id}`)}>
          <span class="sub-icon" style:color={c.color ?? 'inherit'}>
            <Icon name={c.icon ?? 'wallet'} size={14} />
          </span>
          <span class="name">{c.name}</span>
          <span class="balance num">{fmtEur(childBalances.get(c.id) ?? 0, { hide: settings.hide })}</span>
        </button>
      {/each}
      <p class="hint">{tx().subAccountsHint ?? 'Subkonten haben eigene Transaktionen.'}</p>
    </section>
  {/if}

  <div class="card card-pad-lg" style="margin-top: 14px;">
    <div class="card-h">
      <h3>{t().nav.transactions}</h3>
      <div class="head-actions">
        <input
          class="input"
          type="search"
          placeholder={t().common.search}
          bind:value={search}
          style="max-width: 220px;"
        />
        <button class="btn primary" type="button" onclick={openNew}>
          <Icon name="plus" size={13} /> {tx().add ?? 'Hinzufügen'}
        </button>
      </div>
    </div>
    {#if filteredTxs.length === 0}
      <div class="empty">–</div>
    {:else}
      {#each filteredTxs as t (t.id)}
        <TxRow
          tx={t}
          accounts={allAccounts}
          {categories}
          {bucketsById}
          lang={settings.lang}
          hide={settings.hide}
          hideAccount
          viewAccountId={account.id}
          onclick={() => openEdit(t)}
        />
      {/each}
    {/if}
  </div>

  <div class="card card-pad-lg" style="margin-top: 14px;">
    <div class="card-h"><h3>{t().nav.settings}</h3></div>
    <AccountSettingsForm account={account} onSave={handleSave} />
  </div>
{/if}

{#if modalOpen}
  {#if editingTx && isTradeTx(editingTx)}
    <DepotTxModal
      tx={editingTx}
      accounts={allAccounts}
      {categories}
      {bucketsById}
      onClose={closeModal}
      {onSaved}
      {onDeleted}
    />
  {:else}
    <TxModal
      tx={editingTx}
      accounts={allAccounts}
      {categories}
      onClose={closeModal}
      {onSaved}
      {onDeleted}
      defaultAccountId={account?.id ?? null}
    />
  {/if}
{/if}

<style>
  .back {
    margin-bottom: 14px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .back-arrow {
    display: inline-flex;
    align-items: center;
    transform: rotate(180deg);
  }
  .header { display: flex; gap: 16px; align-items: center; }
  .acc-logo {
    width: 56px; height: 56px;
    border-radius: 14px;
    display: grid; place-items: center;
    font-weight: 600;
    flex-shrink: 0;
  }
  .header-body { flex: 1; min-width: 0; }
  .header-body h1 { margin: 0; display: flex; align-items: center; gap: 10px; }
  .sub { font-size: 12px; color: var(--text-faint); margin-top: 4px; }
  .note { font-size: 13px; color: var(--text-muted); margin-top: 8px; }
  .balance { text-align: right; }
  .bal { font-size: 22px; font-weight: 600; }
  .balance-label { font-size: 11px; color: var(--text-faint); }
  .pill {
    display: inline-block;
    font-size: 10.5px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .empty { padding: 24px; text-align: center; color: var(--text-faint); font-size: 13px; }
  .iban-block {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0 0;
    font-size: 13px;
  }
  .iban-block .label { color: var(--text-muted); }
  .iban-block .iban {
    font-family: ui-monospace, monospace;
    letter-spacing: 0.02em;
  }
  .iban-block .copy {
    background: transparent;
    border: 0;
    cursor: pointer;
    color: var(--text-muted);
    display: inline-grid;
    place-items: center;
    padding: 2px 4px;
    border-radius: 4px;
  }
  .iban-block .copy:hover { color: var(--text); background: var(--surface-2); }
  .subaccount-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    cursor: pointer;
    border-radius: 8px;
    background: transparent;
    border: 0;
    text-align: left;
    color: inherit;
  }
  .subaccount-row:hover { background: var(--surface-2); }
  .subaccount-row .sub-icon {
    display: inline-grid;
    place-items: center;
    width: 18px;
    flex-shrink: 0;
  }
  .subaccount-row .name {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subaccount-row .balance {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 13.5px;
  }
  .hint {
    font-size: 12px;
    color: var(--text-faint);
    margin: 10px 4px 0;
  }
  .head-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }
</style>
