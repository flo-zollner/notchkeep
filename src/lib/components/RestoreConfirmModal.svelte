<script lang="ts">
  import { api, type BackupValidation, errMsg} from '$lib/api';
  import Sheet from './Sheet.svelte';
  import { t } from '$lib/settings.svelte';

  interface Props {
    open?: boolean;
    sourcePath: string;
    validation: BackupValidation;
    onClose: () => void;
    onApplied: () => void;
  }
  let { open = true, sourcePath, validation, onClose, onApplied }: Props = $props();

  let typed = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);

  const trigger = $derived(() => {
    const s = t().data.restoreConfirmType;
    const m = s.match(/[„"']([^„"']+)[„"']/);
    return m?.[1] ?? 'restore';
  });

  async function confirm() {
    busy = true;
    error = null;
    try {
      await api.restoreDatabase(sourcePath);
      onApplied();
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }
</script>

<Sheet {open} onClose={() => !busy && onClose()} title="Backup wiederherstellen?">
  <p class="hint">{t().data.restoreConfirmHint}</p>
  <div class="counts">
    <div><span>tx</span> {validation.rowCounts.transactions}</div>
    <div><span>accounts</span> {validation.rowCounts.accounts}</div>
    <div><span>cats</span> {validation.rowCounts.categories}</div>
    <div><span>securities</span> {validation.rowCounts.securities}</div>
    <div><span>recurring</span> {validation.rowCounts.recurringPayments}</div>
  </div>

  <label>
    {t().data.restoreConfirmType}
    <input bind:value={typed} disabled={busy} autocomplete="off" />
  </label>

  {#if error}<p class="error">{error}</p>{/if}

  {#snippet footer()}
    <div class="footer-actions">
      <button class="btn warn" disabled={busy || typed !== trigger()} onclick={confirm}>
        {t().data.restoreButton}
      </button>
      <button class="btn ghost" disabled={busy} onclick={onClose}>{t().common.cancel}</button>
    </div>
  {/snippet}
</Sheet>

<style>
  .hint { font-size: 12px; color: var(--text-muted); margin: 0 0 12px 0; }
  .counts {
    font-size: 11px; color: var(--text-muted);
    display: grid; grid-template-columns: 1fr 1fr; gap: 4px 12px;
    border: 1px solid var(--border); border-radius: 6px;
    padding: 10px; margin-bottom: 12px;
  }
  .counts span { font-weight: 500; color: var(--text); }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--text-muted); }
  input {
    background: var(--surface-2); border: 1px solid var(--border);
    border-radius: 6px; padding: 6px 10px; font: inherit; color: var(--text);
  }
  .error { color: var(--negative); font-size: 12px; }
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
    padding: 8px 14px; border-radius: 6px; border: 1px solid var(--border);
    background: var(--surface-2); color: var(--text); cursor: pointer;
    font: inherit;
  }
  .btn.warn { border-color: var(--negative); color: var(--negative); }
  .btn.ghost { border: 0; background: transparent; color: var(--text-muted); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
