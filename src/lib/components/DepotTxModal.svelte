<script lang="ts">
  import { api, type Account, type Bucket, type Category, type SecurityTrade, type Security, type Transaction, type UpdateTradePayload, errMsg } from '$lib/api';
  import { fmtEur, fmtNumInput, fmtEurInput, parseEur } from '$lib/format';
  import Icon from './Icon.svelte';
  import SecurityPicker from './SecurityPicker.svelte';
  import { settings, t } from '$lib/settings.svelte';
  import { snackbar } from '$lib/snackbar.svelte';
  import Sheet from './Sheet.svelte';

  interface Props {
    tx: Transaction;
    accounts: Account[];
    categories: Category[];
    bucketsById?: Map<number, Bucket>;
    onClose: () => void;
    onSaved: (tx: Transaction) => void;
    onDeleted?: (id: number) => void;
  }

  let { tx, accounts, categories, bucketsById, onClose, onSaved, onDeleted }: Props = $props();

  // ── Initial load: associated trade row + security ─────────────────────────
  let trade = $state<SecurityTrade | null>(null);
  let security = $state<Security | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let error = $state<string | null>(null);
  const tc = $derived(t().common);

  $effect(() => {
    loading = true;
    error = null;
    api.getTrade(tx.id)
      .then(async (tw) => {
        trade = tw.trade;
        security = await api.getSecurity(tw.trade.securityId);
      })
      .catch((e) => { error = errMsg(e); })
      .finally(() => { loading = false; });
  });

  // ── Editable fields ───────────────────────────────────────────────────────
  let sharesStr = $state('');
  let priceStr = $state('');
  let amountStr = $state('');
  let feeStr = $state('');
  let kestStr = $state('');
  let whtStr = $state('');
  let cashAccountId = $state<number | null>(null);
  let depotAccountId = $state<number | null>(null);

  $effect(() => {
    if (!trade) return;
    sharesStr = fmtNumInput(trade.sharesMicro / 1_000_000);
    priceStr = trade.unitPriceMicro != null
      ? fmtNumInput(trade.unitPriceMicro / 1_000_000)
      : '';
    amountStr = fmtEurInput(tx.amount_cents);
    feeStr = fmtEurInput(trade.feeCents);
    kestStr = fmtEurInput(trade.kestCents);
    whtStr = fmtEurInput(trade.withholdingTaxCents);
    cashAccountId = tx.account_id;
    depotAccountId = trade.accountId;
  });

  // ── Read-only context display ─────────────────────────────────────────────
  const cashAccount = $derived(accounts.find((a) => a.id === tx.account_id));
  const category = $derived(
    tx.category_id != null ? categories.find((c) => c.id === tx.category_id) : undefined
  );
  const bucket = $derived(
    tx.bucket_id != null ? bucketsById?.get(tx.bucket_id) : undefined
  );

  // Trade side controls which fields are editable/locked
  const tradeSide = $derived((trade?.side ?? 'buy') as string);
  const isDividend = $derived(tradeSide === 'dividend');
  const isFusion = $derived(tradeSide === 'fusion_out' || tradeSide === 'fusion_in');
  const isCorpAction = $derived(tradeSide === 'corporate_action');
  const sharesEditable = $derived(!isDividend);
  const priceEditable = $derived(!isDividend && !isFusion && !isCorpAction);
  const amountEditable = $derived(!isFusion && !isCorpAction);

  // ── Save ──────────────────────────────────────────────────────────────────
  function parseNum(s: string, unit: number): number | null {
    const n = parseEur(s);
    if (!Number.isFinite(n)) return null;
    return Math.round(n * unit);
  }

  async function save() {
    if (!trade) return;
    error = null;
    saving = true;
    try {
      const payload: UpdateTradePayload = {};

      const sharesNum = parseNum(sharesStr, 1_000_000);
      if (sharesNum != null && sharesNum !== trade.sharesMicro) {
        let signed = Math.abs(sharesNum);
        if (tradeSide === 'sell' || tradeSide === 'fusion_out') signed = -signed;
        payload.sharesMicro = signed;
      }
      const priceNum = parseNum(priceStr, 1_000_000);
      if (priceEditable && priceNum !== trade.unitPriceMicro) {
        payload.unitPriceMicro = priceNum;
      }
      const amountNum = parseNum(amountStr, 100);
      if (amountEditable && amountNum != null && amountNum !== tx.amount_cents) {
        payload.amountCents = amountNum;
      }
      const feeNum = parseNum(feeStr, 100);
      if (feeNum != null && feeNum !== trade.feeCents) {
        payload.feeCents = feeNum;
      }
      const kestNum = parseNum(kestStr, 100);
      if (kestNum != null && kestNum !== trade.kestCents) {
        payload.kestCents = kestNum;
      }
      const whtNum = parseNum(whtStr, 100);
      if (whtNum != null && whtNum !== trade.withholdingTaxCents) {
        payload.withholdingTaxCents = whtNum;
      }
      if (security && security.id !== trade.securityId) {
        payload.securityId = security.id;
      }
      if (cashAccountId != null && cashAccountId !== tx.account_id) {
        payload.txAccountId = cashAccountId;
      }
      if (depotAccountId !== trade.accountId) {
        payload.accountId = depotAccountId ?? undefined;
      }

      await api.updateTrade(tx.id, payload);

      const updated: Transaction = {
        ...tx,
        amount_cents: payload.amountCents ?? tx.amount_cents,
        account_id: payload.txAccountId ?? tx.account_id,
        holding_account_id: payload.accountId ?? tx.holding_account_id,
      };
      onSaved(updated);
      onClose();
    } catch (e) {
      error = errMsg(e);
    } finally {
      saving = false;
    }
  }

  async function deleteTx() {
    const id = tx.id;
    saving = true;
    error = null;
    try {
      await api.deleteTrade(id);
      onDeleted?.(id);
      onClose();
      snackbar.showUndo(tc.deleted, tc.undo, async () => {
        await api.restoreTransaction(id);
        onDeleted?.(id);
      });
    } catch (e) {
      error = errMsg(e);
      saving = false;
    }
  }
