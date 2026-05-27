<script lang="ts">
  import { api, type PathCheckResult, type ChangePathAction, errMsg} from '$lib/api';
  import Sheet from './Sheet.svelte';
  import { t } from '$lib/settings.svelte';

  interface Props {
    open?: boolean;
    targetDir: string;
    check: PathCheckResult;
    onClose: () => void;
    onApplied: () => void;
  }
  let { open = true, targetDir, check, onClose, onApplied }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  const title = $derived(
    check.kind === 'existing' ? t().data.pathChangeTitleExisting : t().data.pathChangeTitleEmpty
  );

  async function apply(action: ChangePathAction) {
    busy = true;
    error = null;
    try {
      await api.changeDataPath(targetDir, action);
      onApplied();
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }
</script>

<Sheet {open} onClose={() => !busy && onClose()} {title}>
  <p class="target">{targetDir}</p>
  {#if check.kind === 'existing'}
    <p class="hint">
      {(check.dbSizeBytes / 1024).toFixed(1)} kB · {check.valid ? '✓ valid' : '⚠ keine valide Notchkeep-DB'}
    </p>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#snippet footer()}
    <div class="footer-actions">
      {#if check.kind === 'existing'}
        {#if check.valid}
          <button disabled={busy} onclick={() => apply('useExisting')}>
            {t().data.pathChangeUseExisting}
          </button>
        {/if}
        <button class="btn warn" disabled={busy} onclick={() => apply('overwriteCopy')}>
          {t().data.pathChangeOverwriteCopy}
        </button>
      {:else}
        <button disabled={busy} onclick={() => apply('move')}>
          {t().data.pathChangeMove}
        </button>
        <button disabled={busy} onclick={() => apply('copy')}>
          {t().data.pathChangeCopy}
        </button>
        <button disabled={busy} onclick={() => apply('startFresh')}>
          {t().data.pathChangeStartFresh}
        </button>
      {/if}
      <button class="btn ghost" disabled={busy} onclick={onClose}>{t().common.cancel}</button>
    </div>
  {/snippet}
</Sheet>

<style>
  .target {
    margin: 0 0 12px 0; font-family: monospace; font-size: 12px;
    color: var(--text-muted); word-break: break-all;
  }
  .hint { font-size: 12px; color: var(--text-muted); margin: 0 0 16px 0; }
  .error { color: var(--negative); font-size: 12px; }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }
  button {
    padding: 8px 14px; border-radius: 6px; border: 1px solid var(--border);
    background: var(--surface-2); color: var(--text); cursor: pointer;
    font: inherit;
  }
  .btn.warn { border-color: var(--warning); color: var(--warning); }
  .btn.ghost { border: 0; background: transparent; color: var(--text-muted); }
  button:disabled { opacity: 0.5; cursor: wait; }
</style>
