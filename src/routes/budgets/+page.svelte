<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import SpendingHeatmap from '$lib/components/SpendingHeatmap.svelte';
  import BudgetYearOverview from '$lib/components/BudgetYearOverview.svelte';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';
  import {
    api,
    type Category,
    type CategoryMonthBudget,
    type InvestmentFlow,
  } from '$lib/api';
  import { fmtEur, parseEurCents } from '$lib/format';

  const MONTH_LABELS_DE = [
    'Januar', 'Februar', 'März', 'April', 'Mai', 'Juni',
    'Juli', 'August', 'September', 'Oktober', 'November', 'Dezember',
  ];
  const MONTH_LABELS_EN = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December',
  ];

  const now = new Date();
  const currentYear = now.getFullYear();
  const currentMonth = now.getMonth() + 1;

  let viewYear = $state(currentYear);
  let viewMonth = $state(currentMonth);

  const SONSTIGE_KEY_PREFIX = 'budgets.sonstige';
  function sonstigeKey(y: number, m: number): string {
    return `${SONSTIGE_KEY_PREFIX}.${y}-${m}`;
  }
  function loadSonstigeBudget(y: number, m: number): number {
    if (typeof localStorage === 'undefined') return 0;
    const raw = localStorage.getItem(sonstigeKey(y, m));
    if (!raw) return 0;
    const n = parseInt(raw, 10);
    return Number.isFinite(n) && n >= 0 ? n : 0;
  }
  function saveSonstigeBudget(y: number, m: number, cents: number): void {
    if (typeof localStorage === 'undefined') return;
    if (cents > 0) localStorage.setItem(sonstigeKey(y, m), String(cents));
    else localStorage.removeItem(sonstigeKey(y, m));
  }

  let sonstigeBudgetCents = $state(0);

  $effect(() => {
    sonstigeBudgetCents = loadSonstigeBudget(viewYear, viewMonth);
  });

  function updateSonstigeBudget(cents: number) {
    sonstigeBudgetCents = Math.max(0, Math.round(cents));
    saveSonstigeBudget(viewYear, viewMonth, sonstigeBudgetCents);
  }

  type Tab = 'month' | 'year';
  let activeTab = $state<Tab>('month');

  let monthRows = $state<CategoryMonthBudget[]>([]);
  let dailyCents = $state<number[]>([]);
  let loading = $state(true);
  let categoriesById = $state<Map<number, Category>>(new Map());
  let uncategorizedSpentCents = $state(0);
  let monthIncomeCents = $state(0);
  let investmentFlow = $state<InvestmentFlow>({
    buysCents: 0,
    sellsCents: 0,
    dividendsCents: 0,
    netInvestedCents: 0,
  });

  const saveTimers = new Map<number, ReturnType<typeof setTimeout>>();
  const SAVE_DEBOUNCE_MS = 600;

  // Slider max is initialized *once* per (category, year, month) and only raised
  // when the value exceeds the current upper bound.
  // The composite key decouples max from the live budget and prevents the thumb
  // from reactively sticking at 50 % when switching between months.
  let sliderMaxEur = $state<Map<string, number>>(new Map());
  const SLIDER_MIN_MAX_EUR = 300;
  const SLIDER_INIT_HEADROOM = 2;
  const SLIDER_BUMP_HEADROOM = 1.2;

  function sliderKey(catId: number, y: number, m: number): string {
    return `${catId}-${y}-${m}`;
  }

  function ensureSliderMax(catId: number, y: number, m: number, currentEur: number): void {
    const key = sliderKey(catId, y, m);
    const existing = sliderMaxEur.get(key);
    if (existing === undefined) {
      sliderMaxEur.set(
        key,
        Math.max(currentEur * SLIDER_INIT_HEADROOM, SLIDER_MIN_MAX_EUR),
      );
    } else if (currentEur > existing) {
      sliderMaxEur.set(key, currentEur * SLIDER_BUMP_HEADROOM);
    }
  }

  $effect(() => {
    const y = viewYear;
    const m = viewMonth;
    loading = true;
    Promise.all([
      api.monthOverview(y, m),
      api.dailySpending(y, m),
      api.listCategories(),
      api.uncategorizedMonthlySpent(y, m),
      api.monthlyCashflow(y, m, 1),
      api.investmentFlowForMonth(y, m),
    ])
      .then(([overview, daily, cats, uncat, cashflow, invFlow]) => {
        monthRows = overview;
        dailyCents = daily;
        categoriesById = new Map(cats.map((c: Category) => [c.id, c]));
        uncategorizedSpentCents = uncat;
        monthIncomeCents = cashflow[0]?.inCents ?? 0;
        investmentFlow = invFlow;
        for (const row of overview) {
          const eur = (row.budgetCents ?? 0) / 100;
          ensureSliderMax(row.categoryId, y, m, eur);
        }
      })
      .finally(() => {
        loading = false;
      });
  });

  // Heatmap expects float euros and a weekday offset (0=Mon) for the 1st of the month.
  const dailyEur = $derived(dailyCents.map((c) => c / 100));
  const heatmapOffset = $derived(((new Date(viewYear, viewMonth - 1, 1).getDay() + 6) % 7));

  // Card stays visible once a budget has been set (even 0 €) OR
  // once there is spending. Removal only happens explicitly via the remove X.
  const rows = $derived(
    monthRows
      .filter((r) => r.budgetCents !== null || r.spentCents > 0)
      .sort((a, b) => b.spentCents - a.spentCents),
  );

  // Grouping: budgeted (budget set AND > 0) vs unbudgeted (no/0 budget but spending > 0)
  const budgetedRows = $derived(
    monthRows
      .filter((r) => r.budgetCents !== null && r.budgetCents > 0)
      .sort((a, b) => b.spentCents - a.spentCents),
  );
  const unbudgetedRows = $derived(
    monthRows
      .filter((r) => (r.budgetCents === null || r.budgetCents === 0) && r.spentCents > 0)
      .sort((a, b) => b.spentCents - a.spentCents),
  );

  // Combined "unplanned": spending in unbudgeted categories + uncategorized transactions
  const nonBudgetedSpentCents = $derived(
    unbudgetedRows.reduce((s, r) => s + r.spentCents, 0) + uncategorizedSpentCents,
  );

  // Sonstige-Card derived helpers
  const sonstigeP = $derived(
    sonstigeBudgetCents > 0 ? (nonBudgetedSpentCents / sonstigeBudgetCents) * 100 : 0,
  );
  const sonstigeCls = $derived(sonstigeP > 100 ? 'over' : sonstigeP > 85 ? 'warn' : '');
  const sonstigeBudgetEur = $derived(sonstigeBudgetCents / 100);

  // KPI totals also include the "other" catch-all (budget + spending):
  // - Budget = sum of category budgets + other budget
  // - Spent = sum of category spending (rows contains BUDGETED + UNBUDGETED) + uncategorized
  //   (otherwise unbudgeted categories would be double-counted).
  const totalBudgetCents = $derived(
    rows.reduce((s, r) => s + (r.budgetCents ?? 0), 0) + sonstigeBudgetCents
  );
  const totalSpentCents = $derived(
    rows.reduce((s, r) => s + r.spentCents, 0) + uncategorizedSpentCents
  );
  const pct = $derived(totalBudgetCents > 0 ? (totalSpentCents / totalBudgetCents) * 100 : 0);
  const overall = $derived(pct > 100 ? 'over' : pct > 85 ? 'warn' : '');

  const monthLabel = $derived.by(() => {
    const labels = settings.lang === 'en' ? MONTH_LABELS_EN : MONTH_LABELS_DE;
    return `${labels[viewMonth - 1]} ${viewYear}`;
  });

  const isCurrentMonth = $derived(
    viewYear === currentYear && viewMonth === currentMonth,
  );

  function stepMonth(delta: -1 | 1) {
    let y = viewYear;
    let m = viewMonth + delta;
    if (m < 1) {
      m = 12;
      y -= 1;
    } else if (m > 12) {
      m = 1;
      y += 1;
    }
    viewYear = y;
    viewMonth = m;
  }

  function setThisMonth() {
    viewYear = currentYear;
    viewMonth = currentMonth;
  }

  function prevMonthLabel(y: number, m: number): string {
    const py = m === 1 ? y - 1 : y;
    const pm = m === 1 ? 12 : m - 1;
    const labels = settings.lang === 'en' ? MONTH_LABELS_EN : MONTH_LABELS_DE;
    return `${labels[pm - 1]} ${py}`;
  }

  async function reload() {
    const [overview, daily, cats, uncat, cashflow, invFlow] = await Promise.all([
      api.monthOverview(viewYear, viewMonth),
      api.dailySpending(viewYear, viewMonth),
      api.listCategories(),
      api.uncategorizedMonthlySpent(viewYear, viewMonth),
      api.monthlyCashflow(viewYear, viewMonth, 1),
      api.investmentFlowForMonth(viewYear, viewMonth),
    ]);
    monthRows = overview;
    dailyCents = daily;
    categoriesById = new Map(cats.map((c: Category) => [c.id, c]));
    uncategorizedSpentCents = uncat;
    monthIncomeCents = cashflow[0]?.inCents ?? 0;
    investmentFlow = invFlow;
    void loadAvgSpending();
    void loadPrevYearSpending();
  }

  /** Map categoryId → average spending over the last 6 full calendar months before the current viewMonth. */
  let avgSpending6m = $state<Map<number, number>>(new Map());
  /** Map categoryId → spent in the same month of the previous year. */
  let prevYearSpent = $state<Map<number, number>>(new Map());

  async function loadPrevYearSpending() {
    try {
      const arr = await api.monthlySpending(viewYear - 1, viewMonth);
      prevYearSpent = new Map(arr.map((r) => [r.categoryId, r.spentCents]));
    } catch {}
  }
  async function loadAvgSpending() {
    const months: Array<[number, number]> = [];
    let y = viewYear, m = viewMonth - 1;
    for (let i = 0; i < 6; i++) {
      if (m === 0) { m = 12; y -= 1; }
      months.push([y, m]);
      m -= 1;
    }
    try {
      const arr = await Promise.all(months.map(([yy, mm]) => api.monthlySpending(yy, mm)));
      const sums = new Map<number, number>();
      for (const monthRows of arr) {
        for (const r of monthRows) {
          sums.set(r.categoryId, (sums.get(r.categoryId) ?? 0) + r.spentCents);
        }
      }
      const avgs = new Map<number, number>();
      for (const [cid, sum] of sums) avgs.set(cid, Math.round(sum / 6));
      avgSpending6m = avgs;
    } catch {}
  }

  function applyAvgToBudget(catId: number) {
    const avg = avgSpending6m.get(catId) ?? 0;
    if (avg <= 0) return;
    debouncedSetBudget(catId, avg);
  }

  function debouncedSetBudget(catId: number, budgetCents: number) {
    const clamped = Math.max(0, Math.round(budgetCents));
    const idx = monthRows.findIndex((r) => r.categoryId === catId);
    if (idx >= 0) {
      monthRows[idx] = {
        ...monthRows[idx],
        budgetCents: clamped,
        overrideCents: clamped,
      };
    }
    ensureSliderMax(catId, viewYear, viewMonth, clamped / 100);

    const prev = saveTimers.get(catId);
    if (prev) clearTimeout(prev);
    saveTimers.set(
      catId,
      setTimeout(async () => {
        saveTimers.delete(catId);
        try {
          await api.setBudget(catId, viewYear, viewMonth, clamped);
        } catch (e) {
          console.error('setBudget failed', e);
          await reload();
        }
      }, SAVE_DEBOUNCE_MS),
    );
  }

  async function clearOverride(catId: number) {
    try {
      await api.clearBudget(catId, viewYear, viewMonth);
    } catch (e) {
      console.error('clearBudget failed', e);
    }
    await reload();
  }

  async function toggleRollover(catId: number) {
    const existing = categoriesById.get(catId);
    if (!existing) return;
    const next: Category = { ...existing, rollover_enabled: !existing.rollover_enabled };
    // Optimistic local update — also update categoriesById + monthRows entry
    // so the hint appears/disappears immediately.
    categoriesById.set(catId, next);
    categoriesById = new Map(categoriesById);
    const idx = monthRows.findIndex((r) => r.categoryId === catId);
    if (idx >= 0) {
      monthRows[idx] = { ...monthRows[idx], rolloverEnabled: next.rollover_enabled };
    }
    try {
      await api.updateCategory(next);
    } catch (e) {
      console.error('updateCategory failed', e);
    }
    await reload();
  }


  let showAddPopover = $state(false);
  let addCategoryId = $state<number | ''>('');
  let addBudgetEur = $state<number | ''>('');
  let addSaving = $state(false);

  // Categories without an effective budget for viewMonth: either null (never set)
  // or explicitly 0 (user dragged slider to 0). In both cases the category
  // should be selectable in the add popover so it can be re-activated.
  const categoriesWithoutBudget = $derived(
    monthRows
      .filter((r) => (r.budgetCents ?? 0) === 0)
      .sort((a, b) => a.categoryName.localeCompare(b.categoryName))
  );

  async function saveNewBudget() {
    if (addCategoryId === '' || addBudgetEur === '' || addSaving) return;
    const cents = Math.round(Number(addBudgetEur) * 100);
    if (!Number.isFinite(cents) || cents <= 0) return;
    addSaving = true;
    try {
      await api.setBudget(addCategoryId as number, viewYear, viewMonth, cents);
      addCategoryId = '';
      addBudgetEur = '';
      showAddPopover = false;
      await reload();
    } catch (e) {
      console.error('setBudget (add) failed', e);
    } finally {
      addSaving = false;
    }
  }

  function clickOutsideAdd(node: HTMLElement, callback: () => void) {
    function handle(ev: MouseEvent) {
      if (!node.contains(ev.target as Node)) callback();
    }
    document.addEventListener('mousedown', handle);
    return { destroy() { document.removeEventListener('mousedown', handle); } };
  }
