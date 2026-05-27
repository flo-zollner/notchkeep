<script lang="ts">
  import { onMount } from 'svelte';
  import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { invalidateAll } from '$app/navigation';
  import { api, type AppConfigInfo, type PathCheckResult, type BackupValidation, type IntegrityReport, fmtEur, errMsg} from '$lib/api';
  import { t, settings } from '$lib/settings.svelte';
  import PathChangeModal from '$lib/components/PathChangeModal.svelte';
  import RestoreConfirmModal from '$lib/components/RestoreConfirmModal.svelte';
  import ResetConfirmModal from '$lib/components/ResetConfirmModal.svelte';

  let info = $state<AppConfigInfo | null>(null);
  let pathChangeContext = $state<{ targetDir: string; check: PathCheckResult } | null>(null);
  let restoreContext = $state<{ sourcePath: string; validation: BackupValidation } | null>(null);
  let resetOpen = $state(false);
  let toast = $state<string | null>(null);
  let forceBusy = $state(false);

  async function forceLock() {
    if (!confirm('Lock von einem anderen Gerät übernehmen? Wenn das andere Gerät noch läuft, kann das zu Sync-Konflikten führen.')) return;
    forceBusy = true;
    try {
      await api.forceAcquireSyncLock();
      toast = '✓ Lock übernommen';
      setTimeout(() => (toast = null), 3000);
      await load();
    } catch (e) {
      toast = String(e);
      setTimeout(() => (toast = null), 4000);
    } finally {
      forceBusy = false;
    }
  }

  function fmtBytes(b: number): string {
    if (b < 1024) return `${b} B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} kB`;
    return `${(b / 1024 / 1024).toFixed(2)} MB`;
  }
  function fmtDateTime(iso: string): string {
    return new Date(iso).toLocaleString();
  }

  async function load() {
    info = await api.getDataPathInfo();
  }
  onMount(load);

  async function startPathChange() {
    const picked = await openDialog({ directory: true });
    if (typeof picked !== 'string') return;
    const check = await api.checkTargetPath(picked);
    pathChangeContext = { targetDir: picked, check };
  }

  async function startBackup() {
    const today = new Date().toISOString().slice(0, 10);
    const target = await saveDialog({
      defaultPath: `budget-app-backup-${today}.sqlite`,
      filters: [{ name: 'SQLite', extensions: ['sqlite'] }],
    });
    if (typeof target !== 'string') return;
    try {
      const result = await api.backupDatabase(target);
      toast = `${t().data.backupSuccess} (${fmtBytes(result.bytes)}, ${result.durationMs}ms)`;
      setTimeout(() => (toast = null), 3000);
    } catch (e) {
      toast = errMsg(e);
    }
  }

  async function startRestore() {
    const picked = await openDialog({
      filters: [{ name: 'SQLite', extensions: ['sqlite'] }],
    });
    if (typeof picked !== 'string') return;
    const v = await api.validateBackup(picked);
    if (!v.ok) {
      toast = v.error ?? 'invalid';
      setTimeout(() => (toast = null), 3000);
      return;
    }
    restoreContext = { sourcePath: picked, validation: v };
  }

  let integrityReport = $state<IntegrityReport | null>(null);
  let integrityBusy = $state(false);

  async function runIntegrityScan() {
    integrityBusy = true;
    try {
      integrityReport = await api.findDataIssues();
    } catch (e) {
      toast = String(e);
      setTimeout(() => (toast = null), 3000);
    } finally {
      integrityBusy = false;
    }
  }

  async function applied(kind?: 'restore' | 'reset') {
    pathChangeContext = null;
    restoreContext = null;
    resetOpen = false;
    await load();
    await invalidateAll();
    if (kind === 'restore') {
      toast = t().data.restoreSuccess;
      setTimeout(() => (toast = null), 3000);
    } else if (kind === 'reset') {
      toast = t().data.resetSuccess;
      setTimeout(() => (toast = null), 3000);
    }
  }
</script>

<h1>{t().data.title}</h1>

