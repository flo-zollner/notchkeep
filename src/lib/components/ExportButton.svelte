<script lang="ts">
  import { save } from '@tauri-apps/plugin-dialog';
  import { api, type ExportFilter } from '$lib/api';
  import Icon from '$lib/components/Icon.svelte';
  import { t } from '$lib/settings.svelte';

  interface Props {
    getFilter: () => ExportFilter;
    label?: string;
    variant?: 'btn' | 'btn-primary';
  }

  let { getFilter, label, variant = 'btn' }: Props = $props();

  let busy = $state(false);
  let status = $state<{ kind: 'ok' | 'err'; text: string } | null>(null);

  function defaultName(): string {
    const d = new Date();
    const yyyy = d.getFullYear();
    const mm = String(d.getMonth() + 1).padStart(2, '0');
    const dd = String(d.getDate()).padStart(2, '0');
    return `budget_${yyyy}-${mm}-${dd}.csv`;
  }

  async function onClick() {
    if (busy) return;
    busy = true;
    status = null;
    try {
      const path = await save({
        defaultPath: defaultName(),
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (!path) {
        busy = false;
        return;
      }
      const result = await api.exportTransactionsCsv(getFilter(), path);
      status = { kind: 'ok', text: t().common.exportSuccess(result.rows) };
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      status = { kind: 'err', text: msg };
    } finally {
      busy = false;
    }
  }
</script>

<div class="export-wrap">
  <button class={variant} onclick={onClick} disabled={busy}>
    <Icon name="download" size={13} />
    {label ?? t().common.exportButton}
  </button>
  {#if status}
    <span class="status" class:err={status.kind === 'err'}>{status.text}</span>
  {/if}
</div>

<style>
  .export-wrap {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .status {
    font-size: 12px;
    color: var(--text-faint);
  }
  .status.err {
    color: var(--danger, #c33);
  }
</style>
