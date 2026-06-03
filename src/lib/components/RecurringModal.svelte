<script lang="ts">
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import DateField from './DateField.svelte';
  import { t } from '$lib/settings.svelte';
  import {
    api,
    type RecurringPayment, type Account, type Category,
    type NewRecurringPayload, type UpdateRecurringPayload, errMsg} from '$lib/api';
  import { fmtEurInput, parseEur } from '$lib/format';
  import { snackbar } from '$lib/snackbar.svelte';

  interface Props {
    recurring: RecurringPayment | null;
    accounts: Account[];
    categories: Category[];
    onClose: () => void;
    onSaved: () => void;
    onDeleted?: () => void;
  }
  let { recurring, accounts, categories, onClose, onSaved, onDeleted }: Props = $props();

  const isEdit = $derived(recurring !== null);

  /* svelte-ignore state_referenced_locally */
  let formName = $state(recurring?.name ?? '');
  /* svelte-ignore state_referenced_locally */
  let formAccountId = $state<number>(recurring?.accountId ?? (accounts[0]?.id ?? 0));
  /* svelte-ignore state_referenced_locally */
  let formCategoryId = $state<number | null>(recurring?.categoryId ?? null);
  /* svelte-ignore state_referenced_locally */
  let formAmount = $state(recurring ? fmtEurInput(recurring.amountCents) : '');
  /* svelte-ignore state_referenced_locally */
  let formSign = $state<'income' | 'expense'>(
    recurring && recurring.amountCents > 0 ? 'income' : 'expense'
  );
  /* svelte-ignore state_referenced_locally */
  let formFrequency = $state<'weekly' | 'monthly' | 'quarterly' | 'yearly'>(
    (recurring?.frequency as 'weekly' | 'monthly' | 'quarterly' | 'yearly' | undefined) ?? 'monthly'
  );
  /* svelte-ignore state_referenced_locally */
  let formAnchorDate = $state(recurring?.anchorDate ?? new Date().toISOString().slice(0, 10));
  /* svelte-ignore state_referenced_locally */
  let formCounterparty = $state(recurring?.counterparty ?? '');
  /* svelte-ignore state_referenced_locally */
  let formNote = $state(recurring?.note ?? '');

  let saving = $state(false);
  let error = $state<string | null>(null);

  const tr = $derived(t().recurring);
  const tc = $derived(t().common);

  function parseAmountCents(): number | null {
    const n = parseEur(formAmount);
    if (!Number.isFinite(n) || n <= 0) return null;
    const cents = Math.round(n * 100);
    return formSign === 'expense' ? -cents : cents;
  }

  async function save() {
    error = null;
    if (!formName.trim()) {
      error = tr.name;
      return;
    }
    const cents = parseAmountCents();
    if (cents === null) {
      error = tr.amount;
      return;
    }
    saving = true;
    try {
      if (recurring === null) {
        const payload: NewRecurringPayload = {
          name: formName.trim(),
          accountId: formAccountId,
          categoryId: formCategoryId,
          amountCents: cents,
          frequency: formFrequency,
          anchorDate: formAnchorDate,
          counterparty: formCounterparty.trim() || null,
          note: formNote.trim() || null,
        };
        await api.createRecurring(payload);
      } else {
        const payload: UpdateRecurringPayload = {
          name: formName.trim(),
          categoryId: formCategoryId,
          amountCents: cents,
          frequency: formFrequency,
          anchorDate: formAnchorDate,
          counterparty: formCounterparty.trim() || null,
          note: formNote.trim() || null,
        };
        await api.updateRecurring(recurring.id, payload);
      }
      onSaved();
    } catch (e) {
      error = errMsg(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!recurring) return;
    const id = recurring.id;
    try {
      await api.deleteRecurring(id);
    } catch (e) {
      error = errMsg(e);
      return;
    }
    onDeleted?.();
    snackbar.showUndo(tc.deleted, tc.undo, async () => {
      await api.restoreRecurring(id);
      onSaved();
    });
  }
</script>

<Sheet open={true} {onClose} title={isEdit ? tr.modalTitleEdit : tr.modalTitleNew}>
  {#snippet footer()}
    <div class="footer-actions">
      {#if isEdit}
        <button type="button" class="btn danger" onclick={remove}>
          {t().common.delete}
        </button>
      {/if}
      <button type="button" class="btn" onclick={onClose}>{t().common.cancel}</button>
      <button type="button" class="btn primary" disabled={saving} onclick={save}>
        {t().common.save}
      </button>
    </div>
  {/snippet}

  <label class="field">
    <span>{tr.name}</span>
    <input type="text" bind:value={formName} placeholder="Miete" />
  </label>

  <label class="field">
    <span>{t().common.account}</span>
    <select bind:value={formAccountId}>
      {#each accounts as a (a.id)}
        <option value={a.id}>{a.name}</option>
      {/each}
    </select>
  </label>

  <label class="field">
    <span>{t().common.categories}</span>
    <select bind:value={formCategoryId}>
      <option value={null}>—</option>
      {#each categories as c (c.id)}
        <option value={c.id}>{c.name}</option>
      {/each}
    </select>
  </label>

  <div class="row-2">
    <label class="field amount-field">
      <span>{tr.amount} (€)</span>
      <input type="text" inputmode="decimal" bind:value={formAmount} placeholder="0.00" />
    </label>
    <div class="field">
      <span>&nbsp;</span>
      <div class="sign-toggle">
        <button type="button" class:on={formSign === 'expense'} onclick={() => (formSign = 'expense')}>
          {tr.expense}
        </button>
        <button type="button" class:on={formSign === 'income'} onclick={() => (formSign = 'income')}>
          {tr.income}
        </button>
      </div>
    </div>
  </div>

  <label class="field">
    <span>{tr.frequency}</span>
    <select bind:value={formFrequency}>
      <option value="weekly">{tr.weekly}</option>
      <option value="monthly">{tr.monthly}</option>
      <option value="quarterly">{tr.quarterly}</option>
      <option value="yearly">{tr.yearly}</option>
    </select>
  </label>

  <label class="field">
    <span>{tr.anchorDate}</span>
    <DateField bind:value={formAnchorDate} />
  </label>

  <label class="field">
    <span>{tr.counterparty}</span>
    <input type="text" bind:value={formCounterparty} placeholder="Vermieter" />
  </label>

  <label class="field">
    <span>{tr.note}</span>
    <textarea bind:value={formNote} rows="2"></textarea>
  </label>

  <p class="err" aria-live="polite">{#if error}<Icon name="alert-circle" size={14} /> {error}{/if}</p>
</Sheet>

<style>
  .field { display: grid; gap: 4px; font-size: 12px; color: var(--text-muted); margin-bottom: 12px; }
  .field input, .field select, .field textarea {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 8px 12px;
    font: inherit;
    color: var(--text);
  }
  .row-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .sign-toggle { display: flex; gap: 0; }
  .sign-toggle button {
    flex: 1;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 8px 8px;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }
  .sign-toggle button:first-child { border-radius: var(--r-sm) 0 0 var(--r-sm); border-right: 0; }
  .sign-toggle button:last-child { border-radius: 0 var(--r-sm) var(--r-sm) 0; }
  .sign-toggle button.on { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); }
  .field.amount-field input { font-family: var(--font-mono); }
  .err { color: var(--negative); font-size: 12px; margin: 0; }
  /* footer-actions */
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
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 8px 12px;
    cursor: pointer;
    font: inherit;
    color: var(--text);
  }
  .btn.primary { background: var(--accent); color: var(--accent-fg); border-color: var(--accent); }
  .btn.primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
