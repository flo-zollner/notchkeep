<script lang="ts">
  import { page } from '$app/state';
  import { t } from '$lib/settings.svelte';
  import { navMain, navManage } from '$lib/nav-config';

  /** Page title: first match from navMain/navManage, otherwise brand name. */
  const title = $derived.by(() => {
    const all = [...navMain, ...navManage];
    const path = page.url.pathname;
    const hit = all.find((n) => n.href === path || (n.href !== '/' && path.startsWith(n.href + '/')));
    return hit ? t().nav[hit.labelKey] : t().appName;
  });
</script>

<header class="mobile-header">
  <div class="brand-dot">N</div>
  <div class="title">{title}</div>
</header>

<style>
  .mobile-header {
    display: none;
    position: sticky; top: 0; z-index: 5;
    padding: calc(12px + env(safe-area-inset-top)) 16px 12px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    align-items: center;
    gap: 10px;
    font-weight: 600;
  }
  .brand-dot {
    width: 22px; height: 22px;
    border-radius: 7px;
    background: linear-gradient(135deg, var(--accent), oklch(0.65 0.16 200));
    display: grid; place-items: center;
    color: var(--accent-fg);
    font-size: 12px; font-weight: 700;
  }
  .title {
    font-size: 16px;
    letter-spacing: -0.02em;
    flex: 1;
  }
  @media (max-width: 599px) {
    .mobile-header { display: flex; }
  }

  /* ── Material Design 3 Small Top App Bar (Android) ── */
  :global(html[data-platform='android']) .mobile-header {
    background: var(--bg);
    border-bottom: none;
    min-height: 64px;
    padding: calc(env(safe-area-inset-top) + 10px) 16px 10px;
  }
  :global(html[data-platform='android']) .mobile-header .title {
    font-size: 22px;
    font-weight: 500;
    letter-spacing: 0;
  }
</style>
