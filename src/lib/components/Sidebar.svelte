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

<aside class="sidebar">
  <div class="brand">
    <div class="brand-dot">S</div>
    {t().appName}
  </div>

  <div class="nav-section">{t().sections.main}</div>
  {#each navMain as item (item.id)}
    <a class="nav-item" class:active={isActive(page.url.pathname, item.href)} href={item.href}>
      <Icon name={item.icon} size={17} />
      {t().nav[item.labelKey]}
      {#if item.id === 'budgets' && budgetAlertCount > 0}
        <span class="alert-badge" title="{budgetAlertCount} Kategorien ≥ 80%">{budgetAlertCount}</span>
      {/if}
    </a>
  {/each}

  <div class="nav-section">{t().sections.manage}</div>
  {#each navManage as item (item.id)}
    <a class="nav-item" class:active={isActive(page.url.pathname, item.href)} href={item.href}>
      <Icon name={item.icon} size={17} />
      {t().nav[item.labelKey]}
    </a>
  {/each}

  {#if priceRefreshStage !== 'idle'}
    <div class="refresh-status" class:err={priceRefreshStage === 'failed'}>
      {#if priceRefreshStage === 'started'}
        <span class="spinner"></span> Kurse werden geladen…
      {:else if priceRefreshStage === 'completed'}
        ✓ Kurse aktualisiert
      {:else}
        ⚠ Kurs-Fetch fehlgeschlagen
      {/if}
    </div>
  {/if}

  <div class="footer-actions">
    <button class="nav-item" onclick={() => setHide(!settings.hide)}>
      <Icon name={settings.hide ? 'eye-off' : 'eye'} size={17} />
      {t().common.hideAmounts}
    </button>
    <button
      class="nav-item"
      onclick={() => setTheme(settings.theme === 'auto' ? 'light' : settings.theme === 'light' ? 'dark' : 'auto')}
    >
      <Icon name={settings.theme === 'auto' ? 'monitor' : settings.theme === 'light' ? 'sun' : 'moon'} size={17} />
      {settings.theme === 'auto' ? t().common.themeAuto : settings.theme === 'light' ? t().common.themeLight : t().common.themeDark}
    </button>
  </div>
</aside>

<style>
  .footer-actions {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .refresh-status {
    margin-top: auto;
    margin-bottom: 8px;
    padding: 6px 10px;
    font-size: 11px;
    color: var(--text-muted);
    background: var(--surface-2);
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .refresh-status.err {
    color: var(--negative);
    background: color-mix(in srgb, var(--negative) 10%, transparent);
  }
  .spinner {
    display: inline-block;
    width: 10px; height: 10px;
    border: 1.5px solid var(--text-muted);
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  .alert-badge {
    margin-left: auto;
    min-width: 18px; height: 18px; padding: 0 5px;
    border-radius: 9px;
    background: var(--negative);
    color: white;
    font-size: 10px; font-weight: 600;
    display: inline-flex; align-items: center; justify-content: center;
  }
  @media (max-width: 1023px) {
    .sidebar { display: none; }
  }
</style>
