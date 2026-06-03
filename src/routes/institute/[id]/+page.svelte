<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import {
    getInstitution,
    listInstitutionsWithSummary,
    type Institution,
    type InstitutionSummary,
    type Account,
    api,
  } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import InstitutionModal from '$lib/components/InstitutionModal.svelte';
  import KPI from '$lib/components/KPI.svelte';
  import { settings, t } from '$lib/settings.svelte';

  const ti = $derived((t() as Record<string, any>).institutions ?? {});
  const tc = $derived(t().common);

  function kindLabel(k: string): string {
    const c = tc as unknown as Record<string, string | undefined>;
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

  const id = $derived(Number(page.params.id));

  let institution = $state<Institution | null>(null);
  let summary = $state<InstitutionSummary | null>(null);
  let accounts = $state<Account[]>([]);
  let editing = $state(false);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function refresh() {
    if (!Number.isFinite(id) || id <= 0) return;
    loading = true;
    error = null;
    try {
      const [inst, summaries, allAccounts] = await Promise.all([
        getInstitution(id),
        listInstitutionsWithSummary(),
        api.listAccounts(),
      ]);
      institution = inst;
      summary = summaries.find((s) => s.id === id) ?? null;
      accounts = allAccounts.filter((a) => a.institution_id === id);
    } catch (e: unknown) {
      error = typeof e === 'string' ? e : ((e as any)?.message ?? String(e));
      institution = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const _id = id;
    void _id;
    void refresh();
  });

  async function handleSaved(inst: Institution) {
    await refresh();
    editing = false;
    // If institution was deleted, refresh() will have set institution = null via error catch.
    // Navigate back to list.
    if (institution === null) {
      goto('/institute');
    }
  }
</script>

<button class="btn back" onclick={() => goto('/institute')}>
  <Icon name="chevron-right" size={12} /> {ti.title ?? 'Institute'}
</button>

<div class="card error-card" aria-live="polite">{#if error}<Icon name="warning" size={14} aria-hidden="true" /> Fehler: {error}{/if}</div>

{#if loading}
  <div class="empty">…</div>
{:else if institution}
  {@const bg = institution.color ?? 'var(--institution-default-color)'}

  <!-- Header card -->
  <div class="card card-pad-lg header">
    <div class="inst-logo" style:background={bg} style:color="var(--accent-fg)">
      {#if institution.icon}
        <Icon name={institution.icon} size={22} />
      {:else}
        {institution.name.slice(0, 2).toUpperCase()}
      {/if}
    </div>
    <div class="header-body">
      <h1>
        {institution.name}
        {#if institution.country}
          <span class="pill country">{institution.country.toUpperCase()}</span>
        {/if}
        {#if institution.archived}
          <span class="pill archived">{(tc as unknown as Record<string,string>).archived ?? 'Archiviert'}</span>
        {/if}
      </h1>
      {#if institution.bic}
        <div class="bic-block">
          <span class="label">{tc.bic ?? 'BIC'}</span>
          <button
            class="bic-copy"
            type="button"
            onclick={() => institution?.bic && navigator.clipboard.writeText(institution.bic)}
            title="BIC kopieren"
          >
            <span class="mono">{institution.bic}</span>
            <Icon name="card" size={13} />
          </button>
        </div>
      {/if}
      {#if institution.note}
        <div class="note">{institution.note}</div>
      {/if}
    </div>
    <div class="header-actions">
      <button
        class="btn btn-edit"
        type="button"
        onclick={() => (editing = true)}
      >
        <Icon name="pencil" size={14} /> {ti.edit ?? 'Bearbeiten'}
      </button>
    </div>
  </div>

  <!-- KPI row -->
  {#if summary}
    <div class="kpi-grid" style="grid-template-columns: repeat(2, 1fr); margin-top: 14px;">
      <KPI
        label={ti.accountCount?.(summary.accountCount) ?? `${summary.accountCount} Konten`}
        value={String(summary.accountCount)}
      />
      <KPI
        label={tc.balance ?? 'Saldo'}
        value={fmtEur(summary.balanceCents, { hide: settings.hide })}
      />
    </div>
  {/if}

  <!-- Accounts list -->
  {#if accounts.length > 0}
    <section class="card card-pad-lg" style="margin-top: 14px;">
      <div class="card-h">
        <h3>{tc.account ?? 'Konten'} ({accounts.length})</h3>
      </div>
      <ul class="account-list">
        {#each accounts as acc (acc.id)}
          <li>
            <a class="account-row" href="/accounts/{acc.id}">
              <span class="acc-icon" style:color={acc.color ?? bg}>
                {#if acc.icon}
                  <Icon name={acc.icon} size={14} />
                {:else}
                  <Icon name="wallet" size={14} />
                {/if}
              </span>
              <span class="acc-name">{acc.name}</span>
              <span class="acc-kind">{kindLabel(acc.kind)}</span>
              <span class="acc-arrow"><Icon name="chevron-right" size={12} /></span>
            </a>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
{:else if !error}
  <EmptyState icon="briefcase" title="Institut nicht gefunden" actionLabel="Zurück" onAction={() => goto('/institute')} />
{/if}

{#if editing && institution}
  <InstitutionModal
    {institution}
    onClose={() => (editing = false)}
    onSaved={handleSaved}
  />
{/if}

<style>
  .back {
    margin-bottom: 14px;
    transform: rotate(180deg);
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .error-card {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--negative);
    margin-bottom: 14px;
  }
  .error-card:empty { display: none; }
  .empty {
    padding: 24px;
    text-align: center;
    color: var(--text-faint);
    font-size: 13px;
  }

  /* Header */
  .header { display: flex; gap: 16px; align-items: flex-start; }
  .inst-logo {
    width: 56px;
    height: 56px;
    border-radius: 14px;
    display: grid;
    place-items: center;
    font-size: 16px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .header-body { flex: 1; min-width: 0; }
  .header-body h1 {
    margin: 0;
    font-size: 20px;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .header-actions { flex-shrink: 0; align-self: flex-start; }

  /* Pills */
  .pill {
    display: inline-block;
    font-size: 10.5px;
    font-weight: 600;
    padding: 4px 8px;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .pill.country {
    background: var(--surface-2);
    color: var(--text-muted);
  }
  .pill.archived {
    background: var(--surface-2);
    color: var(--text-muted);
  }

  /* BIC */
  .bic-block {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 8px;
    font-size: 13px;
  }
  .bic-block .label { color: var(--text-muted); }
  .bic-copy {
    background: transparent;
    border: 1px solid var(--c-border, var(--surface-2));
    cursor: pointer;
    color: var(--text);
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: var(--r-sm);
    font-size: 13px;
    transition: opacity 0.12s;
  }
  .bic-copy:hover { background: var(--surface-2); }
  .mono { font-family: var(--font-mono); letter-spacing: 0.04em; }

  /* Note */
  .note {
    font-size: 13px;
    color: var(--text-muted);
    margin-top: 8px;
  }

  /* Edit button */
  .btn-edit {
    background: var(--surface-2);
    border: 1px solid var(--c-border, transparent);
    color: var(--text);
    padding: 8px 12px;
    border-radius: 8px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    white-space: nowrap;
  }
  .btn-edit:hover { background: var(--surface-3, var(--surface-2)); }

  /* Account list */
  .account-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .account-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px 12px;
    border-radius: 8px;
    text-decoration: none;
    color: inherit;
    transition: opacity 0.1s;
  }
  .account-row:hover { background: var(--surface-2); }
  .acc-icon {
    display: inline-grid;
    place-items: center;
    width: 18px;
    flex-shrink: 0;
  }
  .acc-name {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .acc-kind {
    font-size: 12px;
    color: var(--text-muted);
  }
  .acc-arrow { color: var(--text-faint); }

  /* card-h shared helper (mirrors app-wide convention) */
  .card-h {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }
  .card-h h3 { margin: 0; font-size: 14px; flex: 1; }

  @media (prefers-reduced-motion: reduce) {
    .bic-copy,
    .account-row { transition: none; }
  }
</style>
