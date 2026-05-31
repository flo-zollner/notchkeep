<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import { api, listInstitutionsWithSummary, type Account, type Bucket, type Category, type Transaction, type InstitutionSummary, errMsg, isTradeTx} from '$lib/api';
  import { fmtEur } from '$lib/format';
  import Icon from '$lib/components/Icon.svelte';
  import KPI from '$lib/components/KPI.svelte';
  import TxRow from '$lib/components/TxRow.svelte';
  import TxModal from '$lib/components/TxModal.svelte';
  import DepotTxModal from '$lib/components/DepotTxModal.svelte';
  import ImportStatementsModal from '$lib/components/ImportStatementsModal.svelte';
  import SavingsRateChart, { type SavingsChartMode } from '$lib/components/SavingsRateChart.svelte';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import { settings, t } from '$lib/settings.svelte';
  import { groupByDay } from '$lib/tx-grouping';
  import DateField from '$lib/components/DateField.svelte';

  let transactions = $state<Transaction[]>([]);
  let accounts = $state<Account[]>([]);
  let categories = $state<Category[]>([]);
  let buckets = $state<Bucket[]>([]);
  const bucketsById = $derived(new Map(buckets.map((b) => [b.id, b])));
  let loading = $state(true);
  let error = $state<string | null>(null);
  let monthlyFlows = $state<Array<{ year: number; month: number; inCents: number; outCents: number }>>([]);
  let institutionSummaries = $state<InstitutionSummary[]>([]);
  let accountBalances = $state<Record<number, number>>({});
  let portfolioCents = $state(0);
  let showImportModal = $state(false);

  const SAVINGS_MODE_KEY = 'saldo.savings.mode';
  function loadSavingsMode(): SavingsChartMode {
    if (typeof localStorage === 'undefined') return 'line';
    const v = localStorage.getItem(SAVINGS_MODE_KEY);
    return v === 'bars' || v === 'line' || v === 'inout' ? v : 'line';
  }
  let savingsMode = $state<SavingsChartMode>(loadSavingsMode());
  function setSavingsMode(m: SavingsChartMode) {
    savingsMode = m;
    if (typeof localStorage !== 'undefined') localStorage.setItem(SAVINGS_MODE_KEY, m);
  }

  const SAVINGS_EXCL_INV_KEY = 'saldo.savings.excl_inv';
  function loadSavingsExclInv(): boolean {
    if (typeof localStorage === 'undefined') return false;
    return localStorage.getItem(SAVINGS_EXCL_INV_KEY) === 'true';
  }
  let savingsExclInv = $state<boolean>(loadSavingsExclInv());
  function toggleSavingsExclInv() {
    savingsExclInv = !savingsExclInv;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(SAVINGS_EXCL_INV_KEY, savingsExclInv ? 'true' : 'false');
    }
  }

  type RangePreset = '1m' | '12m' | 'ytd' | 'custom';
  const RANGE_KEY = 'saldo.overview.range';

  function todayStr(): string {
    const d = new Date();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${d.getFullYear()}-${m}-${day}`;
  }
  function startOfCurrentMonth(): string {
    const d = new Date();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    return `${d.getFullYear()}-${m}-01`;
  }

  function loadRange(): { preset: RangePreset; from: string; to: string } {
    const def = { preset: '1m' as RangePreset, from: startOfCurrentMonth(), to: todayStr() };
    if (typeof localStorage === 'undefined') return def;
    try {
      const raw = localStorage.getItem(RANGE_KEY);
      if (!raw) return def;
      const p = JSON.parse(raw) as Partial<{ preset: RangePreset; from: string; to: string }>;
      const preset: RangePreset =
        p.preset === '12m' || p.preset === 'ytd' || p.preset === 'custom' ? p.preset : '1m';
      return { preset, from: p.from || def.from, to: p.to || def.to };
    } catch {
      return def;
    }
  }
  const _loaded = loadRange();
  let rangePreset = $state<RangePreset>(_loaded.preset);
  let customFrom = $state(_loaded.from);
  let customTo = $state(_loaded.to);

  function persistRange() {
    if (typeof localStorage === 'undefined') return;
    localStorage.setItem(
      RANGE_KEY,
      JSON.stringify({ preset: rangePreset, from: customFrom, to: customTo })
    );
  }
  function setRangePreset(p: RangePreset) {
    rangePreset = p;
    if (p === 'custom') {
      if (!customFrom) customFrom = startOfCurrentMonth();
      if (!customTo) customTo = todayStr();
    }
    persistRange();
  }
  function onCustomChange() {
    if (customFrom && customTo && customFrom > customTo) {
      customTo = customFrom;
    }
    persistRange();
  }

  const range = $derived.by(() => {
    const today = todayStr();
    if (rangePreset === '12m') {
      const d = new Date();
      d.setMonth(d.getMonth() - 11);
      const from = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-01`;
      return { from, to: today };
    }
    if (rangePreset === 'ytd') {
      const d = new Date();
      return { from: `${d.getFullYear()}-01-01`, to: today };
    }
    if (rangePreset === 'custom') {
      return { from: customFrom || startOfCurrentMonth(), to: customTo || today };
    }
    return { from: startOfCurrentMonth(), to: today };
  });

  function fmtDateLabel(s: string): string {
    if (!s) return '';
    const [y, m, d] = s.split('-');
    const months = ['Jan', 'Feb', 'Mär', 'Apr', 'Mai', 'Jun', 'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dez'];
    return `${parseInt(d)}. ${months[parseInt(m) - 1]} ${y}`;
  }
  const rangeLabel = $derived(`${fmtDateLabel(range.from)} – ${fmtDateLabel(range.to)}`);

  async function loadCashflow() {
    try {
      const now = new Date();
      monthlyFlows = await api.monthlyCashflow(now.getFullYear(), now.getMonth() + 1, 12, savingsExclInv);
    } catch { /* ignore */ }
  }

  $effect(() => {
    void savingsExclInv;  // reload when toggle changes
    void loadCashflow();
  });

  const avgSavingsRate12m = $derived.by(() => {
    const rates = monthlyFlows
      .filter((f) => f.inCents > 0)
      .map((f) => ((f.inCents - f.outCents) / f.inCents) * 100);
    if (rates.length === 0) return 0;
    return rates.reduce((s, r) => s + r, 0) / rates.length;
  });

  // Static data (accounts/categories/buckets/net-worth inputs) is loaded once;
  // `transactions` is reloaded server-side filtered to the current range.
  // `recentTx` is a separate "top 6 most recent" list independent of the range.
  let recentTx = $state<Transaction[]>([]);

  async function loadStatic() {
    loading = true;
    error = null;
    try {
      const [accs, cats, bks, recent] = await Promise.all([
        api.listAccounts(),
        api.listCategories(),
        api.listBuckets(true),
        api.listTransactions({ limit: 6 }),
      ]);
      accounts = accs;
      categories = cats;
      buckets = bks;
      recentTx = recent.rows;
      await loadNetWorthInputs();
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  async function loadRangeTx() {
    try {
      const page = await api.listTransactions({
        from: range.from,
        to: range.to,
        limit: 5000,
      });
      transactions = page.rows;
    } catch (e) {
      error = errMsg(e);
    }
  }

  $effect(() => {
    void loadStatic();
  });

  // Range change → reload range-specific transactions server-side.
  $effect(() => {
    void range.from;
    void range.to;
    void loadRangeTx();
  });

  $effect(() => {
    void loadInstitutions();
  });

  async function loadInstitutions() {
    try {
      institutionSummaries = await listInstitutionsWithSummary();
    } catch { /* ignore */ }
  }

  async function loadNetWorthInputs() {
    try {
      const activeAccs = accounts.filter((a) => !a.archived);
      const pairs = await Promise.all(
        activeAccs.map(async (a) => [a.id, await api.accountBalance(a.id)] as const)
      );
      accountBalances = Object.fromEntries(pairs);
      const now = new Date();
      const pk = await api.portfolioKpis(now.getFullYear());
      portfolioCents = pk.marketValueCents;
    } catch {
      /* ignore — KPI shows 0 on failure */
    }
  }

  // After a background FX/price refresh (on startup or import), reload data
  // so account balances and institution totals use up-to-date FX rates.
  $effect(() => {
    type Status = { stage: 'started' | 'completed' | 'failed' };
    const unlisten = listen<Status>('price_refresh_status', (e) => {
      if (e.payload.stage === 'completed') {
        void loadStatic();
        void loadRangeTx();
        void loadInstitutions();
      }
    });
    return () => { unlisten.then((u) => u()); };
  });

  const EXCLUDED_KINDS = new Set(['transfer', 'corporate_action']);
  function isCashflowKind(k: string): boolean {
    return !EXCLUDED_KINDS.has(k);
  }

  const txInRange = $derived(
    transactions.filter((tx) => tx.booking_date >= range.from && tx.booking_date <= range.to)
  );

  const income = $derived(
    txInRange.filter((x) => isCashflowKind(x.kind) && x.amount_cents > 0).reduce((s, x) => s + x.amount_cents, 0)
  );
  const expenses = $derived(
    -txInRange.filter((x) => isCashflowKind(x.kind) && x.amount_cents < 0).reduce((s, x) => s + x.amount_cents, 0)
  );
  const net = $derived(income - expenses);
  const savingsRate = $derived(income > 0 ? (net / income) * 100 : 0);

  // Investment rate: what share of income flowed net into investments.
  // buys (amount_cents negative) → absolute sum; sells (amount_cents positive) → direct;
  // dividends are passive income, NOT part of the reallocation (already counted in income).
  const investBuysCents = $derived(
    -txInRange.filter((x) => x.kind === 'buy').reduce((s, x) => s + x.amount_cents, 0)
  );
  const investSellsCents = $derived(
    txInRange.filter((x) => x.kind === 'sell').reduce((s, x) => s + x.amount_cents, 0)
  );
  const netInvestedCents = $derived(investBuysCents - investSellsCents);
  const investmentRate = $derived(income > 0 ? (netInvestedCents / income) * 100 : 0);

  // Savings rate excluding investments: strips buy/sell from income/expenses.
  // Logic: if you earn 1000 € and invest 500 € in ETFs, that is
  // "500 € saved", not "1000 € earned & 500 € spent → 50%".
  const incomeExclInv = $derived(income - investSellsCents);
  const expensesExclInv = $derived(expenses - investBuysCents);
  const netExclInv = $derived(incomeExclInv - expensesExclInv);
  const savingsRateExclInv = $derived(
    incomeExclInv > 0 ? (netExclInv / incomeExclInv) * 100 : 0,
  );
  const displaySavingsRate = $derived(savingsExclInv ? savingsRateExclInv : savingsRate);
  const displaySavingsIncome = $derived(savingsExclInv ? incomeExclInv : income);

  const cashOnlyCents = $derived(
    accounts.filter((a) => !a.archived).reduce((s, a) => s + (accountBalances[a.id] ?? 0), 0)
  );
  const netWorthCents = $derived(cashOnlyCents + portfolioCents);

  const recent = $derived(recentTx);

  let modalOpen = $state(false);
  let modalTx = $state<Transaction | null>(null);

  function openEdit(tx: Transaction) {
    modalTx = tx;
    modalOpen = true;
  }
  function closeModal() {
    modalOpen = false;
  }
  function onSaved(_saved: Transaction) {
    // Pragmatic: reload both after edit (range transactions + recent).
    void loadRangeTx();
    void api.listTransactions({ limit: 6 }).then((p) => (recentTx = p.rows));
  }
  function onDeleted(_id: number) {
    void loadRangeTx();
    void api.listTransactions({ limit: 6 }).then((p) => (recentTx = p.rows));
  }

  const topCats = $derived.by(() => {
    const totals = new Map<number, number>();
    for (const tx of txInRange.filter((x) => isCashflowKind(x.kind) && x.amount_cents < 0 && x.category_id)) {
      totals.set(tx.category_id!, (totals.get(tx.category_id!) ?? 0) + Math.abs(tx.amount_cents));
    }
    return Array.from(totals.entries())
      .map(([id, cents]) => ({
        category: categories.find((c) => c.id === id),
        cents,
      }))
      .filter((x) => x.category)
      .sort((a, b) => b.cents - a.cents)
      .slice(0, 4);
  });
</script>

<div class="topbar">
  <div>
    <h1>{t().nav.overview}</h1>
    <div class="sub">{t().common.yourMoney}</div>
  </div>
  <button class="btn" data-tour="import" onclick={() => (showImportModal = true)} disabled={accounts.filter((a) => !a.archived).length === 0}>
    <Icon name="arrow-up" size={13} />
    {t().common.importStatements}
  </button>
</div>

{#if error}
  <div class="card" style="color:var(--negative); margin-bottom: 14px;">Fehler: {error}</div>
{/if}

<div class="range-bar">
  <div class="seg" role="tablist" aria-label="Zeitraum">
    <button type="button" role="tab" aria-selected={rangePreset === '1m'}
      class:active={rangePreset === '1m'} onclick={() => setRangePreset('1m')}>1M</button>
    <button type="button" role="tab" aria-selected={rangePreset === '12m'}
      class:active={rangePreset === '12m'} onclick={() => setRangePreset('12m')}>12M</button>
    <button type="button" role="tab" aria-selected={rangePreset === 'ytd'}
      class:active={rangePreset === 'ytd'} onclick={() => setRangePreset('ytd')}>YTD</button>
    <button type="button" role="tab" aria-selected={rangePreset === 'custom'}
      class:active={rangePreset === 'custom'} onclick={() => setRangePreset('custom')}>Custom</button>
  </div>
  {#if rangePreset === 'custom'}
    <div class="custom-dates">
      <DateField bind:value={customFrom} onChange={onCustomChange} max={customTo || todayStr()} />
      <span class="dash">–</span>
      <DateField bind:value={customTo} onChange={onCustomChange} min={customFrom} max={todayStr()} />
    </div>
  {:else}
    <span class="range-label">{rangeLabel}</span>
  {/if}
</div>

<div class="kpi-grid" data-tour="dashboard-kpis">
  <KPI label={t().common.netWorth} value={fmtEur(netWorthCents, { hide: settings.hide })} />
  <KPI label={t().common.income} value={fmtEur(income, { hide: settings.hide })} />
  <KPI label={t().common.expenses} value={fmtEur(expenses, { hide: settings.hide })} />
  <KPI
    label={t().common.savingsRate}
    value={displaySavingsIncome > 0 ? `${displaySavingsRate.toFixed(0)}%` : '—'}
    title={savingsExclInv ? 'Investitionen nicht eingerechnet' : 'Investitionen eingerechnet (buy = Ausgabe, sell = Einnahme)'}
  >
    {#snippet topRight()}
      <div class="seg sav-toggle" role="tablist" aria-label="Investitionen ein-/ausrechnen">
        <button
          type="button"
          role="tab"
          aria-selected={!savingsExclInv}
          class:active={!savingsExclInv}
          onclick={() => { if (savingsExclInv) toggleSavingsExclInv(); }}
          title="Investitionen einrechnen"
        >+Inv</button>
        <button
          type="button"
          role="tab"
          aria-selected={savingsExclInv}
          class:active={savingsExclInv}
          onclick={() => { if (!savingsExclInv) toggleSavingsExclInv(); }}
          title="Investitionen ausrechnen"
        >−Inv</button>
      </div>
    {/snippet}
  </KPI>
  <KPI
    label="Investitionsquote"
    value={income > 0 ? `${investmentRate.toFixed(0)}%` : '—'}
  />
</div>

<div class="card card-pad-lg" style="margin-bottom: 16px;">
  <div class="card-h">
    <h3>Sparquote · 12 Monate</h3>
    <div class="head-right">
      <div class="seg" role="tablist" aria-label="Diagrammtyp">
        <button
          type="button"
          role="tab"
          aria-selected={savingsMode === 'line'}
          class:active={savingsMode === 'line'}
          onclick={() => setSavingsMode('line')}
          title="Linie + Fläche"
        >Linie</button>
        <button
          type="button"
          role="tab"
          aria-selected={savingsMode === 'bars'}
          class:active={savingsMode === 'bars'}
          onclick={() => setSavingsMode('bars')}
          title="Balken"
        >Balken</button>
        <button
          type="button"
          role="tab"
          aria-selected={savingsMode === 'inout'}
          class:active={savingsMode === 'inout'}
          onclick={() => setSavingsMode('inout')}
          title="Einnahmen / Ausgaben"
        >Ein/Aus</button>
      </div>
      <span class="muted">∅ {avgSavingsRate12m.toFixed(0)}%</span>
    </div>
  </div>
  <SavingsRateChart flows={monthlyFlows} mode={savingsMode} hide={settings.hide}
    onPointClick={(year, month) => {
      const m = String(month).padStart(2, '0');
      const from = `${year}-${m}-01`;
      const lastDay = new Date(year, month, 0).getDate();
      const to = `${year}-${m}-${String(lastDay).padStart(2, '0')}`;
      goto(`/transactions?from=${from}&to=${to}`);
    }}
  />
</div>

{#if institutionSummaries.length > 0}
<div class="card inst-section" style="margin-bottom: 16px;">
  <div class="card-h">
    <h3>{t().nav.institutions}</h3>
    <a class="btn icon" href="/institute" aria-label={t().nav.institutions}>
      <Icon name="chevron-right" size={14} />
    </a>
  </div>
  <ul class="inst-rows">
    {#each institutionSummaries as inst (inst.id)}
      <li>
        <a href={`/institute/${inst.id}`} class="inst-row">
          <span class="inst-dot" style:background={inst.color ?? 'var(--accent)'}></span>
          <span class="inst-name">{inst.name}</span>
          <span class="inst-meta muted">{t().institutions.accountCount(inst.accountCount)}</span>
          <span class="inst-balance num">{fmtEur(inst.balanceCents, { hide: settings.hide })}</span>
        </a>
      </li>
    {/each}
  </ul>
</div>
{/if}

<div class="grid-12">
  <div class="card col-7">
    <div class="card-h">
      <h3>{t().common.recent}</h3>
      <a class="btn" href="/transactions">
        {t().common.seeAll}
        <Icon name="chevron-right" size={13} />
      </a>
    </div>
    {#if loading}
      <div class="skel-list">
        {#each Array(5) as _, i (i)}
          <Skeleton height="38" marginTop={i === 0 ? 0 : 6} />
        {/each}
      </div>
    {:else if recent.length === 0}
      <div class="empty">{t().common.importStatements}</div>
    {:else}
      {#each groupByDay(recent) as group (group.date)}
        <div class="tx-list-day-header">
          <span>{new Intl.DateTimeFormat(settings.lang === 'de' ? 'de-DE' : 'en-US', { day: '2-digit', month: 'long' }).format(new Date(group.date + 'T12:00:00'))}</span>
          <span class="day-sum num" class:neg={group.totalCents < 0} class:pos={group.totalCents > 0}>
            {fmtEur(group.totalCents, { hide: settings.hide, signed: true })}
          </span>
        </div>
        {#each group.txs as tx (tx.id)}
          <TxRow
            {tx}
            {accounts}
            {categories}
            {bucketsById}
            lang={settings.lang}
            hide={settings.hide}
            onclick={() => openEdit(tx)}
          />
        {/each}
      {/each}
    {/if}
  </div>

  <div class="col-5 stack">
    <div class="card">
      <div class="card-h">
        <h3>{t().common.topCats}</h3>
        <a class="btn icon" href="/budgets" aria-label={t().nav.budgets}>
          <Icon name="chevron-right" size={14} />
        </a>
      </div>
      {#if topCats.length === 0}
        <div class="empty small">—</div>
      {:else}
        {#each topCats as item (item.category!.id)}
          <div class="cat-row">
            <span class="cat-icon" style:color={item.category!.color || 'var(--text-muted)'}>
              <Icon name={item.category!.icon || 'tag'} size={13} />
            </span>
            <span class="cat-name">{item.category!.name}</span>
            <span class="num cat-amt">{fmtEur(item.cents, { hide: settings.hide })}</span>
          </div>
        {/each}
      {/if}
    </div>

  </div>
</div>

{#if modalOpen && modalTx}
  {#if isTradeTx(modalTx)}
    <DepotTxModal
      tx={modalTx}
      {accounts}
      {categories}
      {bucketsById}
      onClose={closeModal}
      onSaved={onSaved}
      onDeleted={onDeleted}
    />
  {:else}
    <TxModal
      tx={modalTx}
      {accounts}
      {categories}
      onClose={closeModal}
      onSaved={onSaved}
      onDeleted={onDeleted}
      onCategoryCreated={(c) => (categories = [...categories, c])}
    />
  {/if}
{/if}

{#if showImportModal}
  <ImportStatementsModal
    {accounts}
    institutions={institutionSummaries}
    onClose={() => { showImportModal = false; }}
    onImported={() => { void loadStatic(); void loadRangeTx(); }}
  />
{/if}

<style>
  .sav-toggle {
    padding: 1px;
  }
  .sav-toggle button {
    font-size: 9.5px;
    padding: 2px 6px;
    line-height: 1.1;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .skel-list { display: grid; gap: 6px; padding: 8px 0; }
  .muted { color: var(--text-muted); font-size: 12px; font-variant-numeric: tabular-nums; }
  .head-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .seg {
    display: inline-flex;
    background: var(--surface-2);
    border-radius: 8px;
    padding: 2px;
    gap: 2px;
  }
  .seg button {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .seg button:hover {
    color: var(--text);
  }
  .seg button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
  }
  .range-bar {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
    flex-wrap: wrap;
  }
  .range-label {
    color: var(--text-muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .custom-dates {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .custom-dates input[type="date"] {
    appearance: none;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 12px;
    color: var(--text);
    font-family: inherit;
    font-variant-numeric: tabular-nums;
  }
  .custom-dates input[type="date"]:focus {
    outline: none;
    border-color: var(--accent);
  }
  .custom-dates .dash {
    color: var(--text-muted);
    font-size: 12px;
  }
  .empty {
    padding: 28px 0;
    text-align: center;
    color: var(--text-faint);
  }
  .empty.small {
    padding: 12px 0;
  }
  .cat-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
  }
  .cat-icon {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    background: var(--surface-2);
    display: grid;
    place-items: center;
  }
  .cat-name {
    flex: 1;
    font-size: 13px;
  }
  .cat-amt {
    font-size: 13px;
    font-weight: 500;
  }
  /* institutions section */
  .inst-rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .inst-rows li + li {
    border-top: 1px solid var(--border);
  }
  .inst-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    text-decoration: none;
    color: inherit;
  }
  .inst-row:hover .inst-name {
    color: var(--accent);
  }
  .inst-dot {
    flex-shrink: 0;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .inst-name {
    flex: 1;
    font-size: 13px;
    font-weight: 500;
    transition: color 0.1s;
  }
  .inst-meta {
    font-size: 12px;
  }
  .inst-balance {
    font-size: 13px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
</style>
