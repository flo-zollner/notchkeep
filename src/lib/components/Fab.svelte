<script lang="ts">
  import { page } from '$app/state';
  import { fabRoutes } from '$lib/nav-config';

  interface Props {
    onClick: () => void;
    label?: string;
  }
  let { onClick, label = 'Neue Transaktion' }: Props = $props();

  const visible = $derived(fabRoutes.has(page.url.pathname));
</script>

{#if visible}
  <button class="fab" data-tour="fab" onclick={onClick} aria-label={label}>
    <span class="fab-plus">+</span>
  </button>
{/if}

<style>
  .fab {
    position: fixed;
    right: 18px;
    bottom: calc(var(--tabbar-height) + env(safe-area-inset-bottom) + 18px);
    width: 56px; height: 56px;
    border-radius: 28px;
    background: var(--accent);
    color: var(--accent-fg);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.25), 0 0 0 0.5px rgba(0, 0, 0, 0.08);
    z-index: 60;
    display: none;
    align-items: center; justify-content: center;
    -webkit-tap-highlight-color: transparent;
  }
  .fab:active { transform: scale(0.94); }
  .fab-plus { font-size: 30px; line-height: 1; font-weight: 400; }
  @media (max-width: 599px) {
    .fab { display: flex; }
  }
</style>
