<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { errMsg, listInstitutions, type Account, type Institution } from '$lib/api';
  import { t } from '$lib/settings.svelte';
  import InstitutionModal from './InstitutionModal.svelte';

  // i18n lookup without typed property access — missing keys fall back to the default via ??.
  const tx = () => t().common as unknown as Record<string, string | undefined>;

  interface Props {
    account: Account;
    onSave: (a: Account) => void | Promise<void>;
    onCancel?: () => void;
    accounts?: Account[];
  }
  let { account, onSave, onCancel, accounts = [] }: Props = $props();

  // Local working copy so that edits only update parent state on save.
  /* svelte-ignore state_referenced_locally */
  let draft = $state<Account>({ ...account });
  let saving = $state(false);
  let error = $state<string | null>(null);

  // Institution dropdown
  let institutions = $state<Institution[]>([]);
  let showInstModal = $state(false);

  async function refreshInstitutions() {
    const visible = await listInstitutions(false);
    institutions = visible;
    // If the account references an archived institution not in the visible list:
    if (draft.institution_id != null && !visible.find((i) => i.id === draft.institution_id)) {
      institutions = await listInstitutions(true);
    }
  }

  $effect(() => { refreshInstitutions(); });

  function onInstitutionCreated(inst: Institution) {
    draft.institution_id = inst.id;
    refreshInstitutions();
    showInstModal = false;
  }

  function handleInstitutionSelect(value: string) {
    if (value === '__create__') {
      showInstModal = true;
      // Dropdown value stays on the current institution_id on the next render
    } else if (value === '') {
      draft.institution_id = null;
    } else {
      draft.institution_id = Number(value);
    }
  }

  function descendantsAndSelf(id: number, all: Account[]): Set<number> {
    const result = new Set<number>([id]);
    let frontier = [id];
    while (frontier.length > 0) {
      const next: number[] = [];
      for (const a of all) {
        if (a.parent_id != null && frontier.includes(a.parent_id) && !result.has(a.id)) {
          result.add(a.id);
          next.push(a.id);
        }
      }
      frontier = next;
    }
    return result;
  }

  const excludedIds = $derived(() =>
    draft.id ? descendantsAndSelf(draft.id, accounts) : new Set<number>(),
  );

  const ICONS = ['bank', 'card', 'piggy', 'wallet', 'briefcase', 'home', 'star'];
  const COLORS = [
    'var(--c1)',
    'var(--c2)',
    'var(--c3)',
    'var(--c4)',
    'var(--c5)',
    'var(--c6)',
  ];

  function setLast4(raw: string) {
    const cleaned = raw.replace(/\D/g, '').slice(0, 4);
    draft.last4 = cleaned.length === 0 ? null : cleaned;
  }

  async function save() {
    if (draft.last4 !== null && !/^\d{4}$/.test(draft.last4)) {
      error = tx().invalidLast4 ?? 'Last4 must be exactly 4 digits';
      return;
    }
    if (draft.iban !== null && !/^[A-Z]{2}\d{2}[A-Z0-9]{11,30}$/.test(draft.iban)) {
      error = tx().invalidIban ?? 'IBAN ungültig (z.B. DE89 3704 …)';
      return;
    }
    saving = true;
    error = null;
    try {
      await onSave(draft);
    } catch (e) {
      error = errMsg(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="form">
  <div class="row">
    <label class="field">
      <span class="field-label">{t().common.name}</span>
      <input class="input" bind:value={draft.name} />
    </label>
    <label class="field">
      <span class="field-label">{t().common.type}</span>
      <select class="input" bind:value={draft.kind}>
        <option value="bank">Bank</option>
        <option value="broker">Depot</option>
        <option value="savings">Tagesgeld</option>
        <option value="credit">Kreditkarte</option>
        <option value="cash">{tx().kindCash ?? 'Bargeld'}</option>
        <option value="loan">{tx().kindLoan ?? 'Schuld / Darlehen'}</option>
      </select>
    </label>
  </div>

  <label class="field">
    <span class="field-label">{tx().parentAccount ?? 'Eltern-Konto'}</span>
    <select
      class="input"
      value={draft.parent_id ?? ''}
      onchange={(e) => {
        const v = (e.target as HTMLSelectElement).value;
        draft.parent_id = v === '' ? null : Number(v);
      }}
    >
      <option value="">{tx().topLevelDash ?? '— Top-Level —'}</option>
      {#each accounts.filter((a) => !excludedIds().has(a.id)) as a (a.id)}
        <option value={a.id}>{a.name}</option>
      {/each}
    </select>
  </label>

  <label class="field">
    <span class="field-label">{tx().institution ?? 'Institut'}</span>
    <select
      class="input"
      value={draft.institution_id === null ? '' : String(draft.institution_id)}
      onchange={(e) => handleInstitutionSelect((e.currentTarget as HTMLSelectElement).value)}
    >
      <option value="">{tx().institutionNone ?? '— Kein Institut —'}</option>
      {#each institutions as inst (inst.id)}
        <option value={String(inst.id)}>
          {inst.name}{inst.archived ? ' (archiviert)' : ''}
        </option>
      {/each}
      <option value="__create__">{(t() as Record<string, any>).institutions?.createNew ?? '+ Neues Institut anlegen…'}</option>
    </select>
  </label>

  {#if showInstModal}
    <InstitutionModal
      institution={null}
      onClose={() => (showInstModal = false)}
      onSaved={onInstitutionCreated}
    />
  {/if}

  <div class="row">
    <div class="field">
      <span class="field-label">{t().common.icon ?? 'Icon'}</span>
      <div class="picker">
        {#each ICONS as ic (ic)}
          <button
            type="button"
            class="picker-btn"
            class:on={draft.icon === ic}
            onclick={() => (draft.icon = ic)}
            aria-label={ic}
          >
            <Icon name={ic} size={14} />
          </button>
        {/each}
        <button
          type="button"
          class="picker-btn"
          class:on={draft.icon === null}
          onclick={() => (draft.icon = null)}
          aria-label="kein Icon"
        >–</button>
      </div>
    </div>
  </div>

  <div class="row">
    <div class="field">
      <span class="field-label">{t().common.color ?? 'Farbe'}</span>
      <div class="picker">
        {#each COLORS as c (c)}
          <button
            type="button"
            class="swatch"
            class:on={draft.color === c}
            style:background={c}
            onclick={() => (draft.color = c)}
            aria-label={c}
          ></button>
        {/each}
        <button
          type="button"
          class="swatch swatch-empty"
          class:on={draft.color === null}
          onclick={() => (draft.color = null)}
          aria-label="keine Farbe"
        >–</button>
      </div>
    </div>
  </div>

  <label class="field">
    <span class="field-label">{tx().last4 ?? 'Letzte 4 Stellen'}</span>
    <input
      class="input num"
      inputmode="numeric"
      maxlength="4"
      placeholder="z. B. 4321"
      value={draft.last4 ?? ''}
      oninput={(e) => setLast4((e.target as HTMLInputElement).value)}
    />
  </label>

  <label class="field">
    <span class="field-label">{tx().iban ?? 'IBAN'}</span>
    <input
      class="input mono"
      placeholder="DE89 3704 0044 0532 0130 00"
      value={draft.iban ?? ''}
      oninput={(e) => {
        const raw = (e.target as HTMLInputElement).value;
        const cleaned = raw.replace(/\s+/g, '').toUpperCase();
        draft.iban = cleaned.length === 0 ? null : cleaned;
      }}
    />
  </label>

  <label class="field">
    <span class="field-label">{tx().note ?? 'Notiz'}</span>
    <textarea
      class="input"
      rows="2"
      value={draft.note ?? ''}
      oninput={(e) => {
        const v = (e.target as HTMLTextAreaElement).value;
        draft.note = v.length === 0 ? null : v;
      }}
    ></textarea>
  </label>

  <label class="toggle">
    <input type="checkbox" bind:checked={draft.archived} />
    <span>{tx().archived ?? 'Archiviert (in Dropdowns ausblenden)'}</span>
  </label>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="actions">
    {#if onCancel}
      <button type="button" class="btn" onclick={onCancel} disabled={saving}>
        {t().common.cancel}
      </button>
    {/if}
    <button type="button" class="btn accent" onclick={save} disabled={saving}>
      {saving ? '…' : t().common.save}
    </button>
  </div>
</div>

<style>
  .form { display: flex; flex-direction: column; gap: 14px; }
  .row { display: flex; gap: 12px; flex-wrap: wrap; }
  .field { display: flex; flex-direction: column; flex: 1 1 200px; }
  .field-label {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-muted);
    margin-bottom: 5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
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
  }
  .swatch.on { border-color: var(--text); }
  .swatch-empty {
    background: var(--surface-2);
    color: var(--text-faint);
    display: grid; place-items: center;
    font-size: 12px;
  }
  .toggle { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .error { color: var(--negative); font-size: 12px; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
</style>
