<script lang="ts">
  import { snackbar } from '$lib/snackbar.svelte';
  import { ripple } from '$lib/actions/ripple';
</script>

<!-- Live region is always present (pre-rendered, guide §21); only the visible
     pill renders when there is a message. -->
<div class="snackbar-region" aria-live="polite" aria-atomic="true">
  {#if snackbar.current}
    {@const s = snackbar.current}
    <div class="snackbar">
      <span class="snackbar-msg">{s.message}</span>
      {#if s.actionLabel}
        <button class="snackbar-action" use:ripple onclick={() => snackbar.runAction()}>
          {s.actionLabel}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .snackbar-region {
    position: fixed;
    left: 0;
    right: 0;
    bottom: calc(24px + env(safe-area-inset-bottom));
    z-index: 200;
    display: flex;
    justify-content: center;
    padding: 0 16px;
    pointer-events: none;
  }
  /* Mobile: sit above the bottom navigation bar. */
  :global(html[data-platform='android']) .snackbar-region {
    bottom: calc(var(--tabbar-height) + env(safe-area-inset-bottom) + 12px);
    justify-content: flex-start;
  }

  .snackbar {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 560px;
    width: 100%;
    padding: 12px 12px 12px 16px;
    border-radius: var(--r-sm);
    background: var(--text);
    color: var(--bg);
    box-shadow: var(--shadow-lg);
    font-size: 13.5px;
    animation: snackbar-in var(--md-dur-medium, 250ms) var(--md-ease-emphasized, ease-out);
  }
  .snackbar-msg {
    flex: 1;
    min-width: 0;
  }
  .snackbar-action {
    flex: none;
    border: none;
    background: transparent;
    color: var(--accent);
    font-weight: 600;
    font-size: 13.5px;
    padding: 8px 12px;
    min-height: var(--tap);
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .snackbar-action:hover {
    background: color-mix(in oklab, var(--accent) 14%, transparent);
  }

  @keyframes snackbar-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .snackbar {
      animation: none;
    }
  }
</style>
