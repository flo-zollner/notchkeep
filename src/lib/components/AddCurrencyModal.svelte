<script lang="ts">
  import { api, type CurrencyStatus, errMsg } from '$lib/api';
  import Sheet from './Sheet.svelte';
  import { t } from '$lib/settings.svelte';

  interface Props {
    open?: boolean;
    onClose: () => void;
    onAdded: (status: CurrencyStatus) => void;
  }
  let { open = true, onClose, onAdded }: Props = $props();

  const tc = $derived(t().currencies);

  let codeInput = $state('');
  let saving = $state(false);
  let error = $state<string | null>(null);

  function isValidCode(s: string): boolean {
    return /^[A-Z]{3}$/.test(s.toUpperCase());
  }

  async function fetchAndSave() {
    error = null;
    const code = codeInput.trim().toUpperCase();
    if (!isValidCode(code)) {
      error = tc.errInvalidCode;
      return;
    }
    saving = true;
    try {
      const result = await api.refreshCurrencyRate(code);
      onAdded(result);
      onClose();
    } catch (e) {
      error = `${tc.errFetch}: ${errMsg(e)}`;
    } finally {
      saving = false;
    }
  }
</script>

<Sheet {open} {onClose} title={tc.addModalTitle}>
  <label class="field">
    {tc.addModalCodeLabel}
    <input
      type="text"
      bind:value={codeInput}
      maxlength="3"
      placeholder="JPY"
      autocomplete="off"
      oninput={(e) => {
        const v = (e.target as HTMLInputElement).value.toUpperCase().replace(/[^A-Z]/g, '');
        codeInput = v;
      }}
    />
    <small class="hint">{tc.addModalCodeHint}</small>
  </label>

  <div class="err" aria-live="polite">{#if error}{error}{/if}</div>

  {#snippet footer()}
    <div class="footer-actions">
      <button type="button" class="btn cancel" onclick={onClose} disabled={saving}>
        {t().common.cancel}
      </button>
      <button
        type="button"
        class="btn save"
        onclick={fetchAndSave}
        disabled={saving || !isValidCode(codeInput)}
      >
        {saving ? '…' : tc.addModalFetchBtn}
      </button>
    </div>
  {/snippet}
</Sheet>

<style>
  .field { display: flex; flex-direction: column; gap: 4px; margin-bottom: 12px; font-size: 13px; }
  .field input {
    padding: 8px 10px; background: var(--surface-2);
    border: 1px solid var(--border); border-radius: 6px;
    color: var(--text); font-family: ui-monospace, monospace;
    text-transform: uppercase; letter-spacing: 0.1em;
    font-size: 16px;
  }
  .hint { color: var(--text-faint); font-size: 11px; }
  .err {
    color: var(--negative); font-size: 13px;
    padding: 10px; background: var(--negative-soft);
    border-radius: 6px; margin-bottom: 12px;
  }
  .err:empty { display: none; }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: nowrap;
  }
  @media (max-width: 600px) {
    .footer-actions button { flex: 1 1 0; min-width: 0; }
  }
  .btn { padding: 8px 16px; border-radius: 6px; border: 0; cursor: pointer; font-size: 14px; }
  .btn.cancel { background: transparent; color: var(--text-muted); }
  .btn.save { background: var(--accent, var(--positive)); color: white; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
