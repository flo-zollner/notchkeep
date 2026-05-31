<script lang="ts">
  import { page } from '$app/state';
  import Icon from './Icon.svelte';
  import { settings, setTheme, setHide, t } from '$lib/settings.svelte';
  import { navMain, navManage, isActive } from '$lib/nav-config';

  interface Props {
    budgetAlertCount: number;
    priceRefreshStage: 'idle' | 'started' | 'completed' | 'failed';
  }
  let { budgetAlertCount, priceRefreshStage }: Props = $props();
</script>

<aside class="rail" data-tour="nav">
  <a href="/" class="rail-brand" title={t().appName}>
    <div class="brand-dot">N</div>
  </a>

  {#each navMain as item (item.id)}
    <a
      class="rail-item"
      class:active={isActive(page.url.pathname, item.href)}
      href={item.href}
      title={t().nav[item.labelKey]}
    >
      <Icon name={item.icon} size={20} />
      {#if item.id === 'budgets' && budgetAlertCount > 0}
        <span class="rail-badge">{budgetAlertCount}</span>
      {/if}
    </a>
  {/each}

  <div class="rail-spacer"></div>

  {#each navManage as item (item.id)}
    <a
      class="rail-item"
      class:active={isActive(page.url.pathname, item.href)}
      href={item.href}
      title={t().nav[item.labelKey]}
    >
      <Icon name={item.icon} size={20} />
    </a>
  {/each}

  <button class="rail-item" onclick={() => setHide(!settings.hide)} title={t().common.hideAmounts}>
    <Icon name={settings.hide ? 'eye-off' : 'eye'} size={20} />
  </button>
  <button
    class="rail-item"
    onclick={() => setTheme(settings.theme === 'auto' ? 'light' : settings.theme === 'light' ? 'dark' : 'auto')}
    title={settings.theme === 'auto' ? t().common.themeLight : settings.theme === 'light' ? t().common.themeDark : t().common.themeAuto}
    aria-label={t().common.theme}
  >
    <Icon name={settings.theme === 'auto' ? 'monitor' : settings.theme === 'light' ? 'sun' : 'moon'} size={20} />
  </button>

  {#if priceRefreshStage === 'started'}
    <div class="rail-refresh" title="Kurse werden geladen"><span class="spinner"></span></div>
  {/if}
</aside>

<style>
  .rail {
    display: none;
    position: sticky; top: 0;
    height: 100vh;
    width: 56px;
    padding: 8px 4px;
    border-right: 1px solid var(--border);
    background: var(--bg);
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .rail-brand {
    padding: 6px;
    margin-bottom: 4px;
  }
  .rail-item {
    width: var(--tap); height: var(--tap);
    display: flex; align-items: center; justify-content: center;
    border-radius: var(--r-sm);
    color: var(--text-muted);
    text-decoration: none;
    position: relative;
  }
  .rail-item.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
  @media (hover: hover) {
    .rail-item:hover { background: var(--surface-2); color: var(--text); }
  }
  .rail-spacer { flex: 1; }
  .rail-badge {
    position: absolute;
    top: 4px; right: 4px;
    min-width: 18px; height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--negative);
    color: white;
    font-size: 10px; font-weight: 600;
    display: inline-flex; align-items: center; justify-content: center;
  }
  .rail-refresh {
    width: var(--tap); height: var(--tap);
    display: flex; align-items: center; justify-content: center;
  }
  .spinner {
    width: 12px; height: 12px;
    border: 1.5px solid var(--text-muted);
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (min-width: 600px) and (max-width: 1023px) {
    .rail { display: flex; }
  }
</style>
