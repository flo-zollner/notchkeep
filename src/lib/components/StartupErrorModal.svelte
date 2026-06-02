<script lang="ts">
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import { invalidateAll } from '$app/navigation';
  import { api, errMsg} from '$lib/api';
  import Sheet from './Sheet.svelte';
  import { t } from '$lib/settings.svelte';

  interface Props {
    badPath: string;
    onResolved: () => void;
  }
  let { badPath, onResolved }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  async function retry() {
    busy = true; error = null;
    try {
      await api.retryStartup();
      await invalidateAll();
      onResolved();
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }

  async function pickPath() {
    const picked = await openDialog({ directory: true });
    if (typeof picked !== 'string') return;
    busy = true; error = null;
    try {
      await api.setPathAndInit(picked);
      await invalidateAll();
      onResolved();
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }

  async function useDefault() {
    busy = true; error = null;
    try {
      await api.resetPathToDefault();
      await invalidateAll();
      onResolved();
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }
</script>

<Sheet open={true} onClose={() => {}} title="Datenpfad nicht erreichbar" dismissable={false}>
  <p class="path">{badPath}</p>
  <p class="hint">{t().data.startupErrorHint}</p>
  {#if error}<p class="error">{error}</p>{/if}

  {#snippet footer()}
    <div class="footer-actions">
      <button disabled={busy} onclick={pickPath}>{t().data.startupErrorPick}</button>
      <button disabled={busy} onclick={useDefault}>{t().data.startupErrorDefault}</button>
      <button class="primary" disabled={busy} onclick={retry}>{t().data.startupErrorRetry}</button>
    </div>
  {/snippet}
</Sheet>

<style>
  .path {
    margin: 0 0 12px 0; font-family: monospace; font-size: 12px;
    color: var(--text-muted); word-break: break-all;
  }
  .hint { font-size: 13px; color: var(--text-muted); margin: 0 0 16px 0; }
  .error { color: var(--negative); font-size: 12px; }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }
  button {
    padding: 10px 14px; border-radius: 6px; border: 1px solid var(--border);
    background: var(--surface-2); color: var(--text); cursor: pointer; font: inherit;
    white-space: normal;
  }
  button.primary {
    background: var(--accent); color: var(--accent-fg, white); border: 0;
  }
  button:disabled { opacity: 0.5; cursor: wait; }
  @media (max-width: 599px) {
    .footer-actions { flex-direction: column; align-items: stretch; }
    .footer-actions button { flex: 1 1 0; min-width: 0; min-height: 44px; }
  }
</style>
