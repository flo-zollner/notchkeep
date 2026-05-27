<script lang="ts">
  import Icon from './Icon.svelte';
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    value: string;
    delta?: number | null;
    pct?: number | null;
    inverted?: boolean;
    sub?: string;
    title?: string;
    pill?: Snippet;
  }

  let { label, value, delta = null, pct = null, inverted = false, sub, title, pill }: Props = $props();

  const positive = $derived(
    delta === null ? false : inverted ? (delta as number) < 0 : (delta as number) > 0
  );
</script>

<div class="card kpi" title={title ?? null}>
  <div class="label">{label}</div>
  <div class="v num">{value}</div>
  {#if sub}<div class="sub">{sub}</div>{/if}
  {#if pill}{@render pill()}{/if}
  {#if delta !== null && delta !== undefined && pct !== null}
    <div class="d" class:up={positive} class:down={!positive}>
      <Icon name={positive ? 'arrow-up' : 'arrow-down'} size={11} />
      <span class="num">{Math.abs(pct).toFixed(1)}%</span>
    </div>
  {/if}
</div>

<style>
  .sub {
    font-size: 11px;
    color: var(--text-faint);
    margin-top: 2px;
  }
</style>
