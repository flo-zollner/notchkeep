<script lang="ts">
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import { t } from '$lib/settings.svelte';
  import {
    createInstitution,
    updateInstitution,
    deleteInstitution,
    type Institution,
    type NewInstitutionPayload,
  } from '$lib/api';

  interface Props {
    institution: Institution | null;
    onClose: () => void;
    onSaved: (inst: Institution) => void;
  }
  let { institution, onClose, onSaved }: Props = $props();

  const ti = $derived((t() as Record<string, any>).institutions ?? {});
  const tc = $derived(t().common);

  const ICONS = ['bank', 'card', 'piggy', 'wallet', 'briefcase', 'home', 'star'];
  const COLORS = [
    'var(--c1)',
    'var(--c2)',
    'var(--c3)',
    'var(--c4)',
    'var(--c5)',
    'var(--c6)',
  ];

  const isEdit = $derived(institution !== null);

  // Form state initialised from prop once at mount; modal is conditionally
  // mounted by parent, so prop reactivity is not desired.
  /* svelte-ignore state_referenced_locally */
  let name = $state(institution?.name ?? '');
  /* svelte-ignore state_referenced_locally */
  let icon = $state<string | null>(institution?.icon ?? null);
  /* svelte-ignore state_referenced_locally */
  let color = $state<string | null>(institution?.color ?? null);
  /* svelte-ignore state_referenced_locally */
  let bic = $state(institution?.bic ?? '');
  /* svelte-ignore state_referenced_locally */
  let country = $state(institution?.country ?? '');
  /* svelte-ignore state_referenced_locally */
  let note = $state(institution?.note ?? '');
  /* svelte-ignore state_referenced_locally */
  let archived = $state(institution?.archived ?? false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let confirmingDelete = $state(false);

  function mapError(msg: string): string {
    const m = msg.toLowerCase();
    if (m.includes('unique') && m.includes('name')) return ti.duplicateName ?? msg;
    if (m.includes('unique') && m.includes('bic')) return ti.duplicateBic ?? msg;
    if (m.includes('bic')) return tc.invalidBic ?? msg;
    if (m.includes('country')) return tc.invalidCountry ?? msg;
    return msg;
  }

  async function save() {
    error = null;
    if (!name.trim()) {
      error = tc.name + ' erforderlich';
      return;
    }

    saving = true;
    try {
      const payload: NewInstitutionPayload = {
        name: name.trim(),
        icon: icon ?? null,
        color: color ?? null,
        bic: bic.trim() || null,
        country: country.trim().toUpperCase() || null,
        note: note.trim() || null,
      };

      let result: Institution;
      if (isEdit && institution) {
        result = await updateInstitution(institution.id, { ...payload, archived });
      } else {
        result = await createInstitution(payload);
      }
      onSaved(result);
      onClose();
    } catch (e) {
      error = mapError((e as Error).message ?? String(e));
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!institution) return;
    saving = true;
    try {
      await deleteInstitution(institution.id);
      onSaved(institution);
      onClose();
    } catch (e) {
      const msg = (e as Error).message ?? String(e);
      const match = msg.match(/institution-has-accounts:(\d+)/);
      if (match) {
        error = ti.hasAccountsError(Number(match[1]));
      } else {
        error = msg;
      }
      confirmingDelete = false;
    } finally {
      saving = false;
    }
  }
</script>

<Sheet open={true} {onClose} title={isEdit ? (ti.edit ?? 'Institut bearbeiten') : (ti.add ?? 'Institut hinzufügen')}>
  {#snippet footer()}
    <div class="footer-actions">
      {#if confirmingDelete}
        <div class="confirm-row">
          <span class="confirm-msg">{ti.confirmDelete ?? 'Institut wirklich löschen?'}</span>
          <div class="spacer"></div>
          <button type="button" class="btn" onclick={() => (confirmingDelete = false)} disabled={saving}>
            {tc.cancel}
          </button>
          <button type="button" class="btn danger" onclick={remove} disabled={saving}>
            {ti.delete ?? tc.delete}
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
            {ti.delete ?? tc.delete}
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
      <span>{tc.bic}</span>
      <input bind:value={bic} type="text" class="mono" placeholder="AAAABBCC" />
    </label>

    <label>
      <span>{tc.country}</span>
      <input
        bind:value={country}
        type="text"
        maxlength="2"
        placeholder="DE"
        oninput={(e) => {
          const el = e.currentTarget as HTMLInputElement;
          el.value = el.value.toUpperCase();
          country = el.value;
        }}
      />
    </label>

    <div class="full">
      <span class="picker-label">{ti.icon ?? 'Icon'}</span>
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
          aria-label="Kein Icon"
        >–</button>
      </div>
    </div>

    <div class="full">
      <span class="picker-label">{ti.color ?? 'Farbe'}</span>
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
          aria-label="Keine Farbe"
        >–</button>
      </div>
    </div>

    <label class="full">
      <span>{ti.note ?? 'Notiz'}</span>
      <textarea bind:value={note} rows="2"></textarea>
    </label>

    {#if isEdit}
      <label class="full check">
        <input bind:checked={archived} type="checkbox" />
        <span>{ti.archive ?? 'Archivieren'}</span>
      </label>
    {/if}
  </div>

  {#if error}
    <p class="err">{error}</p>
  {/if}
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
  .mono { font-family: var(--font-mono, monospace); letter-spacing: 0.05em; }
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
  /* footer-actions */
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }
  .footer-actions .btn.danger { margin-right: auto; background: var(--negative-soft); color: var(--negative); }
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
