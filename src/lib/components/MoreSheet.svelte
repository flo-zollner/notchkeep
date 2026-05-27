<script lang="ts">
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import { moreItems, isActive } from '$lib/nav-config';
  import { settings, setTheme, setHide, t } from '$lib/settings.svelte';
  import { page } from '$app/state';

  interface Props {
    open: boolean;
    onClose: () => void;
  }
  let { open, onClose }: Props = $props();
</script>

<Sheet {open} {onClose} title={t().sections.main}>
  <div class="more-list">
    {#each moreItems as item (item.id)}
      <a
        href={item.href}
        class="more-item"
        class:active={isActive(page.url.pathname, item.href)}
        onclick={onClose}
      >
        <Icon name={item.icon} size={20} />
        <span>{t().nav[item.labelKey]}</span>
      </a>
    {/each}
  </div>

  <div class="more-divider"></div>

  <div class="more-list">
    <button class="more-item" onclick={() => setHide(!settings.hide)}>
      <Icon name={settings.hide ? 'eye-off' : 'eye'} size={20} />
      <span>{t().common.hideAmounts}</span>
    </button>
    <button class="more-item" onclick={() => setTheme(settings.theme === 'dark' ? 'light' : 'dark')}>
      <Icon name={settings.theme === 'dark' ? 'sun' : 'moon'} size={20} />
      <span>{settings.theme === 'dark' ? 'Light' : 'Dark'}</span>
    </button>
  </div>
</Sheet>

<style>
  .more-list { display: flex; flex-direction: column; gap: 2px; }
  .more-item {
    display: flex; align-items: center; gap: 14px;
    padding: 12px 8px;
    min-height: var(--tap);
    border-radius: var(--r-sm);
    color: var(--text);
    text-decoration: none;
    font-size: 15px;
    background: none; border: none; text-align: left; width: 100%;
  }
  .more-item.active { background: var(--surface-2); color: var(--accent); }
  @media (hover: hover) {
    .more-item:hover { background: var(--surface-2); }
  }
  .more-divider {
    height: 1px;
    background: var(--border);
    margin: 12px 0;
  }
</style>
