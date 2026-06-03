<script lang="ts">
  import { api, errMsg} from '$lib/api';
  import Sheet from './Sheet.svelte';
  import { t } from '$lib/settings.svelte';

  interface Props {
    open?: boolean;
    onClose: () => void;
    onApplied: () => void;
  }
  let { open = true, onClose, onApplied }: Props = $props();

  let typed = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);

  const trigger = $derived(() => {
    const s = t().data.resetConfirmType;
    const m = s.match(/[„"']([^„"']+)[„"']/);
    return m?.[1] ?? 'löschen';
  });

  async function confirm() {
    busy = true;
    error = null;
    try {
      await api.wipeDatabase();
      onApplied();
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }
</script>

<Sheet {open} onClose={() => !busy && onClose()} title="Daten zurücksetzen?">
  <p class="hint">{t().data.resetConfirmHint}</p>

  <label>
    {t().data.resetConfirmType}
    <input bind:value={typed} disabled={busy} autocomplete="off" />
  </label>

  <p class="error" aria-live="polite">{error ?? ''}</p>

  {#snippet footer()}
    <div class="footer-actions">
      <button class="btn ghost" disabled={busy} onclick={onClose}>{t().common.cancel}</button>
      <button class="btn danger" disabled={busy || typed !== trigger()} onclick={confirm}>
        {t().data.resetButton}
      </button>
    </div>
  {/snippet}
</Sheet>

<style>
  .hint { font-size: 12px; color: var(--text-muted); margin: 0 0 12px 0; }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--text-muted); }
  input {
    background: var(--surface-2); border: 1px solid var(--border);
    border-radius: var(--r-sm); padding: 8px 12px; font: inherit; color: var(--text);
  }
  .error { color: var(--negative); font-size: 12px; }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }
  .btn {
    padding: 8px 14px; border-radius: var(--r-sm); border: 1px solid var(--border);
    background: var(--surface-2); color: var(--text); cursor: pointer;
    font: inherit;
  }
  .btn.danger {
    border-color: var(--negative); color: var(--negative);
    background: var(--negative-soft);
  }
  .btn.ghost { border: 0; background: transparent; color: var(--text-muted); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
