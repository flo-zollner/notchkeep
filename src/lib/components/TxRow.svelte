<script lang="ts">
  import type { Account, Bucket, Category, Institution, Transaction } from '$lib/api';
  import { fmtDate, fmtEur } from '$lib/api';
  import Icon from './Icon.svelte';
  import { t } from '$lib/settings.svelte';

  interface Props {
    tx: Transaction;
    accounts: Account[];
    categories: Category[];
    lang: 'de' | 'en';
    hide: boolean;
    hideAccount?: boolean;
    bucketsById?: Map<number, Bucket>;
    institutionsById?: Map<number, Institution>;
    hideInstitution?: boolean;
    onclick?: () => void;
    /** Aus welcher Account-Perspektive wird die Tx betrachtet.
     *  Bei Depot-View (viewAccountId === tx.holding_account_id) wird das
     *  Vorzeichen geflippt: Buy = Zufluss (+), Sell = Abfluss (−). */
    viewAccountId?: number;
  }

  let { tx, accounts, categories, lang, hide, hideAccount = false, bucketsById, institutionsById, hideInstitution = false, onclick, viewAccountId }: Props = $props();

  // Vorzeichen aus Depot-Sicht flippen (Doppelbuchung): wenn wir das Depot
  // betrachten und die Tx eigentlich am Cash-Konto hängt, ist ein Kauf für
  // das Depot ein Zufluss (positiv), ein Verkauf ein Abfluss (negativ).
  const isDepotView = $derived(
    viewAccountId != null
      && tx.holding_account_id === viewAccountId
      && tx.account_id !== viewAccountId
  );
  const displayedAmount = $derived(isDepotView ? -tx.amount_cents : tx.amount_cents);

  const bucket = $derived(tx.bucket_id != null ? bucketsById?.get(tx.bucket_id) : undefined);

  const tradeKinds = ['buy', 'sell', 'dividend', 'corporate_action', 'tax', 'fee'] as const;
  const isTrade = $derived(
    (tradeKinds as readonly string[]).includes(tx.kind),
  );
  const tradeKindLabel = $derived(
    isTrade ? (t() as any).txKind?.[tx.kind] ?? tx.kind : null,
  );
  const isTransfer = $derived(tx.kind === 'transfer');
  const isAutoPair = $derived(tx.source === 'auto_pair');

  const cat = $derived(tx.category_id ? categories.find((c) => c.id === tx.category_id) : undefined);
  const acc = $derived(accounts.find((a) => a.id === tx.account_id));
  const institution = $derived(
    !hideInstitution && institutionsById && acc?.institution_id != null
      ? institutionsById.get(acc.institution_id)
      : undefined
  );
  const positive = $derived(displayedAmount > 0);
  const iconName = $derived(cat?.icon || (positive ? 'arrow-down' : 'tag'));
  const color = $derived(cat?.color || 'var(--text-muted)');
  const catLabel = $derived(
    cat?.name ?? (lang === 'de' ? 'Unkategorisiert' : 'Uncategorized')
  );
  const description = $derived(tx.manual_note?.trim() || tx.purpose?.trim() || '');
  // Bei Trade-Tx blenden wir description aus dem subtitle aus — purpose
  // (= Security-Name) ist schon im title; doppelte Anzeige vermeiden.
  // Wenn der User aber manuell eine Notiz hinzugefügt hat (manual_note),
  // dann ist das informativ und bleibt sichtbar.
  const subtitleDescription = $derived(
    isTrade
      ? (tx.manual_note?.trim() ?? '')
      : description
  );
  const subtitle = $derived(
    hideAccount
      ? `${catLabel}${subtitleDescription ? ' · ' + subtitleDescription : ''}`
      : `${catLabel} · ${acc?.name ?? '—'}${subtitleDescription ? ' · ' + subtitleDescription : ''}`
  );
  // Bei Trade-Tx (Buy/Sell/Dividend/Corp-Action) wird der Titel zu
  // "<Kind> von <Security-Name>" statt dem Broker-Counterparty — die
  // Wertpapier-Aktion ist informativer als "flatexDEGIRO".
  // Fusion-Out/Fusion-In (beide kind='corporate_action') bekommen einen
  // expliziten Richtungs-Titel, weil sonst zwei nahezu identische Einträge
  // in der Liste stehen.
  const title = $derived.by(() => {
    const connector = t().common.tradeTitleConnector;
    // purpose hat bei Fusion ein "Fusion: "-Präfix vom Parser — strippen
    // damit das nicht doppelt steht ("Fusion-Ausbuchung von Fusion: X").
    const cleanedPurpose = tx.purpose?.replace(/^Fusion:\s*/, '') ?? '';
    if (tx.trade_side === 'fusion_out' && cleanedPurpose) {
      return `${t().common.tradeFusionOut} ${connector} ${cleanedPurpose}`;
    }
    if (tx.trade_side === 'fusion_in' && cleanedPurpose) {
      return `${t().common.tradeFusionIn} ${connector} ${cleanedPurpose}`;
    }
    if (isTrade && tx.purpose) {
      return `${tradeKindLabel} ${connector} ${tx.purpose}`;
    }
    return tx.counterparty || description || `#${tx.id}`;
  });
