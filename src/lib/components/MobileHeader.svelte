<script lang="ts">
  import { page } from '$app/state';
  import { t } from '$lib/settings.svelte';
  import { navMain, navManage } from '$lib/nav-config';

  /** Page-Titel: erstes Match aus navMain/navManage, sonst Brand-Name */
  const title = $derived.by(() => {
    const all = [...navMain, ...navManage];
    const path = page.url.pathname;
    const hit = all.find((n) => n.href === path || (n.href !== '/' && path.startsWith(n.href + '/')));
    return hit ? t().nav[hit.labelKey] : t().appName;
  });
</script>

<header class="mobile-header">
  <div class="brand-dot">S</div>
  <div class="title">{title}</div>
</header>

<style>
  .mobile-header {
    display: none;
    position: sticky; top: 0; z-index: 5;
    padding: 12px 16px;
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
</style>
