<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    onClose: () => void;
    title?: string;
    header?: Snippet;
    footer?: Snippet;
    children: Snippet;
    /** Enables drag-down-to-dismiss on phone. Default: true. */
    dismissable?: boolean;
  }
  let { open, onClose, title, header, footer, children, dismissable = true }: Props = $props();

  let panel: HTMLDivElement | undefined = $state();
  let dragStartY = $state<number | null>(null);
  let dragDelta = $state(0);
  let isDragging = $state(false);

  function onPointerDown(e: PointerEvent) {
    if (!dismissable) return;
    if (window.matchMedia('(min-width: 600px)').matches) return;
    dragStartY = e.clientY;
    dragDelta = 0;
    isDragging = true;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }
  function onPointerMove(e: PointerEvent) {
    if (!isDragging || dragStartY === null) return;
    dragDelta = Math.max(0, e.clientY - dragStartY);
  }
  function onPointerUp() {
    if (!isDragging) return;
    isDragging = false;
    if (dragDelta > 100) {
      dragDelta = 0;
      onClose();
    } else {
      dragDelta = 0;
    }
    dragStartY = null;
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) onClose();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="sheet-backdrop" onclick={onBackdropClick} role="presentation">
    <div
      class="sheet-panel"
      bind:this={panel}
      style:transform={isDragging ? `translateY(${dragDelta}px)` : ''}
      role="dialog"
      aria-modal="true"
      aria-label={title ?? 'Dialog'}
    >
      <div
        class="sheet-handle-wrap"
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
      >
        <div class="sheet-handle"></div>
      </div>
      {#if header}{@render header()}{:else if title}
        <div class="sheet-header">
          <h3>{title}</h3>
          <button class="sheet-close" onclick={onClose} aria-label="Schließen">×</button>
        </div>
      {/if}
      <div class="sheet-body">
        {@render children()}
      </div>
      {#if footer}
        <div class="sheet-footer">{@render footer()}</div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .sheet-backdrop {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: 100;
    display: flex; align-items: center; justify-content: center;
    backdrop-filter: blur(2px);
  }
  .sheet-panel {
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    display: flex; flex-direction: column;
    border-radius: var(--r-lg);
    max-width: 540px;
    width: 100%;
    max-height: 90vh;
  }
  .sheet-handle-wrap { display: none; padding: 8px 0 4px; cursor: grab; touch-action: none; }
  .sheet-handle { width: 38px; height: 4px; border-radius: 2px; background: var(--border-strong); margin: 0 auto; }
  .sheet-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    position: sticky; top: 0; background: var(--surface); z-index: 1;
  }
  .sheet-header h3 { margin: 0; font-size: 15px; font-weight: 600; }
  .sheet-close {
    width: var(--tap); height: var(--tap);
    font-size: 22px; line-height: 1; color: var(--text-muted);
    border-radius: var(--r-sm);
  }
  .sheet-body { padding: 14px 18px; overflow: auto; flex: 1; }
  .sheet-footer {
    padding: 12px 18px;
    border-top: 1px solid var(--border);
    position: sticky; bottom: 0; background: var(--surface);
  }
  @media (max-width: 599px) {
    .sheet-backdrop { align-items: flex-end; }
    .sheet-panel {
      max-width: none; width: 100%;
      border-radius: var(--r-lg) var(--r-lg) 0 0;
      border-bottom: none;
    }
    .sheet-handle-wrap { display: block; }
  }
</style>