</script>

<Sheet open={true} onClose={onClose} title={security?.name ?? 'Depot-Transaktion'}>
  {#snippet footer()}
    <div class="footer-actions">
      {#if onDeleted}
        <button type="button" class="btn danger" onclick={deleteTx} disabled={saving}>
          🗑 Löschen
        </button>
      {/if}
      <button type="button" class="btn" onclick={onClose} disabled={saving}>Abbrechen</button>
      <button type="button" class="btn accent" onclick={save} disabled={saving}>
        {saving ? 'Speichert …' : 'Speichern'}
      </button>
    </div>
  {/snippet}

    {#if loading}
      <div class="empty">Lädt …</div>
    {:else if error}
      <div class="card err">{error}</div>
    {:else}
      <div class="info-block">
        <div class="info-row">
          <span class="lbl">{t().trade.bookingDate ?? 'Datum'}</span>
          <span>{tx.booking_date}</span>
          {#if tx.counterparty}
            <span class="lbl">{t().trade.counterparty ?? 'Gegenseite'}</span>
            <span>{tx.counterparty}</span>
          {/if}
        </div>
        {#if tx.manual_note || tx.purpose}
          <div class="info-row">
            <span class="lbl">{t().trade.note ?? 'Notiz'}</span>
            <span>{tx.manual_note || tx.purpose}</span>
          </div>
        {/if}
        {#if category || bucket}
          <div class="info-row">
            {#if category}
              <span class="lbl">{t().common.categories ?? 'Kategorie'}</span>
              <span>{category.name}</span>
            {/if}
            {#if bucket}
              <span class="lbl">{t().nav.buckets ?? 'Topf'}</span>
              <span>{bucket.name}</span>
            {/if}
          </div>
        {/if}
        <div class="info-row">
          <span class="lbl">{t().trade.side ?? 'Side'}</span>
          <span class="badge side-{tradeSide}">{tradeSide}</span>
        </div>
      </div>

      {#if isFusion}
        <div class="banner">
          ⚠ Fusion — die paired {tradeSide === 'fusion_out' ? 'Einbuchung' : 'Ausbuchung'} muss separat editiert werden.
        </div>
      {/if}

      <div class="grid">
        <label class="span2">
          Wertpapier
          <SecurityPicker value={security} onSelect={(s) => (security = s)} />
        </label>

        {#if sharesEditable}
          <label>Stück<input type="text" inputmode="decimal" bind:value={sharesStr} /></label>
        {/if}
        {#if priceEditable}
          <label>Preis €/Stück<input type="text" inputmode="decimal" bind:value={priceStr} /></label>
        {/if}
        {#if amountEditable}
          <label class:span2={isDividend}>Cash €<input type="text" inputmode="decimal" bind:value={amountStr} /></label>
        {/if}
        {#if !isFusion}
          <label>Gebühr €<input type="text" inputmode="decimal" bind:value={feeStr} /></label>
          <label>KESt €<input type="text" inputmode="decimal" bind:value={kestStr} /></label>
          <label>Quellenst. €<input type="text" inputmode="decimal" bind:value={whtStr} /></label>
        {/if}

        <label>
          Cash-Konto
          <select bind:value={cashAccountId}>
            {#each accounts.filter((a) => a.kind !== 'broker') as a}
              <option value={a.id}>{a.name}</option>
            {/each}
          </select>
        </label>
        <label>
          Depot
          <select bind:value={depotAccountId}>
            <option value={null}>—</option>
            {#each accounts.filter((a) => a.kind === 'broker') as a}
              <option value={a.id}>{a.name}</option>
            {/each}
          </select>
        </label>
      </div>

    {/if}
</Sheet>

<style>
  .empty { padding: 24px; text-align: center; color: var(--text-faint); }
  .err { color: var(--negative); padding: 12px; }

  .info-block {
    background: var(--surface-2);
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 16px;
    font-size: 13px;
  }
  .info-row { display: flex; flex-wrap: wrap; gap: 6px 14px; margin-bottom: 4px; }
  .info-row:last-child { margin-bottom: 0; }
  .lbl { color: var(--text-faint); }
  .badge {
    display: inline-block; padding: 1px 7px; border-radius: 999px;
    font-size: 11px; background: var(--surface-3, var(--surface-2));
  }
  .side-buy { background: var(--positive-soft); color: var(--positive); }
  .side-sell { background: var(--negative-soft); color: var(--negative); }
  .side-dividend { background: var(--info-soft); color: var(--info); }
  .side-corporate_action { background: var(--warning-soft); color: var(--warning); }
  .side-tax { background: var(--warning-soft); color: var(--warning); }
  .side-fusion_out, .side-fusion_in { background: var(--surface-3, var(--surface-2)); color: var(--text-muted); }

  .banner {
    background: var(--warning-soft);
    color: var(--warning);
    padding: 10px 12px;
    border-radius: 6px;
    margin-bottom: 14px;
    font-size: 13px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  @media (max-width: 599px) {
    .grid { grid-template-columns: 1fr; }
    .grid label.span2 { grid-column: span 1; }
  }
  .grid label {
    display: flex; flex-direction: column; gap: 4px;
    font-size: 12px; color: var(--text-muted);
  }
  .grid label.span2 { grid-column: span 2; }
  .grid input, .grid select {
    padding: 6px 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text);
  }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: nowrap;
  }
  .footer-actions .btn.danger { margin-right: auto; background: var(--negative-soft); color: var(--negative); }
  @media (max-width: 600px) {
    .footer-actions button { flex: 1 1 0; min-width: 0; }
    .footer-actions .btn.danger { flex: 0 0 auto; margin-right: auto; }
  }
  .btn {
    padding: 8px 16px;
    border-radius: 6px;
    border: 0;
    cursor: pointer;
    font-size: 14px;
  }
  .btn.accent { background: var(--accent, var(--positive)); color: white; border: 0; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
