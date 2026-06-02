<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    children: Snippet;
    label?: string;
  }

  let { children, label = 'Mehr' }: Props = $props();

  let open = $state(false);
  let buttonEl = $state<HTMLButtonElement | null>(null);
  let menuEl = $state<HTMLDivElement | null>(null);

  function toggle() {
    open = !open;
  }

  function handleMenuClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (target && target.closest('button, a')) {
      open = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') open = false;
  }

  onMount(() => {
    function onPointerDown(e: PointerEvent) {
      if (!open) return;
      const target = e.target as Node | null;
      if (
        target &&
        !menuEl?.contains(target) &&
        !buttonEl?.contains(target)
      ) {
        open = false;
      }
    }
    window.addEventListener('pointerdown', onPointerDown);
    return () => window.removeEventListener('pointerdown', onPointerDown);
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="overflow-menu-wrap">
  <button
    bind:this={buttonEl}
    class="btn icon"
    type="button"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={label}
    onclick={toggle}
  >
    <Icon name="dots" size={20} />
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      bind:this={menuEl}
      class="overflow-menu"
      role="menu"
      tabindex="-1"
      onclick={handleMenuClick}
    >
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .overflow-menu-wrap {
    position: relative;
    display: inline-flex;
  }

  .overflow-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--r-md, 8px);
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.18));
    min-width: 200px;
    z-index: 60;
    padding: 6px;
  }

  :global(.overflow-menu button),
  :global(.overflow-menu a) {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    padding: 8px 10px;
    border-radius: var(--r-sm, 6px);
    background: none;
    border: none;
    color: inherit;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
    font: inherit;
    text-decoration: none;
  }

  :global(.overflow-menu button:hover),
  :global(.overflow-menu a:hover) {
    background: var(--surface-2);
  }
</style>
