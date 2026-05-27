<script lang="ts">
  import Icon from './Icon.svelte';
  import { api } from '$lib/api';
  import { t } from '$lib/settings.svelte';

  interface Props {
    onRefreshed?: () => void;
  }
  let { onRefreshed }: Props = $props();

  let busy = $state(false);
  let status = $state<{ msg: string; err: boolean } | null>(null);

  async function refresh() {
    busy = true;
    status = null;
    try {
      const r = await api.refreshPrices();
      status = {
        msg: t().portfolio.refreshSuccess.replace('{n}', String(r.pricesUpdated)),
        err: r.pricesFailed > 0,
      };
      onRefreshed?.();
    } catch (e) {
      status = { msg: t().portfolio.refreshFailed + ': ' + String(e), err: true };
    } finally {
      busy = false;
    }
  }
</script>

<button class="ref" type="button" disabled={busy} onclick={refresh}>
  <Icon name="repeat" size={13} />
  {busy ? t().portfolio.refreshBusy : t().portfolio.refreshButton}
</button>
{#if status}
  <span class="status" class:err={status.err}>{status.msg}</span>
{/if}

<style>
  .ref {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font: inherit;
    color: var(--text);
  }
  .ref:hover { background: var(--surface-hover); }
  .ref:disabled { opacity: 0.6; cursor: not-allowed; }
  .status {
    margin-left: 8px;
    font-size: 12px;
    color: var(--text-muted);
  }
  .status.err { color: var(--negative); }
</style>
