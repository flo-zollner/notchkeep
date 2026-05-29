<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import BucketCard from '$lib/components/BucketCard.svelte';
  import BucketModal from '$lib/components/BucketModal.svelte';
  import AllocateDialog from '$lib/components/AllocateDialog.svelte';
  import { settings, t } from '$lib/settings.svelte';
  import { api, type Bucket, type BucketProgress } from '$lib/api';
  import { fmtEur } from '$lib/format';

  let buckets = $state<Bucket[]>([]);
  let progressMap = $state<Map<number, BucketProgress>>(new Map());
  let securitiesValueMap = $state<Map<number, number>>(new Map());
  let trendsMap = $state<Map<number, number[]>>(new Map());
  let showArchived = $state(false);
  let loading = $state(true);
  let readyToAssign = $state(0);

  let editing = $state<Bucket | null>(null);
  let addingNew = $state(false);
  let allocateFor = $state<Bucket | null>(null);

  const tb = $derived(t().buckets);

  $effect(() => {
    void reload(showArchived);
  });

  async function reload(includeArchived: boolean) {
    loading = true;
    try {
      const [bs, p, rta] = await Promise.all([
        api.listBuckets(includeArchived),
        api.listBucketProgress(),
        api.readyToAssign(),
      ]);
      buckets = bs;
      progressMap = new Map(p.map((x) => [x.bucketId, x]));
      readyToAssign = rta;

      // Parallel-fetch the allocated securities totals per bucket.
      const holdingsArr = await Promise.all(
        bs.map(async (b) => {
          const rows = await api.bucketHoldings(b.id);
          const sum = rows.reduce((s, r) => s + r.valueCents, 0);
          return [b.id, sum] as const;
        })
      );
      securitiesValueMap = new Map(holdingsArr);

      // Trend sparkline: 6-month net cashflow per bucket.
      const now = new Date();
      const trendArr = await Promise.all(
        bs.map(async (b) => {
          const flow = await api.bucketMonthlyFlow(b.id, now.getFullYear(), now.getMonth() + 1, 6);
          const series = flow.map((f) => f.inCents - f.outCents);
          return [b.id, series] as const;
        })
      );
      trendsMap = new Map(trendArr);
    } finally {
      loading = false;
    }
  }

  function onSaved() {
    void reload(showArchived);
  }

  function cashFor(b: Bucket): number {
    return progressMap.get(b.id)?.currentCents ?? 0;
  }

  /** Funded status is based on cash saldo only (not including securities). */
  function isFunded(b: Bucket): boolean {
    return b.targetCents !== null && b.targetCents > 0 && cashFor(b) >= b.targetCents;
  }

  const funded = $derived(buckets.filter(isFunded));
  const funding = $derived(buckets.filter((b) => !isFunded(b)));

  const isUnderfunded = $derived(readyToAssign < 0);
</script>

<header class="page-h">
  <h1>{tb.title ?? 'Töpfe'}</h1>
  <div class="actions">
    <label class="toggle">
      <input type="checkbox" bind:checked={showArchived} />
      <span>{tb.showArchived}</span>
    </label>
    <button class="primary" type="button" onclick={() => (addingNew = true)}>
      <Icon name="plus" size={14} /> {tb.add}
    </button>
  </div>
</header>

