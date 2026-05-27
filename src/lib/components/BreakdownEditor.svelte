<script lang="ts">
  import Icon from './Icon.svelte';
  import { t } from '$lib/settings.svelte';
  import { parseEur, type BreakdownRowInput } from '$lib/api';

  interface Props {
    rows: BreakdownRowInput[];
    onChange: (rows: BreakdownRowInput[]) => void;
  }
  let { rows, onChange }: Props = $props();

  const tb = $derived(t().breakdown);

  const sumBps = $derived(rows.reduce((s, r) => s + (r.weightBps || 0), 0));
  const isSumOk = $derived(rows.length === 0 || (sumBps >= 9950 && sumBps <= 10050));
  const sumPct = $derived((sumBps / 100).toFixed(2));

  function updateRow(idx: number, patch: Partial<BreakdownRowInput>) {
    const next = rows.map((r, i) => (i === idx ? { ...r, ...patch } : r));
    onChange(next);
  }

  function addRow() {
    onChange([...rows, { key: '', weightBps: 0 }]);
  }

  function removeRow(idx: number) {
    onChange(rows.filter((_, i) => i !== idx));
  }

  function parsePct(s: string): number {
    const n = parseEur(s);
    if (!Number.isFinite(n)) return 0;
    return Math.round(n * 100);
  }
  function fmtPct(bps: number): string {
    return (bps / 100).toFixed(2);
  }
</script>

<div class="bre">
  <table>
    <thead>
      <tr><th>{tb.key}</th><th>{tb.weight}</th><th></th></tr>
    </thead>
    <tbody>
      {#each rows as row, idx (idx)}
        <tr>
          <td>
            <input
              type="text"
              value={row.key}
              oninput={(e) => updateRow(idx, { key: (e.target as HTMLInputElement).value })}
              placeholder="US / DE / Technology / …"
            />
          </td>
          <td>
            <input
              type="text"
              inputmode="decimal"
              value={fmtPct(row.weightBps)}
              oninput={(e) =>
                updateRow(idx, { weightBps: parsePct((e.target as HTMLInputElement).value) })}
            />
          </td>
          <td>
            <button type="button" class="icon" onclick={() => removeRow(idx)} title={tb.removeRow}>
              <Icon name="x" size={14} />
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <div class="footer">
    <button type="button" class="ghost" onclick={addRow}>
      <Icon name="plus" size={14} /> {tb.addRow}
    </button>
    <span class="sum" class:ok={isSumOk} class:off={!isSumOk}>
      {tb.sum}: {sumPct}%
      {#if rows.length > 0}
        {isSumOk ? tb.sumOk : tb.sumOff}
      {/if}
    </span>
  </div>
</div>

<style>
  .bre table { width: 100%; border-collapse: collapse; }
  .bre th, .bre td { padding: 4px 6px; text-align: left; }
  .bre th { font-size: 11px; color: var(--text-muted); font-weight: 500; }
  .bre input[type="text"] {
    width: 100%;
    box-sizing: border-box;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text);
    font: inherit;
  }
  .bre input[type="text"]:focus {
    outline: none;
    border-color: var(--accent);
  }
  .bre button.icon {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
  }
  .bre button.icon:hover { color: var(--negative); background: var(--negative-soft); }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 10px;
    gap: 12px;
  }
  .footer button.ghost {
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 6px 10px;
    border-radius: 8px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .footer button.ghost:hover { background: var(--surface-hover); }
  .sum { font-size: 12px; color: var(--text-muted); }
  .sum.ok { color: var(--positive); }
  .sum.off { color: var(--negative); }
</style>
