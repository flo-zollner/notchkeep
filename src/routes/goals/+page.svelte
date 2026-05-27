<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import GoalCard from '$lib/components/GoalCard.svelte';
  import GoalModal from '$lib/components/GoalModal.svelte';
  import { t } from '$lib/settings.svelte';
  import {
    api,
    type Goal,
    type GoalProgress,
    type Category,
  } from '$lib/api';

  let goals = $state<Goal[]>([]);
  let progress = $state<Record<number, GoalProgress>>({});
  let categories = $state<Category[]>([]);
  let includeArchived = $state(false);
  let loading = $state(true);

  let modalOpen = $state(false);
  let editGoal = $state<Goal | undefined>(undefined);

  const tg = $derived(t().goals);

  $effect(() => {
    void load(includeArchived);
  });

  async function load(showArchived: boolean) {
    loading = true;
    try {
      const [g, p, c] = await Promise.all([
        api.listGoals(showArchived),
        api.listGoalProgress(showArchived),
        api.listCategories(),
      ]);
      goals = g;
      categories = c;
      progress = Object.fromEntries(p.map((x) => [x.goalId, x]));
    } finally {
      loading = false;
    }
  }

  function categoryOf(g: Goal): Category | undefined {
    return categories.find((c) => c.id === g.categoryId);
  }

  function openNew() {
    editGoal = undefined;
    modalOpen = true;
  }
  function openEdit(g: Goal) {
    editGoal = g;
    modalOpen = true;
  }
  function onSaved() {
    void load(includeArchived);
  }
  function onDeleted() {
    void load(includeArchived);
  }
</script>

<header class="page-h">
  <h1>{tg.title}</h1>
  <div class="actions">
    <label class="toggle">
      <input type="checkbox" bind:checked={includeArchived} />
      <span>{tg.showArchived}</span>
    </label>
    <button class="primary" type="button" onclick={openNew}>
      <Icon name="plus" size={14} /> {tg.new}
    </button>
  </div>
</header>

{#if loading}
  <p class="muted">…</p>
{:else if goals.length === 0}
  <div class="empty">
    <p>{tg.empty}</p>
    <button class="primary" type="button" onclick={openNew}>{tg.emptyCta}</button>
  </div>
{:else}
  <div class="grid">
    {#each goals as g (g.id)}
      <GoalCard
        goal={g}
        progress={progress[g.id] ?? { goalId: g.id, currentCents: 0, monthlyAvgCents: 0, forecastDate: null, onTrack: null }}
        category={categoryOf(g)}
        onEdit={() => openEdit(g)}
      />
    {/each}
  </div>
{/if}

{#if modalOpen}
  <GoalModal
    goal={editGoal}
    {categories}
    onClose={() => (modalOpen = false)}
    onSaved={onSaved}
    onDeleted={onDeleted}
  />
{/if}

<style>
  .page-h {
    display: flex; align-items: center; gap: 12px; margin-bottom: 16px;
  }
  .page-h h1 { flex: 1; margin: 0; font-size: 22px; }
  .actions { display: flex; align-items: center; gap: 12px; }
  .toggle { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-muted); }
  .primary { background: var(--accent); color: var(--accent-fg, white); border: 0; padding: 8px 12px; border-radius: 8px; cursor: pointer; display: inline-flex; align-items: center; gap: 6px; }
  .grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }
  .empty { padding: 48px 0; text-align: center; color: var(--text-muted); }
  .empty button { margin-top: 14px; }
  .muted { color: var(--text-muted); }

  @media (max-width: 599px) {
    .page-h { flex-wrap: wrap; gap: 8px; }
    .page-h h1 { font-size: 20px; }

    /* Goal cards grid → 1 column */
    .grid { grid-template-columns: 1fr; gap: 8px; }
  }
</style>
