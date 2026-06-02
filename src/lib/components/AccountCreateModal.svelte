<script lang="ts">
  import { api, listInstitutionsWithSummary, type InstitutionSummary, errMsg } from '$lib/api';
  import { t } from '$lib/settings.svelte';
  import Sheet from './Sheet.svelte';

  interface Props {
    open: boolean;
    onClose: () => void;
    /** Called after an account was created successfully. */
    onCreated?: () => void;
  }
  let { open, onClose, onCreated }: Props = $props();

  let name = $state('');
  let kind = $state('bank');
  let institutionId = $state<number | null>(null);
  let institutions = $state<InstitutionSummary[]>([]);
  let saving = $state(false);
  let error = $state<string | null>(null);

  /** Kinds that should not carry a bank institution. */
  const NON_INSTITUTION_KINDS = new Set(['cash', 'loan']);
  const KINDS = ['bank', 'broker', 'savings', 'credit', 'cash', 'loan'] as const;

  function kindLabel(k: string): string {
    const tx = t().common as unknown as Record<string, string | undefined>;
    return tx[`kind${k.charAt(0).toUpperCase() + k.slice(1)}`] ?? k;
  }

  $effect(() => {
    if (NON_INSTITUTION_KINDS.has(kind)) institutionId = null;
  });

  // Load institutions once the dialog opens; reset the form on each open.
  $effect(() => {
    if (open) {
      name = '';
      kind = 'bank';
      institutionId = null;
      error = null;
      void listInstitutionsWithSummary().then((list) => (institutions = list));
    }
  });

  async function submit() {
    if (!name.trim() || saving) return;
    saving = true;
    error = null;
    try {
      await api.createAccount(name.trim(), kind, undefined, null, null, institutionId);
      saving = false;
      onCreated?.();
      onClose();
    } catch (e) {
      error = errMsg(e);
      saving = false;
    }
  }
</script>

<Sheet {open} {onClose} title={t().common.addAccount}>
  <form
    class="acc-form"
    onsubmit={(e) => {
      e.preventDefault();
      void submit();
    }}
  >
    {#if error}
      <div class="form-error">{error}</div>
    {/if}
    <div class="field">
      <div class="field-label">{t().common.name}</div>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="input" bind:value={name} placeholder="z.B. TR Verrechnung" required autofocus />
    </div>
    <div class="field">
      <div class="field-label">{t().common.type}</div>
      <select class="input" bind:value={kind}>
        {#each KINDS as k (k)}
          <option value={k}>{kindLabel(k)}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <div class="field-label">{t().common.institution}</div>
      <select
        class="input"
        value={institutionId === null ? '' : String(institutionId)}
        onchange={(e) => {
          const v = (e.currentTarget as HTMLSelectElement).value;
          institutionId = v === '' ? null : Number(v);
        }}
        disabled={NON_INSTITUTION_KINDS.has(kind)}
        title={NON_INSTITUTION_KINDS.has(kind) ? t().common.institutionNone : undefined}
      >
        <option value="">{t().common.institutionNone}</option>
        {#each institutions as inst (inst.id)}
          <option value={String(inst.id)}>{inst.name}</option>
        {/each}
      </select>
    </div>
    <!-- Hidden submit so Enter submits the form; the visible action is the footer button. -->
    <button type="submit" class="visually-hidden" aria-hidden="true" tabindex="-1"></button>
  </form>

  {#snippet footer()}
    <div class="footer-actions">
      <button type="button" class="btn" onclick={onClose}>{t().common.cancel}</button>
      <button type="button" class="btn accent" disabled={!name.trim() || saving} onclick={submit}>
        {t().common.save}
      </button>
    </div>
  {/snippet}
</Sheet>

<style>
  .acc-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .form-error {
    color: var(--negative);
    font-size: 13px;
  }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    border: 0;
  }
  @media (max-width: 599px) {
    .footer-actions .btn {
      flex: 1;
      min-height: 44px;
    }
  }
</style>