</script>

<div class="page-sticky">
  <div class="topbar">
    <div>
      <h1>{t().nav.budgets}</h1>
      <div class="sub">{monthLabel} · {rows.length} {t().common.categories}</div>
    </div>
    <div class="seg">
      <button onclick={() => stepMonth(-1)}>{t().common.lastMonth}</button>
      <button class:on={isCurrentMonth} onclick={setThisMonth}>
        {t().common.thisMonth}
      </button>
      <button onclick={() => stepMonth(1)} aria-label="next month">
        <Icon name="chevron-right" size={12} />
      </button>
    </div>
  </div>

  <div class="tabs">
    <button
      class="tab"
      class:on={activeTab === 'month'}
      onclick={() => (activeTab = 'month')}
    >
      {t().budgets.tabMonth}
    </button>
    <button
      class="tab"
      class:on={activeTab === 'year'}
      onclick={() => (activeTab = 'year')}
    >
      {t().budgets.tabYear}
    </button>
  </div>
</div>

{#if activeTab === 'month'}
<div class="month-layout">

  <!-- KPI strip -->
  <div class="kpi-strip">
    <div class="kpi-tile">
      <div class="kpi-label">Einnahmen</div>
      <div class="kpi-value num kpi-positive">{fmtEur(monthIncomeCents, { hide: settings.hide, decimals: eurDecimals() })}</div>
    </div>
    <div class="kpi-tile">
      <div class="kpi-label">Budget gesamt</div>
      <div class="kpi-value num">{fmtEur(totalBudgetCents, { hide: settings.hide, decimals: eurDecimals() })}</div>
    </div>
    <div class="kpi-tile">
      <div class="kpi-label">{t().common.spent}</div>
      <div class="kpi-value num">{fmtEur(totalSpentCents, { hide: settings.hide, decimals: eurDecimals() })}</div>
    </div>
    <div class="kpi-tile">
      <div class="kpi-label">{t().common.remaining}</div>
      <div class="kpi-value num" class:kpi-positive={totalBudgetCents - totalSpentCents >= 0} class:kpi-negative={totalBudgetCents - totalSpentCents < 0}>
        {fmtEur(totalBudgetCents - totalSpentCents, { hide: settings.hide, decimals: eurDecimals() })}
      </div>
    </div>
    <div class="kpi-tile">
      <div class="kpi-label">Auslastung</div>
      <div class="kpi-value num" class:kpi-green={pct < 85} class:kpi-warn={pct >= 85 && pct <= 100} class:kpi-negative={pct > 100}>
        {pct.toFixed(0)}%
      </div>
    </div>
    <div class="kpi-tile">
      <div class="kpi-label">Ungeplant</div>
      <div class="kpi-value num" class:kpi-warn={nonBudgetedSpentCents > 0}>
        {fmtEur(nonBudgetedSpentCents, { hide: settings.hide, decimals: 0 })}
      </div>
    </div>
  </div>

  <!-- Hero progress bar -->
  <div class="hero-progress card card-pad-lg">
    <div class="hero-head">
      <span class="hero-label">Monatsbudget</span>
      <span class="hero-pct num" class:kpi-green={pct < 85} class:kpi-warn={pct >= 85 && pct <= 100} class:kpi-negative={pct > 100}>{pct.toFixed(0)}%</span>
      <span class="hero-remaining muted">
        {fmtEur(totalBudgetCents - totalSpentCents, { hide: settings.hide, decimals: eurDecimals() })} {t().common.remaining}
      </span>
    </div>
    <div class="bud-bar hero-bar">
      <div class={`bud-fill ${overall}`} style:width={`${Math.min(pct, 100)}%`}></div>
    </div>
  </div>

  <!-- Cash → investments reallocation -->
  <div class="card card-pad-lg invest-flow">
    <div class="card-h">
      <h3 class="section-h">Umschichtungen Geld → Anlagen</h3>
      <span class="muted">{monthLabel}</span>
    </div>
    <div class="invest-grid">
      <div class="invest-cell">
        <span class="invest-label">Käufe</span>
        <span class="invest-val neg">−{fmtEur(investmentFlow.buysCents, { hide: settings.hide, decimals: 0 })}</span>
      </div>
      <div class="invest-cell">
        <span class="invest-label">Verkäufe</span>
        <span class="invest-val pos">+{fmtEur(investmentFlow.sellsCents, { hide: settings.hide, decimals: 0 })}</span>
      </div>
      <div class="invest-cell">
        <span class="invest-label">Dividenden</span>
        <span class="invest-val pos">+{fmtEur(investmentFlow.dividendsCents, { hide: settings.hide, decimals: 0 })}</span>
      </div>
      <div class="invest-cell invest-net">
        <span class="invest-label">Netto in Anlagen</span>
        <span class="invest-val" class:pos={investmentFlow.netInvestedCents > 0} class:neg={investmentFlow.netInvestedCents < 0}>
          {investmentFlow.netInvestedCents > 0 ? '+' : ''}{fmtEur(investmentFlow.netInvestedCents, { hide: settings.hide, decimals: 0 })}
        </span>
      </div>
    </div>
    <div class="invest-hint muted">
      Wertpapierkäufe sind keine Konsum-Ausgaben — sie verschieben Cash in dein Depot. Dividenden fließen separat als Einnahmen ins Cashflow-KPI.
    </div>
  </div>

  <!-- Heatmap -->
  <div class="card card-pad-lg section-gap">
    <div class="card-h">
      <h3 class="section-h">{t().common.categoryHeatmap}</h3>
      <span class="mono muted">{monthLabel}</span>
    </div>
    <figure>
      <SpendingHeatmap
        daily={dailyEur}
        offset={heatmapOffset}
        monthLabel={monthLabel}
        hide={settings.hide}
      />
      <figcaption class="sr-only">Tägliche Ausgaben im {monthLabel}</figcaption>
    </figure>
  </div>

  <!-- Category card grid -->
  <div class="section-gap">
    <div class="card-h section-head-row">
      <h3 class="section-h">{t().common.perCat}</h3>
      <div class="add-budget-wrap" use:clickOutsideAdd={() => (showAddPopover = false)}>
        <button
          type="button"
          class="btn"
          onclick={() => (showAddPopover = !showAddPopover)}
          aria-expanded={showAddPopover}
          disabled={categoriesWithoutBudget.length === 0}
          title={categoriesWithoutBudget.length === 0
            ? 'Alle Kategorien haben bereits ein Budget für diesen Monat'
            : 'Budget für eine Kategorie festlegen'}
        >
          <Icon name="plus" size={13} /> {t().common.add}
        </button>
        {#if showAddPopover}
          <div class="add-popover" role="dialog" aria-modal="true">
            <div class="add-popover-h">Budget hinzufügen</div>
            <label class="add-row">
              <span class="add-label">Kategorie</span>
              <select class="input" bind:value={addCategoryId}>
                <option value="">— wählen —</option>
                {#each categoriesWithoutBudget as c (c.categoryId)}
                  <option value={c.categoryId}>{c.categoryName}</option>
                {/each}
              </select>
            </label>
            <label class="add-row">
              <span class="add-label">Budget (€)</span>
              <input
                type="number"
                class="input"
                min="0"
                step="10"
                bind:value={addBudgetEur}
                placeholder="0"
                onkeydown={(e) => { if (e.key === 'Enter') void saveNewBudget(); }}
              />
            </label>
            <div class="add-popover-actions">
              <button type="button" class="btn ghost" onclick={() => (showAddPopover = false)}>
                Abbrechen
              </button>
              <button
                type="button"
                class="btn primary"
                onclick={saveNewBudget}
                disabled={addCategoryId === '' || addBudgetEur === '' || Number(addBudgetEur) <= 0 || addSaving}
              >
                {addSaving ? '…' : 'Speichern'}
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>

    {#if loading && monthRows.length === 0}
      <div class="empty">…</div>
    {:else if budgetedRows.length === 0 && unbudgetedRows.length === 0 && nonBudgetedSpentCents === 0}
      <EmptyState
        icon="budget"
        title="Noch keine Budget-Daten"
        description="Sobald du Ausgaben in Kategorien erfasst, erscheinen hier deine Budgets."
      />
    {:else}
      <div class="cat-grid">
        <!-- "Other" card always first -->
        <div class="cat-card cat-card-sonstige">
          <header class="cat-card-h">
            <span class="cat-ic" style:color="var(--accent)">
              <Icon name="tag" size={14} />
            </span>
            <div class="cat-meta">
              <div class="cat-name">Sonstige Ausgaben</div>
              <div class="cat-sub muted">
                Nicht-budgetierte Kategorien + Tx ohne Kategorie
                {#if uncategorizedSpentCents > 0}
                  · {fmtEur(uncategorizedSpentCents, { hide: settings.hide, decimals: 0 })} unkategorisiert
                {/if}
              </div>
            </div>
          </header>
          <div class="cat-numbers">
            <div class="cat-spent num" class:over={sonstigeCls === 'over'}>
              {fmtEur(nonBudgetedSpentCents, { hide: settings.hide, decimals: 2 })}
            </div>
            <div class="cat-budget-of muted">/ {fmtEur(sonstigeBudgetCents, { hide: settings.hide, decimals: 0 })}</div>
          </div>
          <div class="bud-bar cat-bar">
            <div class={`bud-fill ${sonstigeCls}`} style:width={`${Math.min(sonstigeP, 100)}%`}></div>
          </div>
          <div class="cat-foot">
            <span class="cat-pct" class:warn={sonstigeCls === 'warn'} class:over={sonstigeCls === 'over'}>
              {sonstigeBudgetCents > 0 ? sonstigeP.toFixed(0) + '%' : '—'}
            </span>
            <input
              type="text"
              inputmode="decimal"
              class="input cat-budget-input"
              value={sonstigeBudgetEur}
              oninput={(e) => updateSonstigeBudget(Math.round(Number((e.target as HTMLInputElement).value) * 100))}
            />
          </div>
          <input
            type="range"
            min={0}
            max={Math.max(sonstigeBudgetEur * 2, 300)}
            step={10}
            value={sonstigeBudgetEur}
            oninput={(e) => updateSonstigeBudget(Math.round(Number((e.target as HTMLInputElement).value) * 100))}
            class="slider"
            aria-label="Sonstige Budget"
          />
        </div>

        {#each budgetedRows as r (r.categoryId)}
          {@const p = (r.budgetCents ?? 0) > 0 ? (r.spentCents / (r.budgetCents ?? 1)) * 100 : 0}
          {@const cls = p > 100 ? 'over' : p > 85 ? 'warn' : ''}
          {@const budgetEur = (r.budgetCents ?? 0) / 100}
          {@const cat = categoriesById.get(r.categoryId)}
          <div class="cat-card">
            <header class="cat-card-h">
              <span class="cat-ic" style:color={cat?.color ?? 'var(--text-muted)'}>
                <Icon name={cat?.icon ?? 'tag'} size={14} />
              </span>
              <div class="cat-meta">
                <div class="cat-name">{r.categoryName}</div>
                {#if r.budgetCents === null}<div class="cat-sub muted">{t().budgets.notSet}</div>{/if}
              </div>
              <div class="cat-actions">
                {#if r.budgetCents !== null}
                  <button class="iconbtn cat-remove" type="button" title="Budget entfernen — Karte verschwindet aus dieser Monatsansicht" onclick={() => clearOverride(r.categoryId)} aria-label="Budget entfernen">
                    <Icon name="x" size={11}/>
                  </button>
                {/if}
                <button class="iconbtn rollover-toggle" type="button" class:on={r.rolloverEnabled} title={t().budgets.rolloverHint} aria-label={t().budgets.rolloverHint} onclick={() => toggleRollover(r.categoryId)}>
                  <Icon name="repeat" size={11}/>
                </button>
                {#if (avgSpending6m.get(r.categoryId) ?? 0) > 0}
                  <button class="iconbtn avg-apply" type="button" title="∅ der letzten 6 Monate als Budget setzen ({fmtEur(avgSpending6m.get(r.categoryId) ?? 0, { hide: settings.hide, decimals: 0 })})" aria-label="Durchschnitt der letzten 6 Monate als Budget setzen" onclick={() => applyAvgToBudget(r.categoryId)}>∅</button>
                {/if}
              </div>
            </header>

            <div class="cat-numbers">
              <div class="cat-spent num" class:over={cls === 'over'}>{fmtEur(r.spentCents, { hide: settings.hide, decimals: 2 })}</div>
              <div class="cat-budget-of muted">/ {fmtEur(r.budgetCents ?? 0, { hide: settings.hide, decimals: 0 })}</div>
            </div>

            <div class="bud-bar cat-bar">
              <div class={`bud-fill ${cls}`} style:width={`${Math.min(p, 100)}%`}></div>
            </div>

            <div class="cat-foot">
              <span class="cat-pct" class:warn={cls === 'warn'} class:over={cls === 'over'}>{p.toFixed(0)}%</span>
              {#if (prevYearSpent.get(r.categoryId) ?? 0) > 0}
                {@const prev = prevYearSpent.get(r.categoryId) ?? 0}
                {@const diff = prev > 0 ? ((r.spentCents - prev) / prev) * 100 : 0}
                <span class="prev-year" title="∆ zu {viewYear - 1}/{String(viewMonth).padStart(2, '0')}: {fmtEur(prev, { hide: settings.hide, decimals: 0 })}">
                  {diff >= 0 ? '▲' : '▼'} {Math.abs(diff).toFixed(0)}%
                </span>
              {/if}
              <input
                type="text"
                inputmode="decimal"
                class="input cat-budget-input"
                value={budgetEur}
                oninput={(e) => debouncedSetBudget(r.categoryId, parseEurCents((e.target as HTMLInputElement).value))}
              />
            </div>

            <input
              type="range"
              min={0}
              max={sliderMaxEur.get(sliderKey(r.categoryId, viewYear, viewMonth)) ?? SLIDER_MIN_MAX_EUR}
              step={10}
              value={budgetEur}
              oninput={(e) => debouncedSetBudget(r.categoryId, parseEurCents((e.target as HTMLInputElement).value))}
              class="slider"
              aria-label={`${r.categoryName} budget`}
            />

            {#if r.rolloverEnabled && r.rolloverCents !== 0}
              <div class="rollover-hint" class:deficit={r.rolloverCents < 0}>
                {(r.rolloverCents < 0 ? t().budgets.deficit : t().budgets.rolloverFrom)
                  .replace('{eur}', fmtEur(Math.abs(r.rolloverCents), { hide: settings.hide, decimals: eurDecimals() }))
                  .replace('{month}', prevMonthLabel(viewYear, viewMonth))}
              </div>
            {/if}
          </div>
        {/each}
      </div>

      {#if unbudgetedRows.length > 0}
        <div class="unbudgeted-section">
          <div class="unbudgeted-h">
            <span>Außerhalb der Budgets</span>
            <span class="unbudgeted-hint">Diese Kategorien haben Ausgaben aber kein Budget. Klick auf ➕ oben oder den Slider in der Karte um eines anzulegen.</span>
          </div>
          <div class="cat-grid">
            {#each unbudgetedRows as r (r.categoryId)}
              {@const p = (r.budgetCents ?? 0) > 0 ? (r.spentCents / (r.budgetCents ?? 1)) * 100 : 0}
              {@const cls = p > 100 ? 'over' : p > 85 ? 'warn' : ''}
              {@const budgetEur = (r.budgetCents ?? 0) / 100}
              {@const cat = categoriesById.get(r.categoryId)}
              <div class="cat-card">
                <header class="cat-card-h">
                  <span class="cat-ic" style:color={cat?.color ?? 'var(--text-muted)'}>
                    <Icon name={cat?.icon ?? 'tag'} size={14} />
                  </span>
                  <div class="cat-meta">
                    <div class="cat-name">{r.categoryName}</div>
                    {#if r.budgetCents === null}<div class="cat-sub muted">{t().budgets.notSet}</div>{/if}
                  </div>
                  <div class="cat-actions">
                    {#if r.budgetCents !== null}
                      <button class="iconbtn cat-remove" type="button" title="Budget entfernen — Karte verschwindet aus dieser Monatsansicht" onclick={() => clearOverride(r.categoryId)} aria-label="Budget entfernen">
                        <Icon name="x" size={11}/>
                      </button>
                    {/if}
                    <button class="iconbtn rollover-toggle" type="button" class:on={r.rolloverEnabled} title={t().budgets.rolloverHint} aria-label={t().budgets.rolloverHint} onclick={() => toggleRollover(r.categoryId)}>
                      <Icon name="repeat" size={11}/>
                    </button>
                    {#if (avgSpending6m.get(r.categoryId) ?? 0) > 0}
                      <button class="iconbtn avg-apply" type="button" title="∅ der letzten 6 Monate als Budget setzen ({fmtEur(avgSpending6m.get(r.categoryId) ?? 0, { hide: settings.hide, decimals: 0 })})" aria-label="Durchschnitt der letzten 6 Monate als Budget setzen" onclick={() => applyAvgToBudget(r.categoryId)}>∅</button>
                    {/if}
                  </div>
                </header>

                <div class="cat-numbers">
                  <div class="cat-spent num" class:over={cls === 'over'}>{fmtEur(r.spentCents, { hide: settings.hide, decimals: 2 })}</div>
                  <div class="cat-budget-of muted">/ {fmtEur(r.budgetCents ?? 0, { hide: settings.hide, decimals: 0 })}</div>
                </div>

                <div class="bud-bar cat-bar">
                  <div class={`bud-fill ${cls}`} style:width={`${Math.min(p, 100)}%`}></div>
                </div>

                <div class="cat-foot">
                  <span class="cat-pct" class:warn={cls === 'warn'} class:over={cls === 'over'}>{p.toFixed(0)}%</span>
                  {#if (prevYearSpent.get(r.categoryId) ?? 0) > 0}
                    {@const prev = prevYearSpent.get(r.categoryId) ?? 0}
                    {@const diff = prev > 0 ? ((r.spentCents - prev) / prev) * 100 : 0}
                    <span class="prev-year" title="∆ zu {viewYear - 1}/{String(viewMonth).padStart(2, '0')}: {fmtEur(prev, { hide: settings.hide, decimals: 0 })}">
                      {diff >= 0 ? '▲' : '▼'} {Math.abs(diff).toFixed(0)}%
                    </span>
                  {/if}
                  <input
                    type="text"
                    inputmode="decimal"
                    class="input cat-budget-input"
                    value={budgetEur}
                    oninput={(e) => debouncedSetBudget(r.categoryId, parseEurCents((e.target as HTMLInputElement).value))}
                  />
                </div>

                <input
                  type="range"
                  min={0}
                  max={sliderMaxEur.get(sliderKey(r.categoryId, viewYear, viewMonth)) ?? SLIDER_MIN_MAX_EUR}
                  step={10}
                  value={budgetEur}
                  oninput={(e) => debouncedSetBudget(r.categoryId, parseEurCents((e.target as HTMLInputElement).value))}
                  class="slider"
                  aria-label={`${r.categoryName} budget`}
                />

                {#if r.rolloverEnabled && r.rolloverCents !== 0}
                  <div class="rollover-hint" class:deficit={r.rolloverCents < 0}>
                    {(r.rolloverCents < 0 ? t().budgets.deficit : t().budgets.rolloverFrom)
                      .replace('{eur}', fmtEur(Math.abs(r.rolloverCents), { hide: settings.hide, decimals: eurDecimals() }))
                      .replace('{month}', prevMonthLabel(viewYear, viewMonth))}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>

</div>
{:else}
  <div class="card card-pad-lg">
    <div class="card-h">
      <h3>{t().budgets.year} {viewYear}</h3>
      <div class="seg">
        <button onclick={() => (viewYear = viewYear - 1)} aria-label="prev year">
          <span style="display: inline-block; transform: rotate(180deg);">
            <Icon name="chevron-right" size={12} />
          </span>
        </button>
        <button
          class:on={viewYear === currentYear}
          onclick={() => (viewYear = currentYear)}
        >
          {currentYear}
        </button>
        <button onclick={() => (viewYear = viewYear + 1)} aria-label="next year">
          <Icon name="chevron-right" size={12} />
        </button>
      </div>
    </div>
    <BudgetYearOverview year={viewYear} />
  </div>
{/if}

<style>
  /* ── Sticky topbar ──────────────────────────────────────────────── */
  .page-sticky {
    position: sticky;
    top: 0;
    z-index: 5;
    background: var(--surface);
    padding-top: 4px;
    margin-top: -4px;
  }

  /* ── Month layout wrapper ──────────────────────────────────────── */
  .month-layout {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .section-gap {
    margin-top: 8px;
  }

  .section-h {
    font-size: 18px;
    font-weight: 600;
  }

  .section-head-row {
    margin-bottom: 12px;
  }

  /* ── KPI-Strip ─────────────────────────────────────────────────── */
  .kpi-strip {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 10px;
  }
  @media (max-width: 1023px) {
    .kpi-strip { grid-template-columns: repeat(3, 1fr); }
  }
  @media (max-width: 599px) {
    /* KPI-Strip: horizontal-scroll on phone */
    .kpi-strip {
      grid-template-columns: none;
      display: flex;
      gap: 10px;
      overflow-x: auto;
      scroll-snap-type: x mandatory;
      margin: 0 -16px 14px;
      padding: 0 16px 6px;
    }
    .kpi-strip > * {
      flex: 0 0 60%;
      max-width: 220px;
      scroll-snap-align: start;
    }

    /* Budget cards: 1-column on phone */
    .cat-grid { grid-template-columns: 1fr; gap: 8px; }
  }

  .kpi-tile {
    background: var(--surface-1, var(--surface-2));
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .kpi-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .kpi-value {
    font-size: 20px;
    font-weight: 600;
    color: var(--text);
    line-height: 1.2;
  }

  .kpi-positive { color: var(--positive); }
  .kpi-negative { color: var(--negative); }
  .kpi-green    { color: var(--positive); }
  .kpi-warn     { color: var(--warning); }

  /* ── Hero Progress Bar ─────────────────────────────────────────── */
  .hero-progress {
    padding: 16px 20px !important;
  }

  .hero-head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 10px;
  }

  .hero-label {
    font-size: 14px;
    font-weight: 600;
    flex: 1;
  }

  .hero-pct {
    font-size: 14px;
    font-weight: 600;
  }

  .hero-remaining {
    font-size: 12px;
  }

  .hero-bar {
    height: 12px !important;
  }

  /* ── Progress Bars ─────────────────────────────────────────────── */
  .bud-bar {
    height: 8px;
    width: 100%;
    background: var(--surface-2);
    border-radius: 999px;
    overflow: hidden;
    position: relative;
  }

  .bud-fill {
    height: 100%;
    border-radius: 999px;
    transition: width 360ms cubic-bezier(.4, 0, .2, 1), background 200ms;
    background: linear-gradient(90deg, var(--positive) 0%, var(--positive) 100%);
  }

  @media (prefers-reduced-motion: reduce) {
    .bud-fill { transition: none; }
  }

  .bud-fill.warn {
    background: linear-gradient(90deg, var(--positive) 0%, var(--warning) 100%);
  }

  .bud-fill.over {
    background: linear-gradient(90deg, var(--warning) 0%, var(--negative) 100%);
  }

  /* ── Category Card Grid ────────────────────────────────────────── */
  .cat-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 12px;
  }

  .cat-card {
    background: var(--surface-1, var(--surface-2));
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  /* Card Header */
  .cat-card-h {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .cat-ic {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: var(--surface-2);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .cat-meta {
    flex: 1;
    min-width: 0;
  }

  .cat-name {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cat-sub {
    font-size: 11px;
    color: var(--text-faint);
    margin-top: 1px;
  }

  .cat-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  /* Action icon-buttons */
  .iconbtn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 3px 6px;
    cursor: pointer;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    font-size: 11px;
    font-family: inherit;
    line-height: 1;
  }

  .iconbtn:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }

  .iconbtn.rollover-toggle.on {
    color: var(--accent);
    border-color: var(--accent);
    background: var(--accent-soft, var(--surface-2));
  }

  .iconbtn.avg-apply:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .iconbtn.cat-remove:hover {
    color: var(--negative);
    border-color: var(--negative);
    background: color-mix(in srgb, var(--negative) 8%, transparent);
  }

  /* Numbers row */
  .cat-numbers {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .cat-spent {
    font-size: 20px;
    font-weight: 600;
    line-height: 1.1;
  }

  .cat-spent.over {
    color: var(--negative);
  }

  .cat-budget-of {
    font-size: 13px;
    color: var(--text-faint);
    font-family: var(--font-mono);
  }

  /* Cat bar */
  .cat-bar {
    height: 8px;
  }

  /* Footer: pct + prev-year + budget input */
  .cat-foot {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .cat-pct {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    min-width: 32px;
  }

  .cat-pct.warn { color: var(--warning); }
  .cat-pct.over { color: var(--negative); }

  .cat-budget-input {
    margin-left: auto;
    width: 80px;
    padding: 4px 8px;
    text-align: right;
    font-size: 13px;
  }

  /* Slider */
  .slider {
    width: 100%;
    accent-color: var(--accent);
  }

  /* Rollover hint */
  .rollover-hint {
    font-size: 11px;
    color: var(--accent);
    margin-top: -4px;
  }

  .rollover-hint.deficit {
    color: var(--negative);
  }

  /* Prev year badge */
  .prev-year {
    font-size: 10px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  /* Misc */
  .muted {
    font-size: 11.5px;
    color: var(--text-faint);
  }

  .over {
    color: var(--negative);
  }

  .empty {
    padding: 24px;
    text-align: center;
    color: var(--text-faint);
    font-size: 13px;
  }

  /* Tabs */
  .tabs {
    display: flex;
    gap: 0;
    margin: -8px 0 0;
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
  }

  .tab.on {
    color: var(--text);
    border-bottom-color: var(--accent);
  }

  .tab:hover:not(.on) {
    color: var(--text);
  }

  /* ── Add-Budget Popover ────────────────────────────────────────── */
  .add-budget-wrap {
    position: relative;
    display: inline-block;
  }
  .add-popover {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 20;
    width: min(320px, calc(100vw - 32px));
    padding: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .add-popover-h {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .add-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .add-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-faint);
  }
  .add-popover-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
    border-top: 1px solid var(--border);
  }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-fg, white);
    border-color: var(--accent);
  }
  .btn.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* ── Sonstige-Card ─────────────────────────────────────────────── */
  .cat-card-sonstige {
    border-style: dashed;
    background: color-mix(in srgb, var(--accent) 4%, var(--surface));
  }

  /* ── Investment Flow Card ──────────────────────────────────────── */
  .invest-flow {
    margin-bottom: 16px;
  }
  .invest-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-top: 8px;
  }
  /* invest-grid: intermediate breakpoint kept at 760px (4→2 cols inside the invest-flow card) */
  @media (max-width: 760px) {
    .invest-grid { grid-template-columns: repeat(2, 1fr); }
  }
  .invest-cell {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 12px;
    background: var(--surface-2);
    border-radius: 8px;
  }
  .invest-cell.invest-net {
    background: color-mix(in srgb, var(--accent) 8%, var(--surface-2));
    border: 1px solid color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .invest-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-faint);
    font-weight: 600;
  }
  .invest-val {
    font-size: 18px;
    font-weight: 600;
    font-family: var(--font-mono);
  }
  .invest-val.pos { color: var(--positive); }
  .invest-val.neg { color: var(--negative); }
  .invest-hint {
    margin-top: 10px;
    font-size: 11.5px;
    color: var(--text-faint);
  }

  /* ── Unbudgeted Section ────────────────────────────────────────── */
  .unbudgeted-section {
    margin-top: 20px;
    padding-top: 16px;
    border-top: 1px dashed var(--border);
  }

  .unbudgeted-h {
    margin-bottom: 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .unbudgeted-h > span:first-child {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .unbudgeted-hint {
    font-size: 12px;
    color: var(--text-faint);
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

  figure {
    margin: 0;
    padding: 0;
  }
</style>
