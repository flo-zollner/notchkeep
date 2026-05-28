<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import { settings, t } from '$lib/settings.svelte';
  import { type Institution, type InstitutionSummary } from '$lib/api';
  import { fmtEur } from '$lib/format';

  interface Props {
    institution: Institution | InstitutionSummary;
    summary?: InstitutionSummary | null;
    onClick?: () => void;
    children?: Snippet;
  }
  let { institution, summary = null, onClick, children }: Props = $props();

  const ti = $derived(t().institutions);

  const iconName = $derived(institution.icon ?? 'bank');
  const color = $derived(institution.color ?? 'var(--accent)');

  // summary can come from the prop or from the institution itself if it already is an InstitutionSummary
  const resolved = $derived(summary ?? ('accountCount' in institution ? institution : null));
</script>

<div class="institution-card" class:clickable={!!onClick} style:--accent={color}>
  <button
    class="card-header"
    type="button"
    onclick={onClick}
    disabled={!onClick}
    aria-label={institution.name}
  >
    <span class="icon" style:background={color}>
      <Icon name={iconName} size={16} />
    </span>
    <span class="name">{institution.name}</span>
    {#if institution.country}
      <span class="country-badge">{institution.country}</span>
    {/if}
    {#if resolved}
      <span class="meta num">
        {ti.accountCount(resolved.accountCount)}
        <span class="sep">·</span>
        {fmtEur(resolved.balanceCents, { hide: settings.hide, decimals: 0 })}
      </span>
    {/if}
  </button>

  {#if children}
    <div class="card-body">
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .institution-card {
    display: flex;
    flex-direction: column;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
  }
  .institution-card.clickable:hover {
    border-color: var(--accent);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    background: none;
    border: 0;
    text-align: left;
    color: var(--text);
    font: inherit;
    width: 100%;
  }
  .card-header:not([disabled]) {
    cursor: pointer;
  }
  .card-header[disabled] {
    cursor: default;
  }

  .icon {
    flex-shrink: 0;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: #fff;
  }

  .name {
    flex: 1;
    font-size: 14px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .country-badge {
    font-size: 10px;
    font-family: monospace;
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--surface);
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .meta {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .sep {
    color: var(--text-faint);
    margin: 0 3px;
  }

  .card-body {
    border-top: 1px solid var(--border);
    padding: 8px 0;
  }
</style>
