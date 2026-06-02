<script lang="ts">
  import { t } from '$lib/settings.svelte';
  import { updateState } from '$lib/updater/updater.svelte';

  let { onInstall, onSkip, onClose, onRestart }: {
    onInstall: () => void;
    onSkip: () => void;
    onClose: () => void;
    onRestart: () => void;
  } = $props();

  const u = $derived(t().updates);
  const c = $derived(t().common);

  const percent = $derived(
    updateState.total > 0 ? Math.round((updateState.downloaded / updateState.total) * 100) : 0
  );

  let titleId = 'update-available-title';
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') onClose(); }} />

<div class="backdrop" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}>
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
  >
    {#if updateState.status === 'ready' && updateState.supportsRestart}
      <div class="header">
        <h3 id={titleId}>{u.readyTitle}</h3>
      </div>
      <div class="body">
        <p>{u.readyBody}</p>
      </div>
      <div class="footer">
        <div class="right-actions">
          <button type="button" class="btn" onclick={onClose}>
            {u.restartLater}
          </button>
          <button type="button" class="btn primary" onclick={onRestart}>
            {u.restartNow}
          </button>
        </div>
      </div>

    {:else if updateState.status === 'ready'}
      <div class="header">
        <h3 id={titleId}>{u.readyTitle}</h3>
      </div>
      <div class="body">
        <p>{u.installerLaunched}</p>
      </div>
      <div class="footer">
        <div class="right-actions">
          <button type="button" class="btn primary" onclick={onClose}>
            {c.close}
          </button>
        </div>
      </div>

    {:else if updateState.status === 'downloading'}
      <div class="header">
        <h3 id={titleId}>{u.downloading}</h3>
      </div>
      <div class="body">
        <div class="progress-wrap" role="progressbar" aria-valuenow={percent} aria-valuemin={0} aria-valuemax={100} aria-label={u.downloading}>
          <div class="progress-bar">
            <div class="progress-fill" style:width="{percent}%"></div>
          </div>
          <span class="progress-label">{percent}%</span>
        </div>
      </div>

    {:else}
      <!-- status === 'available' (or anything else) -->
      <div class="header">
        <h3 id={titleId}>{u.availableTitle(updateState.availableVersion ?? '')}</h3>
      </div>
      <div class="body">
        <p>{u.availableBody}</p>
        {#if updateState.notes}
          <pre class="notes">{updateState.notes}</pre>
        {/if}
      </div>
      <div class="footer">
        <button type="button" class="btn ghost" onclick={onSkip}>
          {u.skipThisVersion}
        </button>
        <div class="right-actions">
          <button type="button" class="btn" onclick={onClose}>
            {c.close}
          </button>
          <button type="button" class="btn primary" onclick={onInstall}>
            {u.install}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(2px);
  }

  .panel {
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    border-radius: var(--r-lg);
    display: flex;
    flex-direction: column;
    max-width: 480px;
    width: calc(100% - 2rem);
    max-height: 90vh;
    overflow: hidden;
  }

  .header {
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--border);
  }

  .header h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
  }

  .body {
    padding: 14px 18px;
    overflow: auto;
    flex: 1;
  }

  .body p {
    margin: 0 0 10px;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .body p:last-child {
    margin-bottom: 0;
  }

  .notes {
    margin: 10px 0 0;
    padding: 10px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--r-sm, 6px);
    font-size: 12px;
    color: var(--text-muted);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 160px;
    overflow-y: auto;
  }

  .progress-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 0;
  }

  .progress-bar {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    background: var(--surface-2);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.2s ease;
  }

  .progress-label {
    font-size: 12px;
    color: var(--text-muted);
    min-width: 36px;
    text-align: right;
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--border);
  }

  .right-actions {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }

  .btn {
    padding: 8px 14px;
    border-radius: var(--r-sm, 6px);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
  }

  .btn.primary {
    background: var(--accent);
    color: var(--accent-fg, white);
    border: 0;
  }

  .btn.ghost {
    border: 0;
    background: transparent;
    color: var(--text-muted);
    padding-left: 4px;
  }

  .btn:hover:not(:disabled) {
    opacity: 0.85;
  }

  @media (max-width: 599px) {
    .footer {
      flex-direction: column;
      align-items: stretch;
    }
    .right-actions {
      flex-direction: row-reverse;
      margin-left: 0;
    }
    .right-actions .btn {
      flex: 1;
    }
    .btn {
      min-height: 44px;
      white-space: normal;
    }
    .btn.ghost {
      text-align: center;
    }
  }
</style>
