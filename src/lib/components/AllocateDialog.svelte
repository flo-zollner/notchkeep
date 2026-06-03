<script lang="ts">
  import { untrack } from 'svelte';
  import Sheet from './Sheet.svelte';
  import DateField from './DateField.svelte';
  import { t } from '$lib/settings.svelte';
  import { api, type Bucket, todayIso } from '$lib/api';
  import { fmtEur } from '$lib/format';
  import { parseEur } from '$lib/format';

  interface Props {
    bucket: Bucket;
    buckets: Bucket[];
    readyToAssignCents: number;
    onSaved: () => void;
    onClose: () => void;
  }
  let { bucket, buckets, readyToAssignCents, onSaved, onClose }: Props = $props();

  const tb = $derived(t().buckets);

  // null = "Unverteilt" (Ready to Assign); number = bucket id.
  // The clicked bucket is the default destination — money flows From -> To.
  let fromId = $state<number | null>(null);
  let toId = $state<number | null>(untrack(() => bucket.id));
  let amountStr = $state('');
  let note = $state('');
  let occurredOn = $state(todayIso());
  let saving = $state(false);
  let error = $state<string | null>(null);

  /** Amount in cents parsed from the input field. NaN if unparseable. */
  const amountCents = $derived.by(() => {
    const n = parseEur(amountStr.replace(',', '.'));
    return Number.isFinite(n) ? Math.round(n * 100) : NaN;
  });

  const validAmount = $derived(!Number.isNaN(amountCents) && amountCents > 0);
  const sameEndpoint = $derived(fromId === toId);

  // Effect on "Unverteilt": leaving it (from = null) lowers it, entering it (to = null) raises it.
  const rtaDelta = $derived(
    !validAmount ? 0 : fromId === null ? -amountCents : toId === null ? amountCents : 0,
  );
  const previewAfter = $derived(rtaDelta === 0 ? null : readyToAssignCents + rtaDelta);
  const wouldGoNegative = $derived(previewAfter !== null && previewAfter < 0);

  async function submit() {
    error = null;
    if (!validAmount) {
      error = tb.errAmountRequired;
      return;
    }
    if (sameEndpoint) {
      error = tb.errSameEndpoint;
      return;
    }
    const cents = amountCents;
    saving = true;
    try {
      if (fromId === null) {
        // Unverteilt -> bucket: positive allocation
        await api.createBucketAllocation({
          bucketId: toId!,
          amountCents: cents,
          occurredOn: occurredOn || null,
          note: note.trim() || null,
        });
      } else if (toId === null) {
        // bucket -> Unverteilt: negative allocation (release reservation)
        await api.createBucketAllocation({
          bucketId: fromId,
          amountCents: -cents,
          occurredOn: occurredOn || null,
          note: note.trim() || null,
        });
      } else {
        // bucket -> bucket
        await api.moveBetweenBuckets(fromId, toId, cents, occurredOn || undefined);
      }
      onSaved();
    } catch (e) {
      error = (e as Error).message ?? String(e);
    } finally {
      saving = false;
    }
  }
</script>

<Sheet open={true} {onClose} title={tb.assign}>
  {#snippet footer()}
    <div class="footer-actions">
      <button type="button" class="btn" onclick={onClose} disabled={saving}>
        {tb.cancel}
      </button>
      <button type="button" class="btn primary" onclick={submit} disabled={saving}>
        {tb.assign}
      </button>
    </div>
  {/snippet}

  <div class="grid">
    <label>
      <span>{tb.moveFrom}</span>
      <select bind:value={fromId}>
        <option value={null}>{tb.unassignedSource}</option>
        {#each buckets as b (b.id)}
          <option value={b.id}>{b.name}</option>
        {/each}
      </select>
    </label>

    <label>
      <span>{tb.moveTo}</span>
      <select bind:value={toId}>
        <option value={null}>{tb.unassignedSource}</option>
        {#each buckets as b (b.id)}
          <option value={b.id}>{b.name}</option>
        {/each}
      </select>
    </label>

    <label class="full">
      <span>{tb.amount}</span>
      <input
        bind:value={amountStr}
        type="text"
        inputmode="decimal"
        placeholder="0,00"
        autocomplete="off"
      />
    </label>

    <label class="full">
      <span>{tb.occurredOn}</span>
      <DateField bind:value={occurredOn} />
    </label>

    <label class="full">
      <span>{tb.allocationNote}</span>
      <input bind:value={note} type="text" placeholder={tb.allocationNote} />
    </label>
  </div>

  <p class="err" aria-live="polite">{#if sameEndpoint}{tb.errSameEndpoint}{/if}</p>

  {#if previewAfter !== null}
    <div class="preview" class:warn={wouldGoNegative}>
      <span>
        {tb.assignPreview
          .replace('{before}', fmtEur(readyToAssignCents))
          .replace('{after}', fmtEur(previewAfter))}
      </span>
      {#if wouldGoNegative}
        <p class="warn-text">{tb.assignWouldGoNegative}</p>
      {/if}
    </div>
  {/if}

  <p class="err" aria-live="polite">{#if error}{error}{/if}</p>
</Sheet>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 12px;
  }
  .full { grid-column: 1 / -1; }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--text-muted);
  }
  input, select {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    color: var(--text);
    font: inherit;
  }
  .preview {
    margin-top: 12px;
    padding: 8px 12px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    font-size: 12px;
    color: var(--text-muted);
  }
  .preview.warn {
    background: color-mix(in srgb, var(--negative) 12%, transparent);
    color: var(--negative);
  }
  .warn-text {
    margin: 4px 0 0 0;
    font-size: 11px;
  }
  .err {
    margin: 8px 0 0;
    font-size: 12px;
    color: var(--negative, #ef4444);
  }
  .err:empty { display: none; }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .btn {
    background: var(--surface-2);
    border: 1px solid var(--border);
    padding: 8px 12px;
    border-radius: 8px;
    cursor: pointer;
    color: var(--text);
    font: inherit;
  }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-fg, white);
    border: 0;
  }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  @media (max-width: 600px) {
    .footer-actions button { flex: 1 1 0; min-width: 0; }
  }
</style>
