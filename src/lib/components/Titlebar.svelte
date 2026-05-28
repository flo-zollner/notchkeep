<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Icon from './Icon.svelte';

  const win = getCurrentWindow();

  async function onMin() {
    try { await win.minimize(); } catch { /* ignore */ }
  }
  async function onMax() {
    try { await win.toggleMaximize(); } catch { /* ignore */ }
  }
  async function onClose() {
    try { await win.close(); } catch { /* ignore */ }
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="title" data-tauri-drag-region>Notchkeep</div>
  <div class="controls">
    <button type="button" onclick={onMin} aria-label="Minimieren" title="Minimieren">
      <Icon name="minus" size={14} />
    </button>
    <button type="button" onclick={onMax} aria-label="Maximieren" title="Maximieren">
      <Icon name="square" size={11} />
    </button>
    <button type="button" class="close" onclick={onClose} aria-label="Schließen" title="Schließen">
      <Icon name="x" size={14} />
    </button>
  </div>
</div>

<style>
  .titlebar {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 32px;
    display: flex;
    align-items: stretch;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    z-index: 1000;
    user-select: none;
    -webkit-user-select: none;
  }
  .title {
    flex: 1;
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-size: 12px;
    color: var(--text-muted);
    font-weight: 500;
    letter-spacing: 0.01em;
    pointer-events: none;
  }
  .controls {
    display: flex;
    height: 100%;
  }
  .controls button {
    width: 46px;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
    /* Re-enable clicks; titlebar drag-region propagates to children otherwise. */
    -webkit-app-region: no-drag;
  }
  .controls button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .controls button.close:hover {
    background: var(--negative);
    color: white;
  }
  /* Hide titlebar on mobile/phone breakpoints — Tauri Android has system chrome */
  @media (max-width: 600px) {
    .titlebar {
      display: none;
    }
  }
</style>