</script>

{#if onclick}
  <button class="tx-row" type="button" {onclick}>
    <span class="tx-icon" style:color>
      <Icon name={iconName} size={14} />
    </span>
    <span class="meta-wrap">
      <span class="tx-name">{title}</span>
      <span class="tx-meta"
        >{subtitle}{#if tx.counterparty_iban}<span class="iban-inline mono"
            > · {tx.counterparty_iban}</span
          >{/if}</span
      >
    </span>
    {#if bucket}
      <span class="bucket-badge" style:--accent={bucket.color ?? 'var(--c1)'}>{bucket.name}</span>
    {/if}
    {#if institution}
      <span class="bank-badge" style:--bank-color={institution.color ?? 'var(--text-faint)'}>{institution.name}</span>
    {/if}
    {#if isTransfer}
      <span class="trade-kind-badge transfer-badge" class:auto-pair={isAutoPair}>
        ↔ Transfer{#if isAutoPair} (Auto){/if}
      </span>
    {:else if isTrade}
      <span class="trade-kind-badge trade-kind-{tx.kind}">
        {tradeKindLabel}
      </span>
    {/if}
    <span class="tx-cat right">
      <span class="mono date">{fmtDate(tx.booking_date, lang)}</span>
    </span>
    <span class="tx-amt right" class:up={positive}>
      {fmtEur(displayedAmount, { hide, signed: positive })}
    </span>
  </button>
{:else}
  <div class="tx-row">
    <span class="tx-icon" style:color>
      <Icon name={iconName} size={14} />
    </span>
    <span class="meta-wrap">
      <span class="tx-name">{title}</span>
      <span class="tx-meta"
        >{subtitle}{#if tx.counterparty_iban}<span class="iban-inline mono"
            > · {tx.counterparty_iban}</span
          >{/if}</span
      >
    </span>
    {#if bucket}
      <span class="bucket-badge" style:--accent={bucket.color ?? 'var(--c1)'}>{bucket.name}</span>
    {/if}
    {#if institution}
      <span class="bank-badge" style:--bank-color={institution.color ?? 'var(--text-faint)'}>{institution.name}</span>
    {/if}
    {#if isTransfer}
      <span class="trade-kind-badge transfer-badge" class:auto-pair={isAutoPair}>
        ↔ Transfer{#if isAutoPair} (Auto){/if}
      </span>
    {:else if isTrade}
      <span class="trade-kind-badge trade-kind-{tx.kind}">
        {tradeKindLabel}
      </span>
    {/if}
    <span class="tx-cat right">
      <span class="mono date">{fmtDate(tx.booking_date, lang)}</span>
    </span>
    <span class="tx-amt right" class:up={positive}>
      {fmtEur(displayedAmount, { hide, signed: positive })}
    </span>
  </div>
{/if}

<style>
  .meta-wrap {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .meta-wrap :global(.tx-name) {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .right {
    text-align: right;
  }
  .date {
    font-size: 11px;
    color: var(--text-faint);
  }
  button.tx-row {
    cursor: pointer;
    font: inherit;
    color: inherit;
  }
  .iban-inline {
    opacity: 0.85;
    letter-spacing: 0.01em;
  }
  .bucket-badge {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--accent);
    color: white;
    opacity: 0.8;
    margin-right: 8px;
    white-space: nowrap;
  }
  .bank-badge {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--bank-color);
    color: var(--bank-color);
    margin-right: 8px;
    white-space: nowrap;
    opacity: 0.85;
  }
  .trade-kind-badge {
    font-size: 10px;
    font-weight: 500;
    padding: 2px 7px;
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--text);
    margin-left: 4px;
    letter-spacing: 0.01em;
  }
  .trade-kind-buy             { background: var(--positive-soft); color: var(--positive); }
  .trade-kind-sell            { background: var(--negative-soft); color: var(--negative); }
  .trade-kind-dividend        { background: var(--info-soft);     color: var(--info); }
  .trade-kind-corporate_action { background: var(--warning-soft);  color: var(--warning); }
  .transfer-badge             { background: var(--surface-3, var(--surface-2)); color: var(--text-muted); }
  .trade-kind-badge.auto-pair {
    background: color-mix(in srgb, var(--text-muted) 18%, transparent);
    color: var(--text-muted);
    font-style: italic;
  }
</style>