<!-- Ready-to-Assign header card -->
<div class="rta-card" class:underfunded={isUnderfunded}>
  <div class="rta-row">
    <span class="rta-label">{tb.readyToAssign}</span>
    <span class="rta-value num" class:negative={isUnderfunded}>
      {fmtEur(readyToAssign, { hide: settings.hide, decimals: 2 })}
    </span>
  </div>
  {#if isUnderfunded}
    <div class="rta-warn">
      <strong>{tb.coverageWarnTitle}</strong>
      <span>
        {tb.coverageWarnBody.replace('{amount}', fmtEur(-readyToAssign, { decimals: 2 }))}
      </span>
    </div>
    <button
      type="button"
      class="rta-fix-btn"
      onclick={() => { allocateFor = buckets[0] ?? null; }}
    >
      {tb.coverageFix}
    </button>
  {/if}
</div>

{#if loading}
  <p class="muted">…</p>
{:else if buckets.length === 0}
  <div class="empty">
    <p>{tb.empty ?? 'Noch keine Töpfe angelegt.'}</p>
    <button class="primary" type="button" onclick={() => (addingNew = true)}>
      {tb.emptyCta ?? 'Ersten Topf anlegen'}
    </button>
  </div>
{:else}
  {#if funding.length > 0}
    <section class="section">
      <h2 class="section-h">{tb.sectionFunding ?? 'In Funding'} <span class="count">{funding.length}</span></h2>
      <div class="grid">
        {#each funding as b (b.id)}
          <BucketCard
            bucket={b}
            progress={progressMap.get(b.id)}
            securitiesValueCents={securitiesValueMap.get(b.id) ?? 0}
            trend={trendsMap.get(b.id)}
            onEdit={() => (editing = b)}
            onAssign={() => { allocateFor = b; }}
          />
        {/each}
      </div>
    </section>
  {/if}
  {#if funded.length > 0}
    <section class="section">
      <h2 class="section-h">{tb.sectionFunded ?? 'Funded'} <span class="count">{funded.length}</span></h2>
      <div class="grid">
        {#each funded as b (b.id)}
          <BucketCard
            bucket={b}
            progress={progressMap.get(b.id)}
            securitiesValueCents={securitiesValueMap.get(b.id) ?? 0}
            trend={trendsMap.get(b.id)}
            onEdit={() => (editing = b)}
            onAssign={() => { allocateFor = b; }}
          />
        {/each}
      </div>
    </section>
  {/if}
{/if}

{#if editing}
  <BucketModal
    bucket={editing}
    onClose={() => (editing = null)}
    onSaved={onSaved}
  />
{/if}
{#if addingNew}
  <BucketModal
    bucket={null}
    onClose={() => (addingNew = false)}
    onSaved={onSaved}
  />
{/if}
{#if allocateFor}
  <AllocateDialog
    bucket={allocateFor}
    {buckets}
    readyToAssignCents={readyToAssign}
    onSaved={() => { allocateFor = null; void reload(showArchived); }}
    onClose={() => { allocateFor = null; }}
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

  /* Ready-to-Assign card */
  .rta-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    margin-bottom: 20px;
  }
  .rta-card.underfunded {
    border-color: var(--negative);
    background: color-mix(in srgb, var(--negative) 6%, var(--surface-2));
  }
  .rta-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .rta-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .rta-value {
    font-size: 20px;
    font-weight: 600;
  }
  .rta-value.negative { color: var(--negative); }
  .rta-warn {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    color: var(--negative);
  }
  .rta-warn strong { font-weight: 600; }
  .rta-fix-btn {
    align-self: flex-start;
    padding: 5px 12px;
    border-radius: 6px;
    border: 1px solid var(--negative);
    color: var(--negative);
    background: transparent;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .rta-fix-btn:hover {
    background: color-mix(in srgb, var(--negative) 10%, transparent);
  }

  .grid {
    display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }
  .section { margin-bottom: 24px; }
  .section-h {
    font-size: 13px; font-weight: 500; color: var(--text-muted);
    text-transform: uppercase; letter-spacing: 0.04em;
    margin: 0 0 10px 0; display: flex; align-items: center; gap: 8px;
  }
  .section-h .count {
    font-size: 11px; font-weight: 400; padding: 1px 6px;
    background: var(--surface-2); border-radius: 999px; color: var(--text-muted);
    text-transform: none; letter-spacing: 0;
  }
  .empty { padding: 48px 0; text-align: center; color: var(--text-muted); }
  .empty button { margin-top: 14px; }
  .muted { color: var(--text-muted); }

  @media (max-width: 599px) {
    .page-h { flex-wrap: wrap; gap: 8px; }
    .page-h h1 { font-size: 20px; }
    .grid { grid-template-columns: 1fr; gap: 8px; }
  }
</style>
