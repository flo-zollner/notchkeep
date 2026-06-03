<script lang="ts">
  import { t } from '$lib/settings.svelte';

  let { onConfirm, onCancel }: {
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  const u = $derived(t().updates);
  const titleId = 'channel-warn-title';
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') onCancel(); }} />

<div class="backdrop" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) onCancel(); }}>
  <div class="panel" role="dialog" aria-modal="true" aria-labelledby={titleId}>
    <div class="header">
      <h3 id={titleId}>{u.betaWarnTitle}</h3>
    </div>
    <div class="body">
      <p>{u.betaWarnBody}</p>
    </div>
    <div class="footer">
      <div class="right-actions">
        <button type="button" class="btn" onclick={onCancel}>{u.betaWarnCancel}</button>
        <button type="button" class="btn primary" onclick={onConfirm}>{u.betaWarnConfirm}</button>
      </div>
    </div>
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
    max-width: 440px;
    width: calc(100% - 2rem);
    max-height: 90vh;
    overflow: hidden;
  }
  .header { padding: 16px 18px 12px; border-bottom: 1px solid var(--border); }
  .header h3 { margin: 0; font-size: 15px; font-weight: 600; color: var(--text); }
  .body { padding: 14px 18px; overflow: auto; flex: 1; }
  .body p { margin: 0; font-size: 13px; color: var(--text-muted); line-height: 1.5; }
  .footer {
    display: flex; align-items: center; justify-content: flex-end;
    flex-wrap: wrap; gap: 8px; padding: 12px 18px; border-top: 1px solid var(--border);
  }
  .right-actions { display: flex; gap: 8px; margin-left: auto; }
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
  .btn.primary { background: var(--accent); color: var(--accent-fg, white); border: 0; }
  .btn:hover:not(:disabled) { opacity: 0.85; }

  @media (max-width: 599px) {
    .footer { flex-direction: column; align-items: stretch; }
    .right-actions { flex-direction: row-reverse; margin-left: 0; }
    .right-actions .btn { flex: 1; }
    .btn { min-height: var(--tap, 48px); white-space: normal; }
  }
</style>
