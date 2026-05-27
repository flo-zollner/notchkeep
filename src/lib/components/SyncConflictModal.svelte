<script lang="ts">
  import Sheet from './Sheet.svelte';
  import { api, type SyncConflictFile } from '$lib/api';

  interface Props {
    conflicts: SyncConflictFile[];
    onResolved: () => void;
  }
  let { conflicts, onResolved }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  function fmtTime(unix: number): string {
    if (!unix) return '—';
    return new Date(unix * 1000).toLocaleString('de-DE');
  }

  async function keepCurrent() {
    if (busy) return;
    busy = true;
    error = null;
    try {
      await api.resolveConflictKeepCurrent();
      onResolved();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function useOther(path: string) {
    if (busy) return;
    busy = true;
    error = null;
    try {
      await api.resolveConflictUseOther(path);
      onResolved();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<Sheet open={true} onClose={() => { /* nicht dismissable bis resolved */ }} title="Sync-Konflikt erkannt" dismissable={false}>
  <p class="intro">
    Syncthing hat <strong>{conflicts.length}</strong> konkurrierende
    Versionen deiner Datenbank gefunden. Sowohl Desktop als auch
    Mobile haben gleichzeitig Änderungen gemacht. Wähle, welche
    Version aktuell weitergenutzt werden soll. Die jeweils anderen
    landen in <code>conflict-trash/</code> (löschbar, nichts geht verloren).
  </p>

  <div class="conflicts">
    {#each conflicts as c (c.path)}
      <div class="conflict">
        <div class="meta">
          <div class="name">{c.name}</div>
          <div class="time">geändert: {fmtTime(c.modifiedUnix)}</div>
        </div>
        <button class="btn" disabled={busy} onclick={() => useOther(c.path)}>
          Diese Version nutzen
        </button>
      </div>
    {/each}
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#snippet footer()}
    <div class="footer-actions">
      <button class="btn primary" disabled={busy} onclick={keepCurrent}>
        Aktuelle DB behalten, Konflikte archivieren
      </button>
    </div>
  {/snippet}
</Sheet>

<style>
  .intro {
    font-size: 13px;
    margin: 0 0 12px;
    color: var(--text);
  }
  .intro code {
    font-family: var(--font-mono);
    background: var(--surface-2);
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 12px;
  }
  .conflicts {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 10px 0 6px;
  }
  .conflict {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--r-sm);
    background: var(--surface-2);
  }
  .meta { flex: 1; min-width: 0; }
  .name {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .time {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 2px;
  }
  .btn {
    min-height: var(--tap);
    padding: 0 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    font-weight: 500;
    font-size: 12px;
    flex-shrink: 0;
  }
  .btn:disabled { opacity: 0.5; cursor: wait; }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-fg);
    border-color: var(--accent);
  }
  @media (hover: hover) {
    .btn:hover:not(:disabled) { background: var(--surface-hover); }
  }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    width: 100%;
  }
  .error {
    margin-top: 10px;
    padding: 10px;
    background: color-mix(in srgb, var(--negative) 10%, transparent);
    color: var(--negative);
    border-radius: var(--r-sm);
    font-size: 12px;
  }
</style>
