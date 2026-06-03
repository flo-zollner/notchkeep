<script lang="ts">
  import { goto } from '$app/navigation';
  import Icon from '$lib/components/Icon.svelte';
  import InstitutionCard from '$lib/components/InstitutionCard.svelte';
  import InstitutionModal from '$lib/components/InstitutionModal.svelte';
  import Skeleton from '$lib/components/Skeleton.svelte';
  import { t } from '$lib/settings.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import { listInstitutionsWithSummary, type InstitutionSummary } from '$lib/api';

  let institutions = $state<InstitutionSummary[]>([]);
  let loading = $state(true);
  let showModal = $state(false);

  const ti = $derived((t() as Record<string, any>).institutions ?? {});

  $effect(() => {
    void reload();
  });

  async function reload() {
    loading = true;
    try {
      institutions = await listInstitutionsWithSummary();
    } finally {
      loading = false;
    }
  }

  function onSaved() {
    showModal = false;
    void reload();
  }
</script>

<header class="page-h">
  <h1>{ti.title ?? 'Institute'}</h1>
  <div class="actions">
    <button class="primary" type="button" onclick={() => (showModal = true)}>
      <Icon name="plus" size={14} /> {ti.add ?? 'Institut hinzufügen'}
    </button>
  </div>
</header>

{#if loading}
  <div class="grid">
    <Skeleton height={120} radius={12} />
    <Skeleton height={120} radius={12} />
    <Skeleton height={120} radius={12} />
  </div>
{:else if institutions.length === 0}
  <EmptyState
    icon="briefcase"
    title={ti.empty ?? 'Noch keine Institute'}
    description="Lege deine erste Bank/Broker an."
    actionLabel={ti.add ?? 'Institut anlegen'}
    onAction={() => (showModal = true)}
  />
{:else}
  <div class="grid">
    {#each institutions as inst (inst.id)}
      <InstitutionCard
        institution={inst}
        onClick={() => goto(`/institute/${inst.id}`)}
      />
    {/each}
  </div>
{/if}

{#if showModal}
  <InstitutionModal
    institution={null}
    onClose={() => (showModal = false)}
    {onSaved}
  />
{/if}

<style>
  .page-h {
    display: flex; align-items: center; gap: 12px; margin-bottom: 16px;
  }
  .page-h h1 { flex: 1; margin: 0; font-size: 22px; }
  .actions { display: flex; align-items: center; gap: 12px; }
  .primary { background: var(--accent); color: var(--accent-fg, white); border: 0; padding: 8px 12px; border-radius: 8px; cursor: pointer; display: inline-flex; align-items: center; gap: 8px; }
  .grid {
    /* 280px: minimum card width — wide enough for institution name + balance KPI side by side */
    display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }
  .empty { padding: 48px 0; text-align: center; color: var(--text-muted); }
  .empty button { margin-top: 14px; }

  @media (max-width: 599px) {
    .page-h { flex-wrap: wrap; gap: 8px; }
    .page-h h1 { font-size: 20px; }

    /* Institute cards grid → 1 column */
    .grid { grid-template-columns: 1fr; gap: 8px; }
  }
</style>
