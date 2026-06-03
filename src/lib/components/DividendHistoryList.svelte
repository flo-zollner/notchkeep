<script lang="ts">
  import { type DividendEntry } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import { settings } from '$lib/settings.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';

  interface Props {
    entries: DividendEntry[];
  }
  let { entries }: Props = $props();

  type Group = { year: number; total: number; rows: DividendEntry[] };

  const groups = $derived.by<Group[]>(() => {
    const map = new Map<number, Group>();
    for (const e of entries) {
      const y = Number(e.bookingDate.slice(0, 4));
      let g = map.get(y);
      if (!g) {
        g = { year: y, total: 0, rows: [] };
        map.set(y, g);
      }
      g.total += e.amountCents;
      g.rows.push(e);
    }
    return Array.from(map.values()).sort((a, b) => b.year - a.year);
  });
</script>

{#if entries.length === 0}
  <EmptyState icon="trending" title="Noch keine Dividenden" description="Hier erscheinen erhaltene Dividenden, sobald welche gebucht sind." />
{:else}
  {#each groups as g (g.year)}
    <div class="group">
      <div class="g-head">
        <strong>{g.year}</strong>
        <span class="num">{fmtEur(g.total, { hide: settings.hide, decimals: 2 })}</span>
      </div>
      <ul>
        {#each g.rows as e (e.txId)}
          <li>
            <span class="date">{e.bookingDate}</span>
            <span class="name">{e.securityName}</span>
            <span class="num amt">{fmtEur(e.amountCents, { hide: settings.hide, decimals: 2 })}</span>
          </li>
        {/each}
      </ul>
    </div>
  {/each}
{/if}

<style>
  .group {
    margin-bottom: 16px;
  }
  .g-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }
  .g-head strong {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 2px;
  }
  li {
    display: grid;
    grid-template-columns: 100px 1fr auto;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text);
    border-radius: 6px;
  }
  li:hover {
    background: var(--surface-2);
  }
  .date {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }
  .num {
    font-variant-numeric: tabular-nums;
  }
  .amt {
    color: var(--positive);
    font-weight: 500;
  }
</style>
