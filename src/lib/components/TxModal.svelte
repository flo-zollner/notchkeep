<script lang="ts">
  import { api, fmtEur, isTradeTx, todayIso, type Account, type Bucket, type Category, type CategorySuggestion, type Transaction, errMsg} from '$lib/api';
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import { settings, t, eurDecimals } from '$lib/settings.svelte';

  interface Props {
    open?: boolean;
    tx: Transaction | null;
    accounts: Account[];
    categories: Category[];
    onClose: () => void;
    onSaved: (tx: Transaction) => void;
    onDeleted?: (id: number) => void;
    onCategoryCreated?: (cat: Category) => void;
    defaultAccountId?: number | null;
  }

  let { open = true, tx, accounts, categories, onClose, onSaved, onDeleted, onCategoryCreated, defaultAccountId }: Props = $props();

  const tx_ = () => t().common as unknown as Record<string, string | undefined>;

  let buckets = $state<Bucket[]>([]);
  let bucketCashMap = $state<Map<number, number>>(new Map());
  let bucketSecMap = $state<Map<number, number>>(new Map());
  $effect(() => {
    api.listBuckets(false).then(async (bs) => {
      buckets = bs;
      try {
        const progress = await api.listBucketProgress();
        bucketCashMap = new Map(progress.map((p) => [p.bucketId, p.currentCents]));
        const secEntries = await Promise.all(
          bs.map(async (b) => {
            const rows = await api.bucketHoldings(b.id);
            return [b.id, rows.reduce((s, r) => s + r.valueCents, 0)] as const;
          })
        );
        bucketSecMap = new Map(secEntries);
      } catch {}
    }).catch(() => {});
  });

  // Inline "+ new category" form
  let creatingCat = $state(false);
  let newCatName = $state('');
  let creatingCatBusy = $state(false);

  async function createInlineCategory() {
    const name = newCatName.trim();
    if (!name) return;
    creatingCatBusy = true;
    try {
      const created = await api.createCategory({ name, icon: 'tag', color: '#94a3b8', rolloverEnabled: false });
      onCategoryCreated?.(created);
      categoryId = created.id;
      creatingCat = false;
      newCatName = '';
    } catch (e) {
      error = errMsg(e);
    } finally {
      creatingCatBusy = false;
    }
  }

  const editing = $derived(tx !== null);
  const isManual = $derived(tx?.source === 'manual' || tx === null);

  // Account-Dropdown: archivierte Accounts ausblenden, aber den aktuell verwendeten Account
  // weiterhin anzeigen, falls die zu bearbeitende Tx auf einen archivierten Account zeigt.
  const accountOptions = $derived(
    accounts.filter((a) => !a.archived || a.id === tx?.account_id),
  );

  // Form state initialised from props once at mount; modal is conditionally
  // mounted by parent, so prop reactivity is not desired.
  // Sign-toggle: 'in' = positive, 'out' = negative
  /* svelte-ignore state_referenced_locally */
  let direction = $state<'in' | 'out'>(tx && tx.amount_cents > 0 ? 'in' : 'out');
  // Absolute amount in cents, displayed/edited as decimal
  /* svelte-ignore state_referenced_locally */
  let amountInput = $state(tx ? (Math.abs(tx.amount_cents) / 100).toFixed(2) : '');
  /* svelte-ignore state_referenced_locally */
  let name = $state(tx?.counterparty ?? '');
  /* svelte-ignore state_referenced_locally */
  let note = $state(tx?.manual_note ?? '');
  /* svelte-ignore state_referenced_locally */
  let bookingDate = $state(tx?.booking_date ?? todayIso());
  /* svelte-ignore state_referenced_locally */
  let accountId = $state<number | null>(
    tx?.account_id ?? defaultAccountId ?? accounts.find((a) => !a.archived)?.id ?? accounts[0]?.id ?? null,
  );
  /* svelte-ignore state_referenced_locally */
  let categoryId = $state<number | null>(tx?.category_id ?? null);
  /* svelte-ignore state_referenced_locally */
  let bucketId = $state<number | null>(tx?.bucket_id ?? null);
  /* svelte-ignore state_referenced_locally */
  let counterpartyIban = $state<string>(tx?.counterparty_iban ?? '');
  let ibanError = $state<string | null>(null);
  // kind: nur Cash-Kinds editierbar. Bei neuer Tx Default 'expense' (wird beim
  // Save für Trade-Tx ohnehin via isTradeTx-Check gefiltert).
  /* svelte-ignore state_referenced_locally */
  let kind = $state<string>(tx?.kind ?? 'expense');

  const selectedBucket = $derived(buckets.find((b) => b.id === bucketId) ?? null);
  const selectedBucketTotal = $derived(
    bucketId !== null
      ? (bucketCashMap.get(bucketId) ?? 0) + (bucketSecMap.get(bucketId) ?? 0)
      : 0
  );
  const selectedBucketTarget = $derived(selectedBucket?.targetCents ?? null);
  const selectedBucketRemaining = $derived(
    selectedBucketTarget !== null
      ? Math.max(0, selectedBucketTarget - selectedBucketTotal)
      : null
  );

  let saving = $state(false);
  let deleting = $state(false);
  let confirmingDelete = $state(false);
  let error = $state<string | null>(null);

  // Live category suggestion
  let suggestion = $state<CategorySuggestion | null>(null);
  let suggestTimer: ReturnType<typeof setTimeout> | null = null;

  function scheduleSuggest() {
    if (suggestTimer) clearTimeout(suggestTimer);
    suggestion = null;
    const q = name.trim();
    if (q.length < 3) return;
    if (categoryId !== null) return; // user picked one already
    suggestTimer = setTimeout(async () => {
      try {
        const hit = await api.suggestCategory(q, accountId);
        if (hit && categoryId === null) suggestion = hit;
      } catch {
        // silently ignore — suggestion is best-effort
      }
    }, 300);
  }

  $effect(() => {
    // re-suggest when name changes
    name;
    scheduleSuggest();
  });

  function applySuggestion() {
    if (suggestion) {
      categoryId = suggestion.categoryId;
      suggestion = null;
    }
  }

  function clearSuggestion() {
    suggestion = null;
  }

  function parseAmountCents(): number | null {
    const cleaned = amountInput.replace(',', '.').trim();
    if (!cleaned) return null;
    const n = Number(cleaned);
    if (!Number.isFinite(n) || n < 0) return null;
    return Math.round(n * 100) * (direction === 'in' ? 1 : -1);
  }

  async function save() {
    error = null;
    const cents = parseAmountCents();
    if (cents === null || cents === 0) {
      error = t().common.amount;
      return;
    }
    if (!accountId) {
      error = t().common.account;
      return;
    }
    if (!bookingDate) {
      error = t().common.date;
      return;
    }

    // Validate IBAN (optional field — empty = OK)
    ibanError = null;
    const ibanTrimmed = counterpartyIban.replace(/\s+/g, '').toUpperCase();
    if (ibanTrimmed.length > 0) {
      if (!/^[A-Z]{2}\d{2}[A-Z0-9]{11,30}$/.test(ibanTrimmed)) {
        ibanError = t().common.invalidIban;
        saving = false;
        return;
      }
    }
    const ibanToSave = ibanTrimmed || null;

    saving = true;
    try {
      if (editing && tx) {
        // Trade-Tx erlauben nur partial-updates (Kategorie + Topf + Konto);
        // Stücke/Preis/Datum/Betrag laufen über die Trade-Commands. IBAN-Update
        // wird hier ignoriert. isTradeTx-Helper aus api.ts ist Single-Source-
        // of-Truth — deckt buy/sell/dividend/corporate_action/tax/fee ab.
        if (isTradeTx(tx)) {
          if (categoryId !== (tx.category_id ?? null)) {
            await api.assignCategory(tx.id, categoryId);
          }
          if (bucketId !== (tx.bucket_id ?? null)) {
            await api.assignBucket(tx.id, bucketId);
          }
          if (accountId && accountId !== tx.account_id) {
            await api.assignAccount(tx.id, accountId);
          }
          onSaved({
            ...tx,
            category_id: categoryId,
            bucket_id: bucketId,
            account_id: accountId ?? tx.account_id,
          });
        } else {
          const updated = await api.updateTransaction({
            id: tx.id,
            accountId,
            bookingDate,
            amountCents: cents,
            currency: tx.currency,
            counterparty: name.trim() || null,
            purpose: tx.purpose,
            categoryId,
            bucketId,
            manualNote: note.trim() || null,
            counterpartyIban: ibanToSave,
            kind: kind !== tx.kind ? kind : undefined,
          });
          onSaved(updated);
        }
      } else {
        const created = await api.createTransaction({
          accountId,
          bookingDate,
          amountCents: cents,
          counterparty: name.trim() || null,
          purpose: null,
          categoryId,
          bucketId,
          manualNote: note.trim() || null,
          counterpartyIban: ibanToSave,
          // Nur explizit setzen wenn vom Default 'expense' abgewichen; sonst
          // soll Backend's Auto-Logik income/expense aus Amount-Vorzeichen ableiten.
          kind: kind === 'expense' ? undefined : kind,
        });
        onSaved(created);
      }
      onClose();
    } catch (e) {
      error = errMsg(e);
    } finally {
      saving = false;
    }
  }

  async function doDelete() {
    if (!tx) return;
    if (!confirmingDelete) {
      confirmingDelete = true;
      return;
    }
    deleting = true;
    error = null;
    try {
      if (isTradeTx(tx)) {
        await api.deleteTrade(tx.id);
      } else {
        await api.deleteTransaction(tx.id);
      }
      onDeleted?.(tx.id);
      onClose();
    } catch (e) {
      error = errMsg(e);
      deleting = false;
    }
  }

