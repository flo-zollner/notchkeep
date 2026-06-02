<script lang="ts">
  import Icon from './Icon.svelte';
  import SecurityPicker from './SecurityPicker.svelte';
  import Sheet from './Sheet.svelte';
  import DateField from './DateField.svelte';
  import { t } from '$lib/settings.svelte';
  import {
    api,
    listInstitutions,
    type Account,
    type Institution,
    type Security,
    type SecurityTrade,
    type TradeSide,
    type NewTradePayload,
  } from '$lib/api';
  import { fmtEur, parseEur } from '$lib/format';

  interface Props {
    onClose: () => void;
    onSaved: () => void;
    trade?: SecurityTrade;
  }
  let { onClose, onSaved, trade }: Props = $props();

  const tt = $derived(t().trade);

  const SIDES: TradeSide[] = ['buy', 'sell', 'dividend', 'corporate_action', 'tax'];

  let accounts = $state<Account[]>([]);
  let security = $state<Security | null>(null);
  let cashAccountId = $state<number | null>(null);
  let depotAccountId = $state<number | null>(null);
  let side = $state<TradeSide>('buy');
  const isTaxSide = $derived(side === 'tax');
  let bookingDate = $state(new Date().toISOString().slice(0, 10));
  let sharesStr = $state('');
  let priceStr = $state('');
  let feeStr = $state('0');
  let taxStr = $state('0');
  let amountStr = $state('');
  let counterparty = $state('');
  let note = $state('');
  let saving = $state(false);
  let error = $state<string | null>(null);

  // Account lists per kind, filtered to active only
  const cashAccounts = $derived(accounts.filter((a) => a.kind !== 'broker'));
  const depotAccounts = $derived(accounts.filter((a) => a.kind === 'broker'));

  // Derive institutions for counterparty default
  let institutions = $state<Institution[]>([]);
  const institutionsById = $derived(new Map(institutions.map((i) => [i.id, i])));

  $effect(() => {
    void loadAccounts();
    void listInstitutions(false).then((list) => { institutions = list; });
  });

  // Smart default for counterparty: if the depot has an institution, use its name.
  // Syncs automatically on depot change — if the user types a custom value and THEN
  // switches the depot, the custom value is intentionally overwritten (switch = explicit intent).
  $effect(() => {
    const depot = accounts.find((a) => a.id === depotAccountId);
    if (!depot?.institution_id) return;
    const inst = institutionsById.get(depot.institution_id);
    if (inst?.name) counterparty = inst.name;
  });

  async function loadAccounts() {
    accounts = (await api.listAccounts()).filter((a) => !a.archived);
    // Defaults: one cash + one depot from the same institution, if unambiguous.
    const broker = accounts.find((a) => a.kind === 'broker');
    if (broker) {
      depotAccountId = broker.id;
      // Cash account in the same institution (if unambiguous)
      const siblings = accounts.filter(
        (a) => a.kind !== 'broker' && a.institution_id === broker.institution_id,
      );
      if (siblings.length === 1) cashAccountId = siblings[0].id;
    }
    if (cashAccountId == null && cashAccounts.length === 1) {
      cashAccountId = cashAccounts[0].id;
    }
  }

  function parseNum(s: string, unit = 1): number | null {
    const n = parseEur(s);
    if (!Number.isFinite(n)) return null;
    return Math.round(n * unit);
  }

  async function save() {
    error = null;
    if (cashAccountId == null) { error = tt.errAccountRequired; return; }
    if (!security) { error = tt.errSecurityRequired; return; }
    const amountNum = parseNum(amountStr, 100);
    if (amountNum == null) { error = tt.errSharesNonZero; return; }
    if (!/^\d{4}-\d{2}-\d{2}$/.test(bookingDate)) { error = tt.errDateInvalid; return; }

    if (isTaxSide) {
      // Securities tax: no shares/price. Cash amount = tax charge.
      const feeNum = parseNum(feeStr, 100) ?? 0;
      saving = true;
      try {
        const payload: NewTradePayload = {
          accountId: cashAccountId,
          securityId: security.id,
          bookingDate,
          side: 'tax',
          sharesMicro: 0,
          unitPriceMicro: null,
          feeCents: feeNum,
          taxCents: Math.abs(amountNum),
          fxRateMicro: null,
          amountCents: amountNum,
          currency: null,
          counterparty: counterparty.trim() || null,
          manualNote: note.trim() || null,
          holdingAccountId: depotAccountId,
        };
        await api.createTrade(payload);
        onSaved();
        onClose();
      } catch (e) {
        error = (e as Error).message ?? String(e);
      } finally {
        saving = false;
      }
      return;
    }

    const sharesNum = parseNum(sharesStr, 1_000_000);
    if (sharesNum == null) { error = tt.errSharesNonZero; return; }
    if ((side === 'buy' || side === 'sell') && sharesNum === 0) {
      error = tt.errSharesNonZero; return;
    }
    const priceNum = parseNum(priceStr, 1_000_000);
    if ((side === 'buy' || side === 'sell') && priceNum == null) {
      error = tt.errUnitPriceRequired; return;
    }
    const feeNum = parseNum(feeStr, 100) ?? 0;
    const taxNum = parseNum(taxStr, 100) ?? 0;

    let sharesSigned = sharesNum;
    if (side === 'sell' && sharesSigned > 0) sharesSigned = -sharesSigned;
    if (side === 'buy' && sharesSigned < 0) sharesSigned = -sharesSigned;

    saving = true;
    try {
      const payload: NewTradePayload = {
        accountId: cashAccountId,
        securityId: security.id,
        bookingDate,
        side,
        sharesMicro: sharesSigned,
        unitPriceMicro: priceNum,
        feeCents: feeNum,
        taxCents: taxNum,
        fxRateMicro: null,
        amountCents: amountNum,
        currency: null,
        counterparty: counterparty.trim() || null,
        manualNote: note.trim() || null,
        holdingAccountId: depotAccountId,
      };
      await api.createTrade(payload);
      onSaved();
      onClose();
    } catch (e) {
      error = (e as Error).message ?? String(e);
    } finally {
      saving = false;
    }
  }