<div class="grid">
  <div class="card card-pad-lg">
    <div class="card-h"><h3>{t().data.currentLocation}</h3></div>
    {#if info}
      <dl>
        <dt>{t().data.path}</dt>
        <dd class="mono">{info.dbPath}</dd>
        <dt>{t().data.size}</dt>
        <dd>{fmtBytes(info.dbSizeBytes)}</dd>
        <dt>{t().data.lastModified}</dt>
        <dd>{fmtDateTime(info.dbModifiedIso)}</dd>
        <dt>{t().data.syncLock}</dt>
        <dd>
          {#if info.lockHolder}
            {info.lockHolder.hostname} ({info.lockHolder.deviceId})
            — {fmtDateTime(info.lockHolder.acquiredAt)}
            <button class="force-lock" type="button" onclick={forceLock} disabled={forceBusy}>
              {forceBusy ? 'Übernehme…' : 'Lock übernehmen'}
            </button>
          {:else}
            —
          {/if}
        </dd>
      </dl>
      <button class="primary" onclick={startPathChange}>{t().data.changePath}</button>
    {:else}
      <p class="muted">…</p>
    {/if}
  </div>

  <div class="card card-pad-lg">
    <div class="card-h"><h3>{t().data.backupTitle}</h3></div>
    <p class="muted">{t().data.backupHint}</p>
    <button class="primary" onclick={startBackup}>{t().data.backupButton}</button>
  </div>

  <div class="card card-pad-lg">
    <div class="card-h"><h3>{t().data.restoreTitle}</h3></div>
    <p class="muted">{t().data.restoreHint}</p>
    <button class="warn" onclick={startRestore}>{t().data.restoreButton}</button>
  </div>

  <div class="card card-pad-lg destructive">
    <div class="card-h"><h3>{t().data.resetTitle}</h3></div>
    <p class="muted">{t().data.resetHint}</p>
    <button class="danger" onclick={() => (resetOpen = true)}>{t().data.resetButton}</button>
  </div>

  <div class="card card-pad-lg">
    <div class="card-h"><h3>Daten-Integrität</h3></div>
    <p class="muted">Scannt nach typischen Inkonsistenzen (verwaiste Trade-Tx, Allokationen zu archivierten Töpfen, Securities ohne Aktivität).</p>
    <button class="primary" onclick={runIntegrityScan} disabled={integrityBusy}>
      {integrityBusy ? 'Scannt…' : 'Prüfen'}
    </button>
    {#if integrityReport}
      <div class="integrity-results">
        {#if integrityReport.tradeKindWithoutTradeRow.length === 0
          && integrityReport.allocationsToArchivedBuckets.length === 0
          && integrityReport.zombieSecurities.length === 0}
          <p class="ok">✓ Keine Probleme gefunden.</p>
        {:else}
          {#if integrityReport.tradeKindWithoutTradeRow.length > 0}
            <h4>Trade-Tx ohne Trade-Details ({integrityReport.tradeKindWithoutTradeRow.length})</h4>
            <ul class="issue-list">
              {#each integrityReport.tradeKindWithoutTradeRow as o (o.id)}
                <li>Tx #{o.id}: {o.kind} · {o.bookingDate} · {o.counterparty ?? '—'} · {fmtEur(o.amountCents, { hide: settings.hide, signed: true, decimals: 0 })}</li>
              {/each}
            </ul>
            <p class="hint">Lösung: Tx löschen + CSV neu importieren, oder manuell Trade-Daten ergänzen.</p>
          {/if}
          {#if integrityReport.allocationsToArchivedBuckets.length > 0}
            <h4>Allokationen zu archivierten Töpfen ({integrityReport.allocationsToArchivedBuckets.length})</h4>
            <ul class="issue-list">
              {#each integrityReport.allocationsToArchivedBuckets as a (a.allocationId)}
                <li>{a.securityName} → {a.bucketName} (archiviert) · {(a.sharesMicro / 1_000_000).toFixed(2)} Stk.</li>
              {/each}
            </ul>
            <p class="hint">Lösung: Topf re-aktivieren oder Allokation auf /portfolio/[id] umzuordnen.</p>
          {/if}
          {#if integrityReport.zombieSecurities.length > 0}
            <h4>Securities ohne Trades + Preise ({integrityReport.zombieSecurities.length})</h4>
            <ul class="issue-list">
              {#each integrityReport.zombieSecurities as z (z.id)}
                <li>{z.name} ({z.isin})</li>
              {/each}
            </ul>
            <p class="hint">Können sicher gelöscht werden, falls nicht mehr benötigt.</p>
          {/if}
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if toast}
  <div class="toast">{toast}</div>
{/if}

{#if pathChangeContext}
  <PathChangeModal
    targetDir={pathChangeContext.targetDir}
    check={pathChangeContext.check}
    onClose={() => (pathChangeContext = null)}
    onApplied={applied}
  />
{/if}

{#if restoreContext}
  <RestoreConfirmModal
    sourcePath={restoreContext.sourcePath}
    validation={restoreContext.validation}
    onClose={() => (restoreContext = null)}
    onApplied={() => applied('restore')}
  />
{/if}

{#if resetOpen}
  <ResetConfirmModal
    onClose={() => (resetOpen = false)}
    onApplied={() => applied('reset')}
  />
{/if}

<style>
  .force-lock {
    margin-left: 8px;
    padding: 2px 8px;
    border-radius: 4px;
    border: 1px solid var(--warning);
    background: transparent;
    color: var(--warning);
    font-size: 11px;
    cursor: pointer;
    font: inherit;
  }
  .force-lock:hover:not(:disabled) {
    background: color-mix(in srgb, var(--warning) 12%, transparent);
  }
  .force-lock:disabled { opacity: 0.5; cursor: wait; }
  h1 { margin: 0 0 18px 0; }
  .grid { display: grid; gap: 16px; max-width: 700px; }
  dl { display: grid; grid-template-columns: 140px 1fr; gap: 6px 14px; font-size: 13px; margin: 0 0 14px 0; }
  dt { color: var(--text-muted); }
  dd { margin: 0; word-break: break-all; }
  .mono { font-family: monospace; font-size: 11px; }
  .muted { color: var(--text-muted); font-size: 13px; margin: 0 0 12px 0; }
  button {
    padding: 8px 14px; border-radius: 6px; border: 1px solid var(--border);
    background: var(--surface-2); color: var(--text); cursor: pointer; font: inherit;
  }
  button.primary { border-color: var(--accent); color: var(--accent); }
  button.warn { border-color: var(--warning); color: var(--warning); }
  button.danger { border-color: var(--negative); color: var(--negative); }
  .card.destructive { border-color: var(--negative); }
  .toast {
    position: fixed; bottom: 24px; left: 50%; transform: translateX(-50%);
    background: var(--surface); padding: 10px 16px; border-radius: 6px;
    border: 1px solid var(--border); font-size: 13px;
  }
  .integrity-results {
    margin-top: 12px;
    display: grid;
    gap: 10px;
    font-size: 13px;
  }
  .integrity-results .ok { color: var(--positive); font-weight: 500; }
  .integrity-results h4 {
    margin: 6px 0 4px 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .issue-list {
    margin: 0; padding: 0 0 0 18px;
    color: var(--text-muted); font-size: 12px;
  }
  .issue-list li { margin-bottom: 2px; }
  .integrity-results .hint {
    color: var(--text-faint); font-size: 11px; margin: 4px 0 0 0;
  }
</style>
