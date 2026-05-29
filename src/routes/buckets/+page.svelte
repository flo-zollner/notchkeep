<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import BucketCard from '$lib/components/BucketCard.svelte';
  import BucketModal from '$lib/components/BucketModal.svelte';
  import { t } from '$lib/settings.svelte';
  import { api, type Bucket, type BucketProgress, type BucketRule, errMsg } from '$lib/api';

  let buckets = $state<Bucket[]>([]);
  let progressMap = $state<Map<number, BucketProgress>>(new Map());
  let securitiesValueMap = $state<Map<number, number>>(new Map());
  let trendsMap = $state<Map<number, number[]>>(new Map());
  let showArchived = $state(false);
  let loading = $state(true);

  let editing = $state<Bucket | null>(null);
  let addingNew = $state(false);

  const tb = $derived(t().buckets);

  $effect(() => {
    void reload(showArchived);
  });

  async function reload(includeArchived: boolean) {
    loading = true;
    try {
      const [bs, p] = await Promise.all([
        api.listBuckets(includeArchived),
        api.listBucketProgress(),
      ]);
      buckets = bs;
      progressMap = new Map(p.map((x) => [x.bucketId, x]));

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

  function totalFor(b: Bucket): number {
    const cash = progressMap.get(b.id)?.currentCents ?? 0;
    const sec = securitiesValueMap.get(b.id) ?? 0;
    return cash + sec;
  }

  function isFunded(b: Bucket): boolean {
    return b.targetCents !== null && b.targetCents > 0 && totalFor(b) >= b.targetCents;
  }

  const funded = $derived(buckets.filter(isFunded));
  const funding = $derived(buckets.filter((b) => !isFunded(b)));

  // ─── Auto-Allocation Rules ───
  let bucketRules = $state<BucketRule[]>([]);
  let editingRule = $state<BucketRule | null>(null);
  let ruleSaving = $state(false);
  let ruleError = $state<string | null>(null);
  let applyBusy = $state(false);
  let applyResult = $state<number | null>(null);

  function makeEmptyRule(): BucketRule {
    return {
      id: 0, priority: 100, name: '',
      counterpartyContains: null,
      minAmountCents: null,
      maxAmountCents: null,
      targetBucketId: buckets[0]?.id ?? 0,
      enabled: true,
    };
  }

  async function reloadRules() {
    try { bucketRules = await api.listBucketRules(); } catch {}
  }
  $effect(() => { void reloadRules(); });

  async function saveRule() {
    if (!editingRule) return;
    ruleSaving = true;
    ruleError = null;
    try {
      // Normalize empty strings to null
      const r = { ...editingRule };
      if (r.counterpartyContains === '') r.counterpartyContains = null;
      if (r.targetBucketId === 0) {
        ruleError = 'Ziel-Topf wählen';
        return;
      }
      if (r.id === 0) {
        const payload = {
          priority: r.priority, name: r.name,
          counterpartyContains: r.counterpartyContains,
          minAmountCents: r.minAmountCents,
          maxAmountCents: r.maxAmountCents,
          targetBucketId: r.targetBucketId,
          enabled: r.enabled,
        };
        await api.createBucketRule(payload);
      } else {
        await api.updateBucketRule(r);
      }
      await reloadRules();
      editingRule = null;
    } catch (e) {
      ruleError = errMsg(e);
    } finally {
      ruleSaving = false;
    }
  }

  async function deleteRuleConfirm(id: number) {
    if (!confirm('Really delete this rule?')) return;
    try {
      await api.deleteBucketRule(id);
      await reloadRules();
    } catch (e) { console.error(e); }
  }

  async function runApplyNow() {
    applyBusy = true;
    try {
      applyResult = await api.applyBucketRulesNow(30);
      // Reload buckets/progress to show the new assignments
      void reload(showArchived);
    } catch (e) { applyResult = -1; console.error(e); }
    finally { applyBusy = false; }
  }
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

<div class="card card-pad-lg" style="margin-top: 18px;">
  <div class="card-h">
    <h3>Auto-Allokations-Regeln</h3>
    <button class="primary" type="button" onclick={() => (editingRule = makeEmptyRule())}>
      + Regel
    </button>
  </div>
  <p class="muted">
    Weist eingehende Income-Tx (amount &gt; 0) ohne Topf automatisch einem Topf zu, basierend
    auf Counterparty-Substring und/oder Betrags-Range. First-match-wins nach priority ASC.
  </p>
  <button class="apply-now" type="button" onclick={runApplyNow} disabled={applyBusy}>
    {applyBusy ? 'Wendet an…' : 'Jetzt auf letzte 30 Tage anwenden'}
  </button>
  {#if applyResult !== null}
    <span class="muted" style="margin-left: 8px;">{applyResult} Tx zugeordnet</span>
  {/if}
  {#if bucketRules.length === 0}
    <p class="empty small">Noch keine Regeln. Lege eine an um Income-Tx automatisch zuzuordnen.</p>
  {:else}
    <ul class="rule-list">
      {#each bucketRules as r (r.id)}
        {@const tgt = buckets.find((b) => b.id === r.targetBucketId)}
        <li class="rule-row" class:disabled={!r.enabled}>
          <span class="rule-prio">{r.priority}</span>
          <span class="rule-name">{r.name}</span>
          <span class="rule-cond muted">
            {#if r.counterpartyContains}„{r.counterpartyContains}"{:else}—{/if}
            {#if r.minAmountCents !== null || r.maxAmountCents !== null}
              · {r.minAmountCents !== null ? `≥ ${(r.minAmountCents / 100).toFixed(0)} €` : ''}
              {r.maxAmountCents !== null ? `≤ ${(r.maxAmountCents / 100).toFixed(0)} €` : ''}
            {/if}
          </span>
          <span class="rule-target">→ {tgt?.name ?? '—'}</span>
          <button class="btn-icon" type="button" onclick={() => (editingRule = { ...r })} title="Bearbeiten">
            <Icon name="pencil" size={12} />
          </button>
          <button class="btn-icon danger" type="button" onclick={() => deleteRuleConfirm(r.id)} title="Löschen">
            <Icon name="x" size={12} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if editingRule}
  <div class="rule-modal-bg" role="presentation" onclick={() => (editingRule = null)}></div>
  <div class="rule-modal" role="dialog" aria-modal="true">
    <h3>{editingRule.id === 0 ? 'Neue Regel' : 'Regel bearbeiten'}</h3>
    <label>
      Name
      <input type="text" bind:value={editingRule.name} />
    </label>
    <label>
      Priority (kleiner = höher)
      <input type="number" bind:value={editingRule.priority} min="1" />
    </label>
    <label>
      Counterparty enthält (case-insensitive, leer = alle)
      <input type="text" bind:value={editingRule.counterpartyContains} placeholder="z.B. Arbeitgeber" />
    </label>
    <label>
      Min Betrag (Cent, leer = kein Min)
      <input type="number" bind:value={editingRule.minAmountCents} placeholder="z.B. 100000 für 1000 €" />
    </label>
    <label>
      Max Betrag (Cent)
      <input type="number" bind:value={editingRule.maxAmountCents} />
    </label>
    <label>
      Ziel-Topf
      <select bind:value={editingRule.targetBucketId}>
        <option value={0}>— wählen —</option>
        {#each buckets as b (b.id)}
          <option value={b.id}>{b.name}</option>
        {/each}
      </select>
    </label>
    <label class="checkbox-line">
      <input type="checkbox" bind:checked={editingRule.enabled} />
      Aktiv
    </label>
    {#if ruleError}<p class="error">{ruleError}</p>{/if}
    <div class="actions">
      <button class="btn ghost" type="button" onclick={() => (editingRule = null)}>Abbrechen</button>
      <button class="btn primary" type="button" onclick={saveRule} disabled={ruleSaving}>
        {ruleSaving ? 'Speichert…' : 'Speichern'}
      </button>
    </div>
  </div>
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

  /* ─── Auto-Allocation Rules ─── */
  .card {
    background: var(--surface); border: 1px solid var(--border); border-radius: 10px;
  }
  .card-pad-lg { padding: 16px 20px; }
  .card-h {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 8px;
  }
  .card-h h3 { margin: 0; font-size: 15px; }
  .apply-now {
    padding: 5px 12px;
    border-radius: 4px;
    border: 1px solid var(--accent);
    color: var(--accent);
    background: transparent;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .apply-now:disabled { opacity: 0.5; cursor: wait; }
  .rule-list {
    margin: 10px 0 0 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 4px;
  }
  .rule-row {
    display: grid;
    grid-template-columns: 32px 1fr 1.6fr 1fr auto auto;
    gap: 8px;
    align-items: center;
    padding: 5px 8px;
    background: var(--surface-2);
    border-radius: 4px;
    font-size: 12px;
  }
  .rule-row.disabled { opacity: 0.5; }
  .rule-prio {
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
    font-size: 11px;
    text-align: right;
  }
  .rule-name { font-weight: 500; }
  .rule-cond { font-size: 11px; }
  .rule-target { color: var(--accent); }
  .btn-icon {
    background: transparent; border: 1px solid var(--border); border-radius: 4px;
    padding: 3px 5px; cursor: pointer; color: var(--text-muted);
  }
  .btn-icon.danger:hover { color: var(--negative); border-color: var(--negative); }
  .rule-modal-bg {
    position: fixed; inset: 0; background: rgba(0,0,0,0.4); z-index: 50;
  }
  .rule-modal {
    position: fixed; left: 50%; top: 50%; transform: translate(-50%, -50%);
    background: var(--surface); padding: 20px; border-radius: 12px;
    z-index: 51; min-width: 360px; max-width: 480px;
    box-shadow: 0 12px 48px rgba(0,0,0,0.3);
    display: grid; gap: 10px;
  }
  .rule-modal h3 { margin: 0 0 4px 0; }
  .rule-modal label {
    display: grid; gap: 3px; font-size: 12px; color: var(--text-muted);
  }
  .rule-modal label.checkbox-line {
    grid-template-columns: auto 1fr;
    align-items: center; gap: 6px;
  }
  .rule-modal input, .rule-modal select {
    background: var(--surface-2); border: 1px solid var(--border);
    border-radius: 4px; padding: 5px 8px; font: inherit; color: var(--text);
  }
  .rule-modal .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px; }
  .rule-modal .actions .btn {
    padding: 6px 14px; border-radius: 4px; border: 1px solid var(--border);
    background: var(--surface-2); color: var(--text); cursor: pointer; font: inherit; font-size: 12px;
  }
  .rule-modal .actions .btn.primary { border-color: var(--accent); color: var(--accent); }
  .rule-modal .actions .btn.ghost { border: 0; background: transparent; color: var(--text-muted); }
  .rule-modal .error { color: var(--negative); font-size: 12px; }

  @media (max-width: 599px) {
    .page-h { flex-wrap: wrap; gap: 8px; }
    .page-h h1 { font-size: 20px; }

    /* Bucket cards grid → 1 column */
    .grid { grid-template-columns: 1fr; gap: 8px; }

    /* Rule rows: collapse 6-col table to stacked layout */
    .rule-row {
      grid-template-columns: 1fr auto auto;
      grid-template-rows: auto auto;
    }
    .rule-prio { grid-row: 1; grid-column: 1; }
    .rule-name { grid-row: 1; grid-column: 1; margin-left: 20px; }
    .rule-cond { grid-row: 2; grid-column: 1 / 3; font-size: 10px; }
    .rule-target { grid-row: 1; grid-column: 2; }

    /* Modal width on phone */
    .rule-modal { min-width: unset; width: calc(100vw - 32px); }
  }
</style>
