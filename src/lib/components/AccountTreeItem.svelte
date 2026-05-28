<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import type { Account } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import { settings } from '$lib/settings.svelte';
  import AccountTreeItem from './AccountTreeItem.svelte';

  interface Props {
    node: Account;
    children: Account[];
    childrenByParent: Map<number | null, Account[]>;
    balances: Map<number, number>;
    trends?: Map<number, number[]>;  // categoryId → 6-month net cents array (ASC)
    depth?: number;
    onOpen: (id: number) => void;
    onEdit?: (account: Account) => void;
    onImport?: (id: number) => void;
    importingId?: number | null;
  }
  let {
    node,
    children,
    childrenByParent,
    balances,
    trends,
    depth = 0,
    onOpen,
    onEdit,
    onImport,
    importingId = null,
  }: Props = $props();

  const trendSeries = $derived(trends?.get(node.id) ?? []);
</script>

<div
  class="row"
  class:archived={node.archived}
  style:padding-left={`${12 + depth * 16}px`}
  role="button"
  tabindex="0"
  onclick={() => onOpen(node.id)}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onOpen(node.id);
    }
  }}
>
  <span class="icon" style:color={node.color ?? 'inherit'}>
    <Icon name={node.icon ?? 'wallet'} size={16} />
  </span>
  <span class="name">
    {node.name}
    {#if node.last4}<span class="last4">·· {node.last4}</span>{/if}
  </span>
  {#if depth === 0 && trendSeries.length >= 2}
    {@const max = Math.max(...trendSeries.map(Math.abs), 1)}
    <svg class="trend" viewBox="0 0 60 18" preserveAspectRatio="none" aria-hidden="true">
      {#each trendSeries as v, i (i)}
        {@const x = (60 / Math.max(trendSeries.length - 1, 1)) * i}
        {@const y = 9 - (v / max) * 7}
        {#if i > 0}
          {@const px = (60 / Math.max(trendSeries.length - 1, 1)) * (i - 1)}
          {@const pv = trendSeries[i - 1]}
          {@const py = 9 - (pv / max) * 7}
          <line x1={px} y1={py} x2={x} y2={y}
            stroke="var(--text-muted)" stroke-width="1" fill="none"
            vector-effect="non-scaling-stroke" />
        {/if}
        <circle cx={x} cy={y} r="1" fill={v >= 0 ? 'var(--positive)' : 'var(--negative)'} />
      {/each}
    </svg>
  {/if}
  <span class="balance num">{fmtEur(balances.get(node.id) ?? 0, { hide: settings.hide })}</span>
  {#if onEdit || onImport}
    <span class="actions" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="toolbar" tabindex="-1">
      {#if onEdit}
        <button
          class="btn icon-btn"
          type="button"
          onclick={() => onEdit?.(node)}
          aria-label="edit"
          title="Bearbeiten"
        >
          <Icon name="pencil" size={13} />
        </button>
      {/if}
      {#if onImport}
        <button
          class="btn icon-btn"
          type="button"
          disabled={importingId === node.id}
          onclick={() => onImport?.(node.id)}
          aria-label="CSV"
          title="CSV"
        >
          <Icon name={importingId === node.id ? 'refresh' : 'download'} size={13} />
        </button>
      {/if}
    </span>
  {/if}
</div>

{#each children as child (child.id)}
  <AccountTreeItem
    node={child}
    children={childrenByParent.get(child.id) ?? []}
    {childrenByParent}
    {balances}
    {trends}
    depth={depth + 1}
    {onOpen}
    {onEdit}
    {onImport}
    {importingId}
  />
{/each}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    cursor: pointer;
    border-radius: 8px;
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row.archived {
    opacity: 0.55;
  }
  .icon {
    display: inline-grid;
    place-items: center;
    width: 18px;
    flex-shrink: 0;
  }
  .name {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .last4 {
    font-family: var(--font-mono);
    color: var(--text-faint);
    font-size: 11.5px;
    margin-left: 6px;
    font-weight: 400;
  }
  .trend {
    width: 60px; height: 18px;
    margin-right: 8px;
    opacity: 0.6;
  }
  .balance {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 13.5px;
  }
  .actions {
    display: flex;
    gap: 6px;
    margin-left: 4px;
  }
  .icon-btn {
    font-size: 11.5px;
    padding: 4px 8px;
  }
</style>
