<script lang="ts">
  import type { Account, Bucket, Category, Institution, Transaction } from '$lib/api';
  import { fmtDate } from '$lib/api';
  import { fmtEur } from '$lib/format';
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
    /** From which account perspective the transaction is viewed.
     *  In depot view (viewAccountId === tx.holding_account_id) the sign is
     *  flipped: Buy = inflow (+), Sell = outflow (−). */
    viewAccountId?: number;
  }

  let { tx, accounts, categories, lang, hide, hideAccount = false, bucketsById, institutionsById, hideInstitution = false, onclick, viewAccountId }: Props = $props();

  // Flip sign for depot view (double-entry): when viewing the depot and the
  // transaction is actually on the cash account, a buy is an inflow (positive)
  // and a sell is an outflow (negative).
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
  // For trade transactions we omit description from the subtitle — purpose
  // (= security name) is already in the title; avoid duplicate display.
  // However, if the user has added a manual note (manual_note), that is
  // informative and stays visible.
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
  // For trade transactions (buy/sell/dividend/corp-action) the title becomes
  // "<kind> of <security name>" instead of the broker counterparty — the
  // securities action is more informative than "flatexDEGIRO".
  // Fusion-out/fusion-in (both kind='corporate_action') get an explicit
  // directional title to avoid two nearly identical entries in the list.
  const title = $derived.by(() => {
    const connector = t().common.tradeTitleConnector;
    // purpose has a "Fusion: " prefix from the parser — strip it
    // so it doesn't appear doubled ("Fusion-debit of Fusion: X").
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
      <span aria-hidden={hide ? 'true' : undefined}>{fmtEur(displayedAmount, { hide, signed: positive })}</span>
      {#if hide}<span class="sr-only">{lang === 'de' ? 'verborgen' : 'hidden'}</span>{/if}
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
      <span aria-hidden={hide ? 'true' : undefined}>{fmtEur(displayedAmount, { hide, signed: positive })}</span>
      {#if hide}<span class="sr-only">{lang === 'de' ? 'verborgen' : 'hidden'}</span>{/if}
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
    padding: 4px 8px;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-fg);
    opacity: 0.8;
    margin-right: 8px;
    white-space: nowrap;
  }
  .bank-badge {
    font-size: 10px;
    padding: 4px 8px;
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
    padding: 4px 8px;
    border-radius: var(--r-sm);
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
  @media (max-width: 600px) {
    .bank-badge {
      display: none;
    }
  }
</style>
