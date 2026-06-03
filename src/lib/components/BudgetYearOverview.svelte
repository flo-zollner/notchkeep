<script lang="ts">
  import { api, type CategoryMonthBudget } from '$lib/api';
  import { fmtEur, fmtEurInput, parseEur } from '$lib/format';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';

  interface Props {
    year: number;
  }
  let { year }: Props = $props();

  let matrix = $state<CategoryMonthBudget[][]>([]);
  let categoryOrder = $state<{ id: number; name: string }[]>([]);
  let loading = $state(true);

  type EditKey = { catId: number; month: number };
  let editing = $state<EditKey | null>(null);
  let editValue = $state('');

  async function load() {
    loading = true;
    try {
      const overviews = await Promise.all(
        Array.from({ length: 12 }, (_, i) => api.monthOverview(year, i + 1)),
      );
      const seen = new Map<number, string>();
      for (const m of overviews) {
        for (const row of m) {
          if (!seen.has(row.categoryId)) seen.set(row.categoryId, row.categoryName);
        }
      }
      categoryOrder = Array.from(seen, ([id, name]) => ({ id, name }))
        .sort((a, b) => a.name.localeCompare(b.name));
      const lookups = overviews.map((rows) => new Map(rows.map((r) => [r.categoryId, r])));
      matrix = categoryOrder.map((cat) =>
        lookups.map((mp) => mp.get(cat.id)!),
      );
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void year;
    load();
  });

  function rowSum(row: CategoryMonthBudget[]): number {
    return row.reduce((s, b) => s + (b?.budgetCents ?? 0), 0);
  }

  function isOverride(b: CategoryMonthBudget | undefined): boolean {
    return !!b && b.overrideCents !== null && b.overrideCents !== undefined;
  }

  function startEdit(catId: number, month1: number, value: number) {
    editing = { catId, month: month1 };
    editValue = value > 0 ? fmtEurInput(value) : '';
  }

  async function commitEdit() {
    if (!editing) return;
    const { catId, month } = editing;
    const raw = editValue.trim();
    editing = null;
    try {
      if (raw === '') {
        await api.clearBudget(catId, year, month);
      } else {
        const n = parseEur(raw);
        if (Number.isFinite(n) && n >= 0) {
          await api.setBudget(catId, year, month, Math.round(n * 100));
        }
      }
    } catch (e) {
      console.error('cell commit failed', e);
    }
    await load();
  }

  function cancelEdit() {
    editing = null;
    editValue = '';
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') commitEdit();
    else if (e.key === 'Escape') cancelEdit();
  }

  const monthsShort = $derived.by(() =>
    settings.lang === 'en'
      ? ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec']
      : ['Jan','Feb','Mär','Apr','Mai','Jun','Jul','Aug','Sep','Okt','Nov','Dez']
  );
</script>

{#if loading && matrix.length === 0}
  <div class="empty">…</div>
{:else if matrix.length === 0}
  <EmptyState icon="budget" title="Noch keine Budgets" description="Lege Budgets für deine Kategorien an, um Soll/Ist zu sehen." />
{:else}
  <div class="table-wrap">
    <table class="year-table">
      <thead>
        <tr>
          <th class="sticky">{t().common.categories}</th>
          {#each monthsShort as m, i (i)}<th>{m}</th>{/each}
          <th>{t().budgets.sum}</th>
        </tr>
      </thead>
      <tbody>
        {#each categoryOrder as cat, ci (cat.id)}
          <tr>
            <td class="sticky cat">{cat.name}</td>
            {#each matrix[ci] as cell, mi (mi)}
              {@const isEdit = editing?.catId === cat.id && editing?.month === mi + 1}
              <td
                class="cell"
                class:override={isOverride(cell)}
                class:fill={cell && !isOverride(cell) && cell.budgetCents !== null}
                class:editing={isEdit}
                title={t().budgets.editCell}
                onclick={() => !isEdit && startEdit(cat.id, mi + 1, cell?.budgetCents ?? 0)}
              >
                {#if isEdit}
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    type="text"
                    class="cell-input"
                    bind:value={editValue}
                    onblur={commitEdit}
                    onkeydown={onKey}
                    autofocus
                  />
                {:else if cell && cell.budgetCents !== null}
                  {fmtEur(cell.budgetCents, { hide: settings.hide, decimals: eurDecimals() })}
                {:else}
                  <span class="muted">—</span>
                {/if}
              </td>
            {/each}
            <td class="sum">{fmtEur(rowSum(matrix[ci]), { hide: settings.hide, decimals: eurDecimals() })}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .table-wrap {
    overflow-x: auto;
    max-width: 100%;
  }
  .year-table {
    border-collapse: collapse;
    font-size: 12px;
    width: 100%;
    min-width: 720px;
  }
  .year-table th,
  .year-table td {
    padding: 8px 10px;
    text-align: right;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }
  .year-table th.sticky,
  .year-table td.sticky {
    position: sticky;
    left: 0;
    background: var(--surface);
    text-align: left;
    z-index: 1;
  }
  .year-table thead th {
    color: var(--text-muted);
    font-weight: 500;
    font-size: 11px;
  }
  td.cat {
    color: var(--text);
  }
  td.cell {
    cursor: pointer;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  td.cell.override {
    color: var(--text);
  }
  td.cell.fill {
    color: var(--text-muted);
    font-style: italic;
  }
  td.cell:hover:not(.editing) {
    background: var(--surface-2);
  }
  td.cell.editing {
    padding: 4px;
  }
  .cell-input {
    width: 100%;
    background: var(--surface-2);
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 4px 6px;
    color: var(--text);
    font-size: 12px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  td.sum {
    color: var(--text);
    font-weight: 500;
    border-left: 1px solid var(--border);
  }
  .muted {
    color: var(--text-faint);
  }
</style>
