<script lang="ts">
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import DateField from './DateField.svelte';
  import { t } from '$lib/settings.svelte';
  import {
    api,
    type Bucket,
    type NewBucketPayload,
    type UpdateBucketPayload,
  } from '$lib/api';
  import { fmtEurInput, parseEur } from '$lib/format';

  interface Props {
    bucket: Bucket | null;
    onClose: () => void;
    onSaved: () => void;
    onDeleted?: (id: number) => void;
  }
  let { bucket, onClose, onSaved, onDeleted }: Props = $props();

  const tb = $derived(t().buckets);
  const tc = $derived(t().common);

  const ICONS = ['wallet', 'piggy', 'star', 'goal', 'target', 'plane', 'home', 'heart', 'briefcase', 'shield'];
  const COLORS = [
    'var(--c1)',
    'var(--c2)',
    'var(--c3)',
    'var(--c4)',
    'var(--c5)',
    'var(--c6)',
  ];

  const isEdit = $derived(bucket !== null);

  // Form state initialised from prop once at mount; modal is conditionally
  // mounted by parent, so prop reactivity is not desired.
  /* svelte-ignore state_referenced_locally */
  let name = $state(bucket?.name ?? '');
  /* svelte-ignore state_referenced_locally */
  let icon = $state<string | null>(bucket?.icon ?? null);
  /* svelte-ignore state_referenced_locally */
  let color = $state<string | null>(bucket?.color ?? null);
  /* svelte-ignore state_referenced_locally */
  let note = $state(bucket?.note ?? '');
  /* svelte-ignore state_referenced_locally */
  let targetEur = $state(
    bucket?.targetCents != null ? fmtEurInput(bucket.targetCents) : '',
  );
  /* svelte-ignore state_referenced_locally */
  let startDate = $state(bucket?.startDate ?? '');
  /* svelte-ignore state_referenced_locally */
  let targetDate = $state(bucket?.targetDate ?? '');
  /* svelte-ignore state_referenced_locally */
  let archived = $state(bucket?.archived ?? false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let confirmingDelete = $state(false);

  async function save() {
    error = null;
    if (!name.trim()) {
      error = tb.errNameRequired;
      return;
    }

    let targetCents: number | null = null;
    const targetTrimmed = targetEur.trim();
    if (targetTrimmed !== '') {
      const parsed = Math.round(parseEur(targetTrimmed) * 100);
      if (!Number.isFinite(parsed) || parsed < 0) {
        error = tb.errTargetInvalid;
        return;
      }
      targetCents = parsed;
    }

    if (targetDate && startDate && targetDate < startDate) {
      error = tb.errDateOrder;
      return;
    }

    saving = true;
    try {
      if (isEdit && bucket) {
        const payload: UpdateBucketPayload = {
          name: name.trim(),
          icon,
          color,
          note: note.trim() || null,
          targetCents,
          startDate: startDate || null,
          targetDate: targetDate || null,
          archived,
        };
        await api.updateBucket(bucket.id, payload);
      } else {
        const payload: NewBucketPayload = {
          name: name.trim(),
          icon,
          color,
          note: note.trim() || null,
          targetCents,
          startDate: startDate || null,
          targetDate: targetDate || null,
        };
        await api.createBucket(payload);
      }
      onSaved();
      onClose();
    } catch (e) {
      error = (e as Error).message ?? String(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!bucket) return;
    saving = true;
    try {
      await api.deleteBucket(bucket.id);
      onDeleted?.(bucket.id);
      onSaved();
      onClose();
    } catch (e) {
      error = (e as Error).message ?? String(e);
      confirmingDelete = false;
    } finally {
      saving = false;
    }
  }
</script>

<Sheet open={true} {onClose} title={isEdit ? tb.edit : tb.add}>
  {#snippet footer()}
    <div class="footer-actions">
      {#if confirmingDelete}
        <div class="confirm-row">
          <span class="confirm-msg">{tb.confirmDelete}</span>
          <div class="spacer"></div>
          <button type="button" onclick={() => (confirmingDelete = false)} disabled={saving}>
            {tc.cancel}
          </button>
          <button type="button" class="btn danger" onclick={remove} disabled={saving}>
            {tb.delete}
          </button>
        </div>
      {:else}
        {#if isEdit}
          <button
            type="button"
            class="btn danger"
            onclick={() => (confirmingDelete = true)}
            disabled={saving}
          >
            {tb.delete}
          </button>
        {/if}
        <button type="button" class="btn" onclick={onClose} disabled={saving}>{tc.cancel}</button>
        <button type="button" class="btn primary" onclick={save} disabled={saving}>{tc.save}</button>
      {/if}
    </div>
  {/snippet}

  <div class="grid">
    <label class="full">
      <span>{tc.name}</span>
      <input bind:value={name} type="text" />
    </label>

    <label>
      <span>{tb.targetCents}</span>
      <input bind:value={targetEur} type="text" inputmode="decimal" placeholder="0,00" />
    </label>

    <label>
      <span>{tb.startDate}</span>
      <DateField bind:value={startDate} />
    </label>

    <label>
      <span>{tb.targetDate}</span>
      <DateField bind:value={targetDate} />
    </label>

    <div class="full">
      <span class="picker-label">{tb.pickIcon}</span>
      <div class="picker">
        {#each ICONS as ic (ic)}
          <button
            type="button"
            class="picker-btn"
            class:on={icon === ic}
            onclick={() => (icon = ic)}
            aria-label={ic}
          >
            <Icon name={ic} size={14} />
          </button>
        {/each}
        <button
          type="button"
          class="picker-btn"
          class:on={icon === null}
          onclick={() => (icon = null)}
          aria-label={tb.pickIcon}
        >–</button>
      </div>
    </div>

    <div class="full">
      <span class="picker-label">{tb.pickColor}</span>
      <div class="picker">
        {#each COLORS as c (c)}
          <button
            type="button"
            class="swatch"
            class:on={color === c}
            style:background={c}
            onclick={() => (color = c)}
            aria-label={c}
          ></button>
        {/each}
        <button
          type="button"
          class="swatch swatch-empty"
          class:on={color === null}
          onclick={() => (color = null)}
          aria-label={tb.pickColor}
        >–</button>
      </div>
    </div>

    <label class="full">
      <span>{tb.note}</span>
      <textarea bind:value={note} rows="2"></textarea>
    </label>

    {#if isEdit}
      <label class="full check">
        <input bind:checked={archived} type="checkbox" />
        <span>{tb.archived}</span>
      </label>
    {/if}
  </div>

  <p class="err" aria-live="polite">{#if error}{error}{/if}</p>
</Sheet>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 12px;
  }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--text-muted); }
  .full { grid-column: 1 / -1; }
  label.check { flex-direction: row; align-items: center; gap: 8px; }
  input, textarea {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    color: var(--text);
    font: inherit;
  }
  .picker-label {
    font-size: 12px;
    color: var(--text-muted);
    display: block;
    margin-bottom: 4px;
  }
  .picker { display: flex; gap: 6px; flex-wrap: wrap; }
  .picker-btn {
    width: 32px; height: 32px;
    border-radius: 8px;
    background: var(--surface-2);
    border: 1px solid transparent;
    color: var(--text-muted);
    display: grid; place-items: center;
    cursor: pointer;
  }
  .picker-btn.on { border-color: var(--accent); color: var(--text); }
  .swatch {
    width: 26px; height: 26px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    padding: 0;
  }
  .swatch.on { border-color: var(--text); }
  .swatch-empty {
    background: var(--surface-2);
    color: var(--text-faint);
    display: grid; place-items: center;
    font-size: 12px;
  }
  .err { margin: 0; font-size: 12px; color: var(--danger, #ef4444); }
  .err:empty { display: none; }
  /* footer-actions */
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: nowrap;
  }
  .footer-actions .btn.danger { margin-right: auto; background: var(--negative-soft); color: var(--negative); }
  @media (max-width: 600px) {
    .footer-actions button { flex: 1 1 0; min-width: 0; }
    .footer-actions .btn.danger { flex: 0 0 auto; margin-right: auto; }
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
  .btn.primary { background: var(--accent); color: var(--accent-fg, white); border: 0; }
  .btn.primary:disabled { opacity: .5; cursor: not-allowed; }
  .btn:disabled { opacity: .5; cursor: not-allowed; }
  .confirm-row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 10px 12px;
    background: var(--negative-soft, var(--surface-2));
    border: 1px solid var(--border);
    border-radius: 10px;
  }
  .confirm-msg {
    font-size: 13px;
    color: var(--text);
  }
  .spacer { flex: 1; }
</style>