</script>

<Sheet {open} {onClose} title={editing ? t().common.editTx : t().common.newTx}>
  <div class="dir-toggle" role="tablist">
      <button
        class="dir-btn"
        class:active={direction === 'out'}
        role="tab"
        aria-selected={direction === 'out'}
        onclick={() => (direction = 'out')}
      >
        {t().common.expense_out}
      </button>
      <button
        class="dir-btn"
        class:active={direction === 'in'}
        role="tab"
        aria-selected={direction === 'in'}
        onclick={() => (direction = 'in')}
      >
        {t().common.income_in}
      </button>
    </div>

    <div class="grid">
      <label class="field">
        <span class="field-label">{t().common.name}</span>
        <input
          class="input"
          type="text"
          bind:value={name}
          placeholder="REWE, Gehalt, …"
          autocomplete="off"
        />
      </label>

      {#if suggestion}
        <div class="suggest">
          <span class="suggest-text">
            {t().common.suggestion}: <strong>{suggestion.categoryName}</strong>
            <span class="score">· {(suggestion.score * 100).toFixed(0)}%</span>
          </span>
          <button class="btn primary sm" onclick={applySuggestion}>
            {t().common.applySuggestion}
          </button>
          <button class="btn icon sm" onclick={clearSuggestion} aria-label="dismiss">
            <Icon name="x" size={12} />
          </button>
        </div>
      {/if}

      <div class="row-2">
        <label class="field">
          <span class="field-label">{t().common.amount}</span>
          <input
            class="input"
            type="text"
            inputmode="decimal"
            bind:value={amountInput}
            placeholder="0,00"
          />
        </label>
        <label class="field">
          <span class="field-label">{t().common.date}</span>
          <input class="input" type="date" bind:value={bookingDate} />
        </label>
      </div>

      <label class="field">
        <span class="field-label">{t().common.type}</span>
        <select class="input" bind:value={kind}>
          <option value="expense">{t().txKind.expense}</option>
          <option value="income">{t().txKind.income}</option>
          <option value="transfer">{t().txKind.transfer}</option>
          <option value="fee">{t().txKind.fee}</option>
          <option value="tax_general">{t().txKind.tax_general}</option>
        </select>
      </label>

      <div class="row-2">
        <label class="field">
          <span class="field-label">{t().common.account}</span>
          <select class="input" bind:value={accountId}>
            {#each accountOptions as a (a.id)}
              <option value={a.id}>{a.name}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span class="field-label">{t().common.categories}</span>
          {#if creatingCat}
            <div class="inline-new">
              <input
                class="input"
                type="text"
                bind:value={newCatName}
                placeholder={t().common.newCategory}
                autocomplete="off"
                onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); createInlineCategory(); } }}
              />
              <button
                class="btn primary sm"
                onclick={createInlineCategory}
                disabled={creatingCatBusy || !newCatName.trim()}
                aria-label={t().common.save}
              >
                <Icon name="check" size={12} />
              </button>
              <button
                class="btn icon sm"
                onclick={() => { creatingCat = false; newCatName = ''; }}
                aria-label={t().common.cancel}
              >
                <Icon name="x" size={12} />
              </button>
            </div>
          {:else}
            <div class="cat-row">
              <select class="input" bind:value={categoryId}>
                <option value={null}>— {t().common.uncategorized} —</option>
                {#each categories as c (c.id)}
                  <option value={c.id}>{c.name}</option>
                {/each}
              </select>
              <button
                class="btn icon sm"
                onclick={() => { creatingCat = true; categoryId = null; }}
                aria-label={t().common.newCategory}
                title={t().common.newCategory}
              >
                <Icon name="plus" size={12} />
              </button>
            </div>
          {/if}
        </label>
      </div>

      <label class="field">
        <span class="field-label">{t().common.description}</span>
        <input
          class="input"
          type="text"
          bind:value={note}
          placeholder={t().common.descriptionPh}
        />
        {#if !isManual && tx?.purpose}
          <span class="hint">{tx.purpose}</span>
        {/if}
      </label>

      <label class="field">
        <span class="field-label">{t().common.counterpartyIban}</span>
        <div class="iban-row">
          <input
            class="input mono"
            type="text"
            bind:value={counterpartyIban}
            placeholder="DE89 3704 0044 0532 0130 00"
            autocapitalize="characters"
            spellcheck="false"
          />
          {#if counterpartyIban}
            <button
              type="button"
              class="btn"
              title="Copy"
              onclick={() => navigator.clipboard.writeText(counterpartyIban.replace(/\s+/g, '').toUpperCase())}
            >
              <Icon name="check" size={13} />
            </button>
          {/if}
        </div>
        {#if ibanError}
          <span class="hint err">{ibanError}</span>
        {/if}
      </label>

      <label class="field">
        <span class="field-label">{tx_().bucket ?? 'Topf'}</span>
        <select class="input" bind:value={bucketId}>
          <option value={null}>{tx_().bucketNone ?? '— kein Topf —'}</option>
          {#each buckets as b (b.id)}
            <option value={b.id}>{b.name}</option>
          {/each}
          {#if tx?.bucket_id && !buckets.some((b) => b.id === tx!.bucket_id)}
            <option value={tx.bucket_id}>({tx_().archived ?? 'archiviert'})</option>
          {/if}
        </select>
        {#if selectedBucket}
          <div class="bucket-state">
            <span class="bs-item">
              <span class="bs-lbl">{tx_().bucketCurrent ?? 'Bereits'}:</span>
              <span class="bs-num">{fmtEur(selectedBucketTotal, { hide: settings.hide, decimals: eurDecimals() })}</span>
            </span>
            {#if selectedBucketTarget !== null && selectedBucketTarget > 0}
              <span class="bs-sep">·</span>
              <span class="bs-item">
                <span class="bs-lbl">{tx_().bucketTarget ?? 'Ziel'}:</span>
                <span class="bs-num">{fmtEur(selectedBucketTarget, { hide: settings.hide, decimals: eurDecimals() })}</span>
              </span>
              <span class="bs-sep">·</span>
              <span class="bs-item" class:done={selectedBucketRemaining === 0}>
                <span class="bs-lbl">{selectedBucketRemaining === 0 ? (tx_().bucketReached ?? 'erreicht ✓') : (tx_().bucketRemaining ?? 'Noch')}:</span>
                {#if selectedBucketRemaining !== 0}
                  <span class="bs-num">{fmtEur(selectedBucketRemaining ?? 0, { hide: settings.hide, decimals: eurDecimals() })}</span>
                {/if}
              </span>
            {/if}
          </div>
        {/if}
      </label>
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

  {#snippet footer()}
    <div class="footer-actions">
      {#if editing}
        <button
          class="btn danger"
          onclick={doDelete}
          disabled={deleting}
        >
          {confirmingDelete ? t().common.confirmDelete : t().common.delete}
        </button>
      {/if}
      <button class="btn" onclick={onClose}>{t().common.cancel}</button>
      <button class="btn primary" onclick={save} disabled={saving}>
        {saving ? '…' : t().common.save}
      </button>
    </div>
  {/snippet}
</Sheet>

<style>
  .dir-toggle {
    display: grid;
    grid-template-columns: 1fr 1fr;
    background: var(--surface-2);
    border-radius: var(--r-sm);
    padding: 3px;
    margin-bottom: 16px;
  }
  .dir-btn {
    background: transparent;
    border: 0;
    padding: 8px 10px;
    border-radius: 6px;
    font: inherit;
    font-size: 13px;
    color: var(--text-muted);
    cursor: pointer;
  }
  .dir-btn.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
  .grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .row-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .field {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .hint {
    font-size: 11px;
    color: var(--text-faint);
    margin-top: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bucket-state {
    display: flex; flex-wrap: wrap; align-items: baseline;
    gap: 4px 6px; margin-top: 6px;
    font-size: 11px; color: var(--text-muted);
  }
  .bucket-state .bs-lbl { margin-right: 2px; }
  .bucket-state .bs-num {
    font-variant-numeric: tabular-nums; color: var(--text);
  }
  .bucket-state .bs-sep { color: var(--text-faint); }
  .bucket-state .done { color: var(--positive); }
  .bucket-state .done .bs-lbl { color: var(--positive); }
  .suggest {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: var(--accent-soft);
    border-radius: var(--r-sm);
    font-size: 13px;
  }
  .suggest-text {
    flex: 1;
    min-width: 0;
  }
  .suggest .score {
    color: var(--text-faint);
    font-size: 11px;
    margin-left: 4px;
  }
  .btn.sm {
    padding: 4px 10px;
    font-size: 12px;
  }
  .error {
    margin-top: 12px;
    padding: 8px 10px;
    background: var(--negative-soft);
    color: var(--negative);
    border-radius: var(--r-sm);
    font-size: 12px;
  }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }
  .footer-actions .btn.danger {
    margin-right: auto;
    background: var(--negative-soft);
    color: var(--negative);
  }
  .cat-row, .inline-new {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .cat-row select {
    flex: 1;
    min-width: 0;
  }
  .inline-new input {
    flex: 1;
    min-width: 0;
  }
  .iban-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .iban-row .input {
    flex: 1;
  }
  .hint.err {
    color: var(--negative);
  }
  .input.mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    letter-spacing: 0.02em;
  }
</style>
