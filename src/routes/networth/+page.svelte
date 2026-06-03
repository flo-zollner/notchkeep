<script lang="ts">
  import Donut from '$lib/components/Donut.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import KPI from '$lib/components/KPI.svelte';
  import NetWorthChart from '$lib/components/NetWorthChart.svelte';
  import NetWorthIndexChart from '$lib/components/NetWorthIndexChart.svelte';
  import { goto } from '$app/navigation';
  import { listen } from '@tauri-apps/api/event';
  import {
    api,
    listInstitutions,
    type Account,
    type AccountMarketValue,
    type Holding,
    type Institution,
    type MonthlyFlow,
    type NetWorthForecastPoint,
    type NetWorthPoint, errMsg} from '$lib/api';
  import { settings, t } from '$lib/settings.svelte';
  import { bp } from '$lib/breakpoints';

  const MONTH_LABELS_DE = [
    'Jan', 'Feb', 'Mär', 'Apr', 'Mai', 'Jun',
    'Jul', 'Aug', 'Sep', 'Okt', 'Nov', 'Dez',
  ];
  const MONTH_LABELS_EN = [
    'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
    'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
  ];

  function monthLabel(year: number, month: number): string {
    const labels = settings.lang === 'en' ? MONTH_LABELS_EN : MONTH_LABELS_DE;
    return `${labels[month - 1]} ${String(year).slice(2)}`;
  }

  function fmtEur(v: number, opts: { hide?: boolean; decimals?: number } = {}): string {
    if (opts.hide) return '•••• €';
    const d = opts.decimals ?? 0;
    return (
      v.toLocaleString('de-DE', { minimumFractionDigits: d, maximumFractionDigits: d }) + ' €'
    );
  }

  const now = new Date();
  const curYear = now.getFullYear();
  const curMonth = now.getMonth() + 1;

  type Range = '1J' | '3J' | '5J' | 'all';
  let range = $state<Range>('1J');

  function monthsForRange(r: Range): number {
    return r === '1J' ? 12 : r === '3J' ? 36 : r === '5J' ? 60 : 0;
  }

  type RunwayWindow = 3 | 6 | 12;
  let runwayWindow = $state<RunwayWindow>(6);

  const PALETTE = ['var(--c1)', 'var(--c2)', 'var(--c3)', 'var(--c4)', 'var(--c5)', 'var(--c6)'];
  const FALLBACK_LOGO_BG = 'var(--fallback-logo-bg)';

  let accounts = $state<Account[]>([]);
  let balances = $state<Record<number, number>>({});
  let cashflow = $state<MonthlyFlow[]>([]);
  let history = $state<NetWorthPoint[]>([]);
  let forecast = $state<NetWorthForecastPoint[]>([]);
  let holdings = $state<Holding[]>([]);
  let portfolioCents = $state(0);
  let portfolioByAccount = $state<Record<number, number>>({});
  let institutions = $state<Institution[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Index chart: own range + full history loaded once
  let indexRange = $state<Range>('1J');
  let indexFullHistory = $state<NetWorthPoint[]>([]);
  let indexBaselineIdx = $state<number | null>(null);

  type DonutMode = 'lumped' | 'perHolding' | 'perAccount' | 'perInstitution' | 'perKind';
  let donutMode = $state<DonutMode>('lumped');

  // Account kind order as on /accounts
  const KIND_ORDER = ['bank', 'savings', 'broker', 'cash', 'credit', 'loan'] as const;

  function kindLabel(kind: string): string {
    const tx = t().common as unknown as Record<string, string | undefined>;
    const key = `kind${kind.charAt(0).toUpperCase() + kind.slice(1)}`;
    return tx[key] ?? kind;
  }

  const institutionsById = $derived(new Map(institutions.map((i) => [i.id, i])));

  async function loadAll(months: number, window: number) {
    loading = true;
    error = null;
    try {
      const [accs, cf, hist, fc, pk, hs, pba, insts] = await Promise.all([
        api.listAccounts(),
        api.monthlyCashflow(curYear, curMonth, window),
        api.netWorthHistory(curYear, curMonth, months),
        api.netWorthForecast(curYear, curMonth, 6, 6),
        api.portfolioKpis(curYear),
        api.listHoldings(),
        api.portfolioValueByAccount(),
        listInstitutions(true),
      ]);
      accounts = accs;
      cashflow = cf;
      history = hist;
      forecast = fc;
      portfolioCents = pk.marketValueCents;
      holdings = hs;
      portfolioByAccount = Object.fromEntries(
        (pba as AccountMarketValue[]).map((r) => [r.accountId, r.marketValueCents]),
      );
      institutions = insts;
      // Load full history for the index chart (months=0 → all)
      const indexHist = await api.netWorthHistory(curYear, curMonth, 0);
      indexFullHistory = indexHist;
      const active = accs.filter((a) => !a.archived);
      const balPairs = await Promise.all(
        active.map((a) => api.accountBalance(a.id).then((b) => [a.id, b] as const)),
      );
      balances = Object.fromEntries(balPairs);
    } catch (e) {
      error = errMsg(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadAll(monthsForRange(range), runwayWindow);
  });

  // After a background FX/price refresh: reload everything (holdings, portfolio values,
  // account balances are all FX-dependent).
  $effect(() => {
    type Status = { stage: 'started' | 'completed' | 'failed' };
    const unlisten = listen<Status>('price_refresh_status', (e) => {
      if (e.payload.stage === 'completed') {
        void loadAll(monthsForRange(range), runwayWindow);
      }
    });
    return () => { unlisten.then((u) => u()); };
  });

  const activeAccounts = $derived(accounts.filter((a) => !a.archived));

  // Account total including securities market value per account (consistent with perKind/perAccount).
  function accountTotalCents(a: Account): number {
    return (balances[a.id] ?? 0) + (portfolioByAccount[a.id] ?? 0);
  }

  // Group active accounts by kind, sorted in KIND_ORDER order.
  const accountsByKind = $derived.by(() => {
    const map = new Map<string, Account[]>();
    for (const a of activeAccounts) {
      const arr = map.get(a.kind) ?? [];
      arr.push(a);
      map.set(a.kind, arr);
    }
    const known = KIND_ORDER.filter((k) => map.has(k));
    const unknown = [...map.keys()]
      .filter((k) => !(KIND_ORDER as readonly string[]).includes(k))
      .sort();
    return [...known, ...unknown].map((kind) => {
      const items = map.get(kind) ?? [];
      const total = items.reduce((s, a) => s + accountTotalCents(a), 0);
      return { kind, label: kindLabel(kind), items, total };
    });
  });

  const cashOnlyCents = $derived(activeAccounts.reduce((s, a) => s + (balances[a.id] ?? 0), 0));
  const netWorthCents = $derived(cashOnlyCents + portfolioCents);
  const assetsCents = $derived(
    activeAccounts.reduce((s, a) => s + Math.max(0, balances[a.id] ?? 0), 0) + portfolioCents,
  );
  const liabilitiesCents = $derived(
    Math.abs(activeAccounts.reduce((s, a) => s + Math.min(0, balances[a.id] ?? 0), 0)),
  );
  const liquidCents = $derived(
    activeAccounts
      .filter((a) => a.kind === 'bank' || a.kind === 'savings')
      .reduce((s, a) => s + (balances[a.id] ?? 0), 0),
  );
  const avgExpensesCents = $derived(
    cashflow.length > 0
      ? Math.round(cashflow.reduce((s, m) => s + m.outCents, 0) / cashflow.length)
      : 0,
  );
  const runwayMonths = $derived(
    liquidCents > 0 && avgExpensesCents > 0 ? liquidCents / avgExpensesCents : null,
  );

  const deltaCents = $derived(
    history.length >= 2 ? history[history.length - 1].totalCents - history[history.length - 2].totalCents : 0,
  );
  const deltaPct = $derived.by(() => {
    if (history.length < 2) return 0;
    const prev = history[history.length - 2].totalCents;
    if (prev === 0) return 0;
    return (deltaCents / prev) * 100;
  });

  const chartHistory = $derived(
    history.map((p) => ({ m: monthLabel(p.year, p.month), v: p.totalCents / 100, year: p.year, month: p.month })),
  );
  const chartForecast = $derived(
    forecast.map((p) => ({
      m: monthLabel(p.year, p.month),
      mid: p.midCents / 100,
      lo: p.loCents / 100,
      hi: p.hiCents / 100,
    })),
  );

  const indexChartHistory = $derived.by(() => {
    const months = monthsForRange(indexRange);
    const slice = months === 0 ? indexFullHistory : indexFullHistory.slice(-months);
    return slice.map((p) => ({ m: monthLabel(p.year, p.month), v: p.totalCents / 100 }));
  });

  const indexBaselineLabel = $derived.by(() => {
    if (indexBaselineIdx === null) return null;
    const pt = indexChartHistory[indexBaselineIdx];
    return pt ? `Baseline: ${pt.m} = 100%` : null;
  });

  const allocation = $derived.by(() => {
    if (donutMode === 'perAccount') {
      // Account balance + portfolio market value per account combined.
      return activeAccounts
        .map((a, i) => ({
          name: a.name,
          v: ((balances[a.id] ?? 0) + (portfolioByAccount[a.id] ?? 0)) / 100,
          color: a.color ?? PALETTE[i % PALETTE.length],
        }))
        .filter((d) => d.v > 0);
    }
    if (donutMode === 'perInstitution') {
      // Accounts + portfolio grouped per institution.
      const byInst = new Map<number | null, number>(); // institution_id (or null) -> total cents
      for (const a of activeAccounts) {
        const total = (balances[a.id] ?? 0) + (portfolioByAccount[a.id] ?? 0);
        const key = a.institution_id ?? null;
        byInst.set(key, (byInst.get(key) ?? 0) + total);
      }
      const items: { name: string; v: number; color: string }[] = [];
      // Sorted according to the institution list (alphabetically), plus a special "No institution" group at the end
      for (const [j, inst] of institutions.entries()) {
        const cents = byInst.get(inst.id);
        if (cents != null && cents !== 0) {
          items.push({ name: inst.name, v: cents / 100, color: inst.color ?? PALETTE[j % PALETTE.length] });
        }
      }
      const noInstCents = byInst.get(null);
      if (noInstCents != null && noInstCents !== 0) {
        items.push({ name: t().accounts.withoutInstitution, v: noInstCents / 100, color: PALETTE[items.length % PALETTE.length] });
      }
      return items;
    }
    if (donutMode === 'perKind') {
      // Accounts + portfolio grouped by account kind (broker, bank, savings, …).
      const byKind = new Map<string, number>();
      for (const a of activeAccounts) {
        const total = (balances[a.id] ?? 0) + (portfolioByAccount[a.id] ?? 0);
        byKind.set(a.kind, (byKind.get(a.kind) ?? 0) + total);
      }
      const known = KIND_ORDER.filter((k) => byKind.has(k));
      const unknown = [...byKind.keys()].filter((k) => !(KIND_ORDER as readonly string[]).includes(k)).sort();
      const items: { name: string; v: number; color: string }[] = [];
      for (const [j, kind] of [...known, ...unknown].entries()) {
        const cents = byKind.get(kind);
        if (cents != null && cents !== 0) {
          items.push({ name: kindLabel(kind), v: cents / 100, color: PALETTE[j % PALETTE.length] });
        }
      }
      return items;
    }
    const accountSlices = activeAccounts
      .map((a, i) => ({
        name: a.name,
        v: (balances[a.id] ?? 0) / 100,
        color: a.color ?? PALETTE[i % PALETTE.length],
      }))
      .filter((d) => d.v > 0);
    if (donutMode === 'perHolding') {
      const offset = accountSlices.length;
      const holdingSlices = holdings
        .map((h, j) => ({
          name: h.name,
          v: h.marketValueCents / 100,
          color: PALETTE[(offset + j) % PALETTE.length],
        }))
        .filter((d) => d.v > 0);
      return [...accountSlices, ...holdingSlices];
    }
    // 'lumped': one combined "securities" slice
    if (portfolioCents > 0) {
      accountSlices.push({
        name: t().common.securities,
        v: portfolioCents / 100,
        color: PALETTE[accountSlices.length % PALETTE.length],
      });
    }
    return accountSlices;
  });
</script>

<div class="topbar">
  <div>
    <h1>{t().nav.networth}</h1>
    <div class="sub">{t().common.assets} − {t().common.liabilities}</div>
  </div>
  <button class="btn"><Icon name="refresh" size={14} /> {t().common.synced}</button>
</div>

<div class="sr-error" aria-live="polite" aria-atomic="true">{error ? `Fehler: ${error}` : ''}</div>

<div class="kpi-grid">
  <KPI
    label={t().common.netWorth}
    value={fmtEur(netWorthCents / 100, { hide: settings.hide })}
    delta={deltaCents / 100}
    pct={deltaPct}
  />
  <KPI
    label={t().common.assets}
    value={fmtEur(assetsCents / 100, { hide: settings.hide })}
  />
  <KPI
    label={t().common.liabilities}
    value={fmtEur(liabilitiesCents / 100, { hide: settings.hide })}
    sub={t().common.liabilitiesSub}
    title={t().common.liabilitiesTooltip}
    inverted
  />
  <KPI
    label={t().common.runway}
    value={runwayMonths === null
      ? '—'
      : `${runwayMonths.toFixed(1)} ${t().common.months}`}
    sub={t().common.runwaySub(runwayWindow)}
  >
    {#snippet pill()}
      <div class="seg seg-sm" role="group" aria-label={t().common.runwayWindowLabel}>
        {#each [3, 6, 12] as w (w)}
          <button
            class:on={runwayWindow === w}
            onclick={() => (runwayWindow = w as RunwayWindow)}
            aria-pressed={runwayWindow === w}
          >
            {w}
          </button>
        {/each}
      </div>
    {/snippet}
  </KPI>
</div>

<div class="grid-12">
  <div class="card col-12 card-pad-lg">
    <div class="card-h">
      <h3>{t().common.forecast}</h3>
      <div class="seg">
        {#each ['1J', '3J', '5J', 'all'] as r (r)}
          <button class:on={range === r} onclick={() => (range = r as Range)}>
            {r === 'all' ? t().common.all : r}
          </button>
        {/each}
      </div>
    </div>
    {#if loading && history.length === 0}
      <div class="empty">…</div>
    {:else if history.length === 0}
      <EmptyState
        icon="networth"
        title="Noch keine Vermögensdaten"
        description="Lege Konten an und erfasse Buchungen — dann zeigt sich hier deine Vermögensentwicklung."
      />
    {:else}
      <figure class="chart-figure">
        <figcaption class="sr-only">{t().common.forecast} — {range === 'all' ? t().common.all : range}</figcaption>
        <NetWorthChart history={chartHistory} forecast={chartForecast} height={$bp === 'phone' ? 200 : 260} hide={settings.hide}
          onPointClick={(year, month) => {
            const m = String(month).padStart(2, '0');
            const from = `${year}-${m}-01`;
            const lastDay = new Date(year, month, 0).getDate();
            const to = `${year}-${m}-${String(lastDay).padStart(2, '0')}`;
            goto(`/transactions?from=${from}&to=${to}`);
          }}
        />
      </figure>
    {/if}
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h">
      <h3>{t().common.breakdown}</h3>
      <div class="seg seg-sm" role="group" aria-label={t().common.breakdownMode}>
        {#each [
          { k: 'lumped' as DonutMode, label: t().common.breakdownModeLumped },
          { k: 'perHolding' as DonutMode, label: t().common.breakdownModePerHolding },
          { k: 'perAccount' as DonutMode, label: t().common.breakdownModePerAccount },
          { k: 'perInstitution' as DonutMode, label: t().common.breakdownModePerInstitution },
          { k: 'perKind' as DonutMode, label: t().common.breakdownModePerKind },
        ] as opt (opt.k)}
          <button
            class:on={donutMode === opt.k}
            onclick={() => (donutMode = opt.k)}
            aria-pressed={donutMode === opt.k}
          >
            {opt.label}
          </button>
        {/each}
      </div>
    </div>
    {#if allocation.length === 0}
      <EmptyState icon="networth" title="Keine Aufteilung" description="Sobald Konten mit Beständen existieren, erscheint hier die Aufteilung." />
    {:else}
      <figure class="chart-figure">
        <figcaption class="sr-only">{t().common.breakdown}</figcaption>
        <Donut data={allocation} hide={settings.hide} />
      </figure>
    {/if}
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h">
      <div>
        <h3>{t().common.relativeChange ?? 'Relative Entwicklung (indexiert)'}</h3>
        <span class="sub" style="font-size: 11px; color: var(--text-faint); display: block;">
          {indexBaselineLabel ?? (t().common.relativeChangeHint ?? 'Erster Datenpunkt = 100%')}
        </span>
      </div>
      <div class="seg">
        {#each ['1J', '3J', '5J', 'all'] as r (r)}
          <button class:on={indexRange === r} onclick={() => { indexRange = r as Range; indexBaselineIdx = null; }}>
            {r === 'all' ? t().common.all : r}
          </button>
        {/each}
      </div>
    </div>
    {#if loading && indexFullHistory.length === 0}
      <div class="empty">…</div>
    {:else if indexChartHistory.length === 0}
      <div class="empty">—</div>
    {:else}
      <figure class="chart-figure">
        <figcaption class="sr-only">{t().common.relativeChange ?? 'Relative Entwicklung (indexiert)'} — {indexRange === 'all' ? t().common.all : indexRange}</figcaption>
        <NetWorthIndexChart
          history={indexChartHistory}
          height={$bp === 'phone' ? 180 : 220}
          hide={settings.hide}
          baselineIdx={indexBaselineIdx}
          onBaselineChange={(i) => indexBaselineIdx = i}
        />
      </figure>
    {/if}
  </div>

  <div class="card col-12 card-pad-lg">
    <div class="card-h">
      <h3>{t().nav.accounts}</h3>
      <a class="btn" href="/accounts"><Icon name="plus" size={13} /> {t().common.addAccount}</a>
    </div>
    {#if activeAccounts.length === 0}
      <EmptyState icon="accounts" title="Noch keine Konten" description="Lege Konten an, um deine Vermögensaufteilung zu sehen." />
    {:else}
      {#each accountsByKind as group (group.kind)}
        <div class="kind-section">
          <div class="kind-h">
            <h4>{group.label}</h4>
            <span class="kind-total num">
              {fmtEur(group.total / 100, { hide: settings.hide, decimals: 0 })}
            </span>
          </div>
          <div class="acc-grid">
            {#each group.items as a, i (a.id)}
              {@const bg = a.color ?? PALETTE[i % PALETTE.length] ?? FALLBACK_LOGO_BG}
              {@const inst = a.institution_id != null ? institutionsById.get(a.institution_id) : null}
              <a class="acc" href={`/accounts/${a.id}`} aria-label={a.name}>
                <div class="acc-logo" style:background={bg} style:color="var(--on-logo-bg)">
                  {#if a.icon}
                    <Icon name={a.icon} size={14} />
                  {:else}
                    {a.name.slice(0, 2).toUpperCase()}
                  {/if}
                </div>
                <div class="acc-body">
                  <div class="acc-name">{a.name}</div>
                  <div class="acc-sub">
                    {#if inst}
                      <span class="bank-badge" style:--bank-color={inst.color ?? 'var(--text-faint)'}>{inst.name}</span>
                    {:else}
                      <span class="bank-badge no-inst">{t().accounts.withoutInstitution}</span>
                    {/if}
                    <span class="dot">·</span>{a.currency}
                  </div>
                </div>
                <div class="acc-right">
                  <div class="num bal">
                    {fmtEur(accountTotalCents(a) / 100, { hide: settings.hide, decimals: 2 })}
                  </div>
                </div>
              </a>
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .acc-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 12px;
  }
  @media (min-width: 600px) and (max-width: 1023px) {
    .acc-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @media (max-width: 599px) {
    .acc-grid {
      grid-template-columns: minmax(0, 1fr);
      gap: 8px;
    }
  }
  @media (max-width: 599px) {
    .topbar {
      flex-wrap: wrap;
      gap: 8px;
    }
    .topbar h1 {
      font-size: 20px;
    }
    .topbar .sub {
      font-size: 12px;
    }
  }
  @media (max-width: 599px) {
    .card-h {
      flex-wrap: wrap;
      gap: 8px;
    }
    .card-h :global(.seg) {
      flex-wrap: wrap;
    }
    .card-h :global(.seg button) {
      font-size: 10.5px;
      padding: 4px 8px;
    }
  }
  .acc {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    border-radius: var(--r-md);
    background: var(--surface-2);
    color: inherit;
    text-decoration: none;
  }
  .acc:hover {
    background: var(--surface-3, var(--surface-2));
  }
  .acc-logo {
    width: 32px;
    height: 32px;
    border-radius: var(--r-md);
    display: grid;
    place-items: center;
    font-weight: 600;
    flex-shrink: 0;
    font-size: 12px;
  }
  .acc-body {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }
  .acc-name {
    font-weight: 500;
    font-size: 13.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acc-sub {
    font-size: 11.5px;
    color: var(--text-faint);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .acc-sub .bank-badge {
    font-size: 10px;
    padding: 1px 8px;
    border-radius: var(--r-sm);
    border: 1px solid var(--bank-color, var(--text-faint));
    color: var(--bank-color, var(--text-faint));
    opacity: 0.9;
    white-space: nowrap;
  }
  .acc-sub .bank-badge.no-inst {
    opacity: 0.6;
    font-style: italic;
  }
  .acc-sub .dot {
    color: var(--text-faint);
    opacity: 0.6;
  }
  .kind-section {
    margin-bottom: 16px;
  }
  .kind-section:last-child {
    margin-bottom: 0;
  }
  .kind-h {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin: 0 4px 8px;
  }
  .kind-h h4 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  .kind-total {
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 600;
  }
  .acc-right {
    text-align: right;
  }
  .bal {
    font-weight: 600;
  }
  .empty {
    padding: 24px;
    text-align: center;
    color: var(--text-faint);
    font-size: 13px;
  }
  .chart-figure {
    margin: 0;
    padding: 0;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  .sr-error {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  .sr-error:not(:empty) {
    position: static;
    width: auto;
    height: auto;
    padding: 0;
    margin: 0;
    overflow: visible;
    clip: auto;
    white-space: normal;
  }
</style>