</script>

<Sheet open={true} {onClose} title={tt.titleNew}>
  {#snippet footer()}
    <div class="footer-actions">
      <button type="button" class="btn" onclick={onClose}>{tt.cancel}</button>
      <button type="button" class="btn primary" disabled={saving} onclick={save}>{tt.save}</button>
    </div>
  {/snippet}

  <div class="grid">
    <label class="span2">
      {tt.security}
      <SecurityPicker value={security} onSelect={(s) => (security = s)} />
    </label>
    <label>
      {tt.side}
      <select bind:value={side}>
        {#each SIDES as s}
          <option value={s}>{tt.sides[s]}</option>
        {/each}
      </select>
    </label>
    <label>
      Cash-Konto
      <select bind:value={cashAccountId}>
        <option value={null}>—</option>
        {#each cashAccounts as a}
          <option value={a.id}>{a.name}</option>
        {/each}
      </select>
    </label>
    <label>
      Depot
      <select bind:value={depotAccountId}>
        <option value={null}>—</option>
        {#each depotAccounts as a}
          <option value={a.id}>{a.name}</option>
        {/each}
      </select>
    </label>
    <label>{tt.bookingDate}<DateField bind:value={bookingDate} /></label>
    {#if !isTaxSide}
      <label>{tt.shares}<input type="text" inputmode="decimal" bind:value={sharesStr} /></label>
      <label>{tt.unitPrice}<input type="text" inputmode="decimal" bind:value={priceStr} /></label>
    {/if}
    <label>{tt.fee}<input type="text" inputmode="decimal" bind:value={feeStr} /></label>
    {#if !isTaxSide}
      <label>{tt.tax}<input type="text" inputmode="decimal" bind:value={taxStr} /></label>
    {/if}
    {#if trade}
      <label>
        {t().common.kest}
        <input type="text" value={fmtEur(trade.kestCents / 100, { hide: false, decimals: 2 })} readonly />
      </label>
      <label>
        {t().common.withholdingTax}
        <input type="text" value={fmtEur(trade.withholdingTaxCents / 100, { hide: false, decimals: 2 })} readonly />
      </label>
    {/if}
    <label class="span2">
      Cash €
      <input type="text" inputmode="decimal" bind:value={amountStr} placeholder="±EUR" />
      <small class="hint">{tt.amountHint}</small>
    </label>
    <label class="span2">{tt.counterparty}<input type="text" bind:value={counterparty} /></label>
    <label class="span2">{tt.note}<textarea rows="2" bind:value={note}></textarea></label>
  </div>

  {#if error}<p class="err">{error}</p>{/if}
</Sheet>

<style>
  .grid {
    display: grid; grid-template-columns: 1fr 1fr; gap: 10px 12px;
  }
  @media (max-width: 599px) {
    .grid { grid-template-columns: 1fr; }
  }
  .grid label { display: flex; flex-direction: column; font-size: 12px; gap: 4px; color: var(--text-muted); }
  .grid input, .grid select, .grid textarea {
    padding: 8px 10px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--surface-2); color: var(--text);
    font: inherit;
  }
  .grid input:focus, .grid select:focus, .grid textarea:focus {
    outline: none; border-color: var(--accent);
  }
  .grid .span2 { grid-column: 1 / -1; }
  .grid .hint { color: var(--text-faint); font-size: 11px; }
  .err { color: var(--negative); font-size: 13px; margin: 10px 0 0; }
  /* footer-actions */
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: nowrap;
  }
  @media (max-width: 600px) {
    .footer-actions button { flex: 1 1 0; min-width: 0; }
  }
  .btn {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 8px 12px;
    border-radius: 8px;
    cursor: pointer;
    font: inherit;
  }
  .btn:hover { background: var(--surface-hover); }
  .btn.primary {
    background: var(--accent); color: var(--accent-fg); border: 0;
  }
  .btn.primary:hover { background: var(--accent-hover); }
  .btn.primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
