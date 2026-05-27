<script lang="ts">
  import { t } from '$lib/settings.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    title: string;
    sub?: string;
    children?: import('svelte').Snippet;
  }

  let { title, sub, children }: Props = $props();
</script>

<div class="topbar">
  <div>
    <h1>{title}</h1>
    {#if sub}<div class="sub">{sub}</div>{/if}
  </div>
</div>

<div class="card card-pad-lg soon">
  <Icon name="target" size={28} />
  <div class="soon-title">{t().common.soon}</div>
  {#if children}
    <div class="soon-body">{@render children()}</div>
  {/if}
</div>

<style>
  .soon {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px 22px;
    color: var(--text-muted);
    text-align: center;
  }
  .soon-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
  }
  .soon-body {
    font-size: 13px;
    max-width: 480px;
  }
</style>
