<script lang="ts">
  import { page } from '$app/state';
  import Icon from './Icon.svelte';
  import { t } from '$lib/settings.svelte';
  import { mobileTabs, isActive } from '$lib/nav-config';
  import { ripple } from '$lib/actions/ripple';
  import MoreSheet from './MoreSheet.svelte';

  interface Props {
    budgetAlertCount: number;
  }
  let { budgetAlertCount }: Props = $props();

  let moreOpen = $state(false);

  const moreActive = $derived.by(() => {
    const path = page.url.pathname;
    // "More" is considered active when the current path is not among the mobileTabs
    return !mobileTabs.some((tab) => isActive(path, tab.href));
  });
</script>

<nav class="tabbar" data-tour="mobilenav">
  {#each mobileTabs as tab (tab.id)}
    <a class="tab" class:active={isActive(page.url.pathname, tab.href)} href={tab.href} use:ripple>
      <span class="tab-icon">
        <Icon name={tab.icon} size={22} />
        {#if tab.id === 'budgets' && budgetAlertCount > 0}
          <span class="tab-badge">{budgetAlertCount}</span>
        {/if}
      </span>
      <span class="tab-label">{t().nav[tab.labelKey]}</span>
    </a>
  {/each}
  <button class="tab" class:active={moreActive} onclick={() => (moreOpen = true)} aria-label="Mehr" use:ripple>
    <span class="tab-icon"><Icon name="dots" size={22} /></span>
    <span class="tab-label">Mehr</span>
  </button>
</nav>

<MoreSheet open={moreOpen} onClose={() => (moreOpen = false)} />

<style>
  .tabbar {
    display: none;
    position: fixed; bottom: 0; left: 0; right: 0;
    height: var(--tabbar-height);
    padding-bottom: env(safe-area-inset-bottom);
    background: var(--surface);
    border-top: 1px solid var(--border);
    z-index: 50;
  }
  .tab {
    flex: 1;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 2px;
    padding: 6px 0;
    color: var(--text-muted);
    text-decoration: none;
    background: none; border: none;
    -webkit-tap-highlight-color: transparent;
  }
  .tab:active { opacity: 0.6; }
  .tab.active { color: var(--accent); }
  .tab-icon { position: relative; display: inline-flex; }
  .tab-badge {
    position: absolute;
    top: -4px; right: -8px;
    min-width: 16px; height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--negative);
    color: white;
    font-size: 9px; font-weight: 600;
    display: inline-flex; align-items: center; justify-content: center;
  }
  .tab-label { font-size: 11px; font-weight: 500; }
  @media (max-width: 599px) {
    .tabbar { display: flex; }
  }

  /* ── Material Design 3 Navigation Bar (Android) ── */
  :global(html[data-platform='android']) .tabbar {
    background: var(--md-sys-color-surface-container);
    border-top: none;
  }
  :global(html[data-platform='android']) .tab {
    justify-content: flex-start;
    padding-top: 12px;
    gap: 4px;
  }
  /* The icon wrapper becomes the MD3 active-indicator pill. */
  :global(html[data-platform='android']) .tab-icon {
    width: 64px;
    height: 32px;
    border-radius: 16px;
    align-items: center;
    justify-content: center;
    transition: background var(--md-dur-medium) var(--md-ease-emphasized);
  }
  :global(html[data-platform='android']) .tab.active .tab-icon {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  :global(html[data-platform='android']) .tab.active .tab-label {
    color: var(--text);
    font-weight: 600;
  }
  :global(html[data-platform='android']) .tab-label {
    font-size: 12px;
  }
  /* Ripple replaces the flat opacity press feedback. */
  :global(html[data-platform='android']) .tab:active {
    opacity: 1;
  }
</style>
