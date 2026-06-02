<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import RecurringModal from '$lib/components/RecurringModal.svelte';
  import DetectSuggestionsModal from '$lib/components/DetectSuggestionsModal.svelte';
  import UpcomingRow from '$lib/components/UpcomingRow.svelte';
  import RecurringRow from '$lib/components/RecurringRow.svelte';
  import { t } from '$lib/settings.svelte';
  import {
    api,
    type RecurringPayment, type RecurringOverview, type Occurrence,
    type Account, type Category,
  } from '$lib/api';
  import { page } from '$app/state';

  let recurrings = $state<RecurringPayment[]>([]);
  let overview = $state<RecurringOverview[]>([]);
  let accounts = $state<Account[]>([]);
  let categories = $state<Category[]>([]);
  let showArchived = $state(false);
  let loading = $state(true);

  let editing = $state<RecurringPayment | null>(null);
  let addingNew = $state(false);
  let detecting = $state(false);

  const tr = $derived(t().recurring);

  let fabHandled = false;
  $effect(() => {
    if (!fabHandled && page.url.searchParams.get('new') === '1') {
      fabHandled = true;
      addingNew = true;
    }
  });

  async function loadAll() {
    loading = true;
    try {
      const [rs, ov, accs, cats] = await Promise.all([
        api.listRecurring(showArchived),
        api.recurringOverview(3),
        api.listAccounts(),
        api.listCategories(),
      ]);
      recurrings = rs;
      overview = ov;
      accounts = accs;
      categories = cats;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void showArchived;
    void loadAll();
  });

  const upcoming = $derived.by(() => {
    const out: Array<{ recurring: RecurringPayment; occurrence: Occurrence }> = [];
    for (const o of overview) {
      for (const occ of o.occurrences) {
        out.push({ recurring: o.recurring, occurrence: occ });
      }
    }
    out.sort((a, b) => a.occurrence.dueDate.localeCompare(b.occurrence.dueDate));
    return out.slice(0, 50);
  });

  async function archiveToggle(r: RecurringPayment) {
    try {
      await api.updateRecurring(r.id, { archived: !r.archived });
      loadAll();
    } catch (e) {
      console.error('archiveToggle', e);
    }
  }
</script>

<header class="page-h">
  <h1>{tr.title}</h1>
  <div class="actions">
    <button class="secondary" type="button" onclick={() => (detecting = true)}>
      {tr.fromHistory}
    </button>
    <button class="primary" type="button" onclick={() => (addingNew = true)}>
      <Icon name="plus" size={13} /> {tr.add}
    </button>
  </div>
</header>

{#if loading && recurrings.length === 0}
  <p class="muted">…</p>
{:else}
  <section>
    <h2>{tr.upcoming}</h2>
    {#if upcoming.length === 0}
      <p class="muted">—</p>
    {:else}
      <ul class="upcoming-list">
        {#each upcoming as u (`${u.recurring.id}-${u.occurrence.dueDate}`)}
          <li><UpcomingRow recurring={u.recurring} occurrence={u.occurrence} onclick={() => (editing = u.recurring)} /></li>
        {/each}
      </ul>
    {/if}
  </section>

  <section>
    <div class="sec-head">
      <h2>{tr.active} ({recurrings.filter((r) => !r.archived).length})</h2>
      <label class="toggle">
        <input type="checkbox" bind:checked={showArchived} />
        {tr.showArchived}
      </label>
    </div>
    {#if recurrings.length === 0}
      <p class="muted">—</p>
    {:else}
      <div class="rec-list">
        {#each recurrings as r (r.id)}
          <RecurringRow recurring={r} onEdit={(x) => (editing = x)} onArchiveToggle={archiveToggle} />
        {/each}
      </div>
    {/if}
  </section>
{/if}

{#if addingNew}
  <RecurringModal
    recurring={null}
    accounts={accounts}
    categories={categories}
    onClose={() => (addingNew = false)}
    onSaved={() => { addingNew = false; loadAll(); }}
  />
{/if}
{#if editing}
  <RecurringModal
    recurring={editing}
    accounts={accounts}
    categories={categories}
    onClose={() => (editing = null)}
    onSaved={() => { editing = null; loadAll(); }}
    onDeleted={() => { editing = null; loadAll(); }}
  />
{/if}
{#if detecting}
  <DetectSuggestionsModal
    accounts={accounts}
    onClose={() => (detecting = false)}
    onCreated={() => { detecting = false; loadAll(); }}
  />
{/if}

<style>
  .page-h {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 20px;
  }
  .page-h h1 { margin: 0; font-size: 22px; letter-spacing: -0.02em; }
  .actions { display: flex; gap: 8px; }
  .actions button {
    background: var(--surface-2); border: 1px solid var(--border);
    border-radius: 8px; padding: 6px 12px; cursor: pointer;
    font: inherit; color: var(--text); display: inline-flex; gap: 6px; align-items: center;
    font-size: 13px;
  }
  .actions .primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); }
  .muted { color: var(--text-muted); font-size: 13px; }
  section { margin-bottom: 24px; }
  section h2 { font-size: 14px; font-weight: 500; margin: 0 0 12px; }
  .sec-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
  .sec-head h2 { margin: 0; }
  .toggle { display: flex; gap: 4px; align-items: center; font-size: 12px; color: var(--text-muted); }
  .upcoming-list, .rec-list { list-style: none; padding: 0; margin: 0; display: grid; gap: 4px; }
  .upcoming-list li { list-style: none; }

  @media (max-width: 599px) {
    .page-h { flex-wrap: wrap; gap: 8px; }
    .page-h h1 { font-size: 20px; }

    /* Actions wrap below title */
    .actions { flex-wrap: wrap; gap: 8px; }
    .actions button { min-height: var(--tap, 44px); }
  }
</style>
