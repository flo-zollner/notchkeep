<script lang="ts">
  import {
    api,
    type Account,
    type Category,
    type Rule,
    type RuleCondition,
    type RuleCombinator,
    type MatchFieldId,
    type MatchOpId,
    type NewRulePayload, errMsg} from '$lib/api';
  import Icon from './Icon.svelte';
  import { t } from '$lib/settings.svelte';
  import Sheet from './Sheet.svelte';

  interface Props {
    onClose: () => void;
  }
  let { onClose }: Props = $props();

  // ─── Form-State: Conditions als UI-Variante mit getrennten Feldern. Beim
  // Save in serialisierte RuleCondition (value: string) umgewandelt.

  type CondForm = {
    field: MatchFieldId;
    op: MatchOpId;
    text: string;
    rangeMinEur: string;
    rangeMaxEur: string;
    amountEur: string;
    accountId: number | null;
  };

  function emptyCond(): CondForm {
    return {
      field: 'counterparty',
      op: 'contains',
      text: '',
      rangeMinEur: '',
      rangeMaxEur: '',
      amountEur: '',
      accountId: null,
    };
  }

  const OPS_BY_FIELD: Record<MatchFieldId, MatchOpId[]> = {
    counterparty: ['contains', 'equals', 'starts_with', 'ends_with', 'regex'],
    description: ['contains', 'equals', 'starts_with', 'ends_with', 'regex'],
    amount: ['range', 'equals'],
    account: ['equals'],
  };

  function fieldLabel(f: MatchFieldId): string {
    const c = t().common;
    return f === 'counterparty' ? c.fieldCounterparty
      : f === 'description' ? c.fieldDescription
      : f === 'amount' ? c.fieldAmount
      : c.fieldAccount;
  }
  function opLabel(o: MatchOpId): string {
    const c = t().common;
    return o === 'contains' ? c.opContains
      : o === 'equals' ? c.opEquals
      : o === 'starts_with' ? c.opStartsWith
      : o === 'ends_with' ? c.opEndsWith
      : o === 'regex' ? c.opRegex
      : c.opRange;
  }

  // ─── Encode/Decode zwischen API-Form (RuleCondition.value: string) und CondForm.

  function condToForm(c: RuleCondition, accounts: Account[]): CondForm {
    const f = emptyCond();
    f.field = c.field;
    f.op = c.op;
    if (c.field === 'amount' && c.op === 'range') {
      const [mi, ma] = c.value.split('..');
      f.rangeMinEur = mi ? (Number(mi) / 100).toFixed(2) : '';
      f.rangeMaxEur = ma ? (Number(ma) / 100).toFixed(2) : '';
    } else if (c.field === 'amount' && c.op === 'equals') {
      const n = Number(c.value);
      f.amountEur = Number.isFinite(n) ? (n / 100).toFixed(2) : '';
    } else if (c.field === 'account') {
      const id = Number(c.value);
      f.accountId = Number.isFinite(id) && accounts.some((a) => a.id === id) ? id : null;
    } else {
      f.text = c.value;
    }
    return f;
  }

  function eurStringToCents(s: string): number | null {
    const cleaned = s.replace(',', '.').trim();
    if (!cleaned) return null;
    const n = Number(cleaned);
    if (!Number.isFinite(n)) return null;
    return Math.round(n * 100);
  }

  /** Liefert die serialisierte Condition oder `null`, wenn unvollständig/ungültig. */
  function formToCond(f: CondForm): RuleCondition | null {
    if (f.field === 'amount' && f.op === 'range') {
      const mi = eurStringToCents(f.rangeMinEur);
      const ma = eurStringToCents(f.rangeMaxEur);
      if (mi === null || ma === null) return null;
      return { field: 'amount', op: 'range', value: `${mi}..${ma}` };
    }
    if (f.field === 'amount' && f.op === 'equals') {
      const cents = eurStringToCents(f.amountEur);
      if (cents === null) return null;
      return { field: 'amount', op: 'equals', value: String(cents) };
    }
    if (f.field === 'account') {
      if (f.accountId === null) return null;
      return { field: 'account', op: 'equals', value: String(f.accountId) };
    }
    if (!f.text.trim()) return null;
    return { field: f.field, op: f.op, value: f.text };
  }

  // ─── Top-level State

  let rules = $state<Rule[]>([]);
  let categories = $state<Category[]>([]);
  let accounts = $state<Account[]>([]);
  let loading = $state(true);
  let selectedId = $state<number | null>(null);
  let dirty = $state(false);
  let saving = $state(false);
  let confirmingDelete = $state(false);
  let error = $state<string | null>(null);
  let applyResult = $state<string | null>(null);

  // Form fields
  let formName = $state('');
  let formCombinator = $state<RuleCombinator>('and');
  let formConditions = $state<CondForm[]>([]);
  let formTargetCategoryId = $state<number | null>(null);
  let formPriority = $state(100);
  let formEnabled = $state(true);

  // Match-Preview
  let previewCount = $state<number | null>(null);
  let previewLoading = $state(false);
  let previewTimer: ReturnType<typeof setTimeout> | null = null;

  async function load() {
    loading = true;
    try {
      [rules, categories, accounts] = await Promise.all([
        api.listRules(),
        api.listCategories(),
        api.listAccounts(),
      ]);
    } finally {
      loading = false;
    }
  }
  load();

  function startNew() {
    selectedId = null;
    formName = '';
    formCombinator = 'and';
    formConditions = [emptyCond()];
    formTargetCategoryId = categories[0]?.id ?? null;
    formPriority = 100;
    formEnabled = true;
    confirmingDelete = false;
    error = null;
    applyResult = null;
    previewCount = null;
    dirty = true;
    schedulePreview();
  }

  function select(r: Rule) {
    selectedId = r.id;
    formName = r.name;
    formCombinator = r.combinator;
    formConditions =
      r.conditions.length > 0
        ? r.conditions.map((c) => condToForm(c, accounts))
        : [emptyCond()];
    formTargetCategoryId = r.targetCategoryId;
    formPriority = r.priority;
    formEnabled = r.enabled;
    confirmingDelete = false;
    error = null;
    applyResult = null;
    dirty = false;
    schedulePreview();
  }

  function markDirty() {
    dirty = true;
    applyResult = null;
    schedulePreview();
  }

  function setFieldOnCondition(i: number, field: MatchFieldId) {
    const c = formConditions[i];
    c.field = field;
    if (!OPS_BY_FIELD[field].includes(c.op)) {
      c.op = OPS_BY_FIELD[field][0];
    }
    markDirty();
  }

  function setOpOnCondition(i: number, op: MatchOpId) {
    formConditions[i].op = op;
    markDirty();
  }

  function addCondition() {
    formConditions.push(emptyCond());
    markDirty();
  }
  function removeCondition(i: number) {
    formConditions.splice(i, 1);
    if (formConditions.length === 0) formConditions.push(emptyCond());
    markDirty();
  }

  function buildPayload(): NewRulePayload | null {
    if (!formName.trim()) return null;
    if (formTargetCategoryId === null) return null;
    const conds: RuleCondition[] = [];
    for (const f of formConditions) {
      const c = formToCond(f);
      if (!c) return null;
      conds.push(c);
    }
    if (conds.length === 0) return null;
    return {
      priority: formPriority,
      name: formName.trim(),
      combinator: formCombinator,
      conditions: conds,
      targetCategoryId: formTargetCategoryId,
      enabled: formEnabled,
    };
  }

  function schedulePreview() {
    if (previewTimer !== null) clearTimeout(previewTimer);
    previewTimer = setTimeout(runPreview, 350);
  }

  async function runPreview() {
    const payload = buildPayload();
    if (!payload) {
      previewCount = null;
      return;
    }
    previewLoading = true;
    try {
      previewCount = await api.previewRuleMatch(payload);
    } catch {
      previewCount = null;
    } finally {
      previewLoading = false;
    }
  }

  async function save() {
    error = null;
    const payload = buildPayload();
    if (!payload) {
      error = t().common.condition;
      return;
    }
    saving = true;
    try {
      if (selectedId === null) {
        const created = await api.createRule(payload);
        rules = await api.listRules();
        select(created);
      } else {
        const updated: Rule = {
          id: selectedId,
          priority: payload.priority,
          name: payload.name,
          combinator: payload.combinator,
          conditions: payload.conditions,
          targetCategoryId: payload.targetCategoryId,
          enabled: payload.enabled,
        };
        const back = await api.updateRule(updated);
        rules = await api.listRules();
        select(back);
      }
      dirty = false;
    } catch (e) {
      error = errMsg(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (selectedId === null) return;
    if (!confirmingDelete) {
      confirmingDelete = true;
      return;
    }
    try {
      await api.deleteRule(selectedId);
      selectedId = null;
      dirty = false;
      previewCount = null;
      rules = await api.listRules();
    } catch (e) {
      error = errMsg(e);
    }
  }

  async function applyToExisting() {
    if (selectedId === null) return;
    applyResult = null;
    try {
      const n = await api.applyRuleToExisting(selectedId);
      applyResult = t().common.appliedToExisting(n);
    } catch (e) {
      error = errMsg(e);
    }
  }

  function categoryName(id: number | null | undefined): string {
    if (id === null || id === undefined) return '—';
    return categories.find((c) => c.id === id)?.name ?? '—';
  }


</script>

<Sheet open={true} {onClose} title={t().common.rulesModalTitle}>
    <div class="rules-grid">
      <!-- Liste -->
      <div class="list-col">
        {#if loading}
          <div class="muted small">…</div>
        {:else if rules.length === 0 && !dirty}
          <div class="empty-list muted small">{t().common.noRules}</div>
        {:else}
          <ul class="list">
            {#each rules as r (r.id)}
              <li>
                <button
                  class="list-item"
                  class:active={selectedId === r.id}
                  class:disabled={!r.enabled}
                  onclick={() => select(r)}
                >
                  <span class="row1">
                    <span class="rule-name">{r.name}</span>
                    <span class="comb-pill" class:or={r.combinator === 'or'}>
                      {r.combinator === 'or' ? 'ODER' : 'UND'}
                    </span>
                  </span>
                  <span class="row2 muted">
                    {r.conditions.length} {t().common.condition} ·
                    {categoryName(r.targetCategoryId)}
                  </span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        <button class="btn ghost full" onclick={startNew}>
          <Icon name="plus" size={12} /> {t().common.newRule}
        </button>
      </div>

      <!-- Editor -->
      <div class="edit-col">
        {#if !dirty && selectedId === null}
          <div class="empty">
            <div class="muted">{t().common.newRule} …</div>
          </div>
        {:else}
          <label class="field">
            <span class="field-label">{t().common.name}</span>
            <input
              class="input"
              type="text"
              bind:value={formName}
              oninput={markDirty}
              placeholder="z.B. REWE → Lebensmittel"
              autocomplete="off"
            />
          </label>

          <div class="row-3">
            <label class="field">
              <span class="field-label">{t().common.targetCategory}</span>
              <select
                class="input"
                bind:value={formTargetCategoryId}
                onchange={markDirty}
              >
                <option value={null} disabled>{t().common.pickCategory}</option>
                {#each categories as c (c.id)}
                  <option value={c.id}>{c.name}</option>
                {/each}
              </select>
            </label>
            <label class="field">
              <span class="field-label">{t().common.priority}</span>
              <input
                class="input"
                type="number"
                bind:value={formPriority}
                oninput={markDirty}
              />
            </label>
            <label class="field enabled-field">
              <span class="field-label">{t().common.enabled}</span>
              <button
                class="toggle"
                class:on={formEnabled}
                onclick={() => { formEnabled = !formEnabled; markDirty(); }}
                aria-pressed={formEnabled}
                aria-label={t().common.enabled}
                type="button"
              >
                <span class="knob" class:on={formEnabled}></span>
              </button>
            </label>
          </div>

          <div class="field">
            <span class="field-label">{t().common.condition}</span>
            <div class="seg comb-seg">
              <button
                class:on={formCombinator === 'and'}
                onclick={() => { formCombinator = 'and'; markDirty(); }}
                type="button"
              >{t().common.matchAll}</button>
              <button
                class:on={formCombinator === 'or'}
                onclick={() => { formCombinator = 'or'; markDirty(); }}
                type="button"
              >{t().common.matchAny}</button>
            </div>
          </div>

          <div class="conditions">
            {#each formConditions as cond, i (i)}
              <div class="cond-row">
                <select
                  class="input"
                  value={cond.field}
                  onchange={(e) =>
                    setFieldOnCondition(i, (e.currentTarget as HTMLSelectElement).value as MatchFieldId)}
                >
                  <option value="counterparty">{fieldLabel('counterparty')}</option>
                  <option value="description">{fieldLabel('description')}</option>
                  <option value="amount">{fieldLabel('amount')}</option>
                  <option value="account">{fieldLabel('account')}</option>
                </select>

                <select
                  class="input"
                  value={cond.op}
                  onchange={(e) =>
                    setOpOnCondition(i, (e.currentTarget as HTMLSelectElement).value as MatchOpId)}
                >
                  {#each OPS_BY_FIELD[cond.field] as op (op)}
                    <option value={op}>{opLabel(op)}</option>
                  {/each}
                </select>

                <div class="cond-value">
                  {#if cond.field === 'amount' && cond.op === 'range'}
                    <input
                      class="input"
                      type="text"
                      inputmode="decimal"
                      placeholder={t().common.rangeMin}
                      bind:value={cond.rangeMinEur}
                      oninput={markDirty}
                    />
                    <input
                      class="input"
                      type="text"
                      inputmode="decimal"
                      placeholder={t().common.rangeMax}
                      bind:value={cond.rangeMaxEur}
                      oninput={markDirty}
                    />
                  {:else if cond.field === 'amount' && cond.op === 'equals'}
                    <input
                      class="input"
                      type="text"
                      inputmode="decimal"
                      placeholder="€"
                      bind:value={cond.amountEur}
                      oninput={markDirty}
                    />
                  {:else if cond.field === 'account'}
                    <select
                      class="input"
                      bind:value={cond.accountId}
                      onchange={markDirty}
                    >
                      <option value={null} disabled>{t().common.pickAccount}</option>
                      {#each accounts as a (a.id)}
                        <option value={a.id}>{a.name}</option>
                      {/each}
                    </select>
                  {:else}
                    <input
                      class="input"
                      type="text"
                      bind:value={cond.text}
                      oninput={markDirty}
                      placeholder={cond.op === 'regex' ? '^REWE\\b' : '…'}
                    />
                  {/if}
                </div>

                <button
                  class="btn icon remove"
                  onclick={() => removeCondition(i)}
                  aria-label="−"
                  type="button"
                >
                  <Icon name="x" size={12} />
                </button>
              </div>
            {/each}
            <button class="btn ghost add-cond" onclick={addCondition} type="button">
              <Icon name="plus" size={12} /> {t().common.addCondition}
            </button>
          </div>

          <div class="preview">
            <span class="muted small">{t().common.matchPreview}:</span>
            <span class="preview-count">
              {#if previewLoading}…
              {:else if previewCount === null}—
              {:else}{t().common.matchCount(previewCount)}{/if}
            </span>
            {#if selectedId !== null && !dirty}
              <button class="btn apply" onclick={applyToExisting} type="button" title={t().common.applyToExistingDesc}>
                <Icon name="repeat" size={12} /> {t().common.applyToExisting}
              </button>
            {/if}
          </div>

          {#if applyResult}
            <div class="info">{applyResult}</div>
          {/if}
          {#if error}
            <div class="error">{error}</div>
          {/if}

          <div class="actions">
            {#if selectedId !== null}
              <button class="btn delete" onclick={remove} type="button">
                {confirmingDelete ? t().common.confirmDelete : t().common.delete}
              </button>
            {/if}
            <div class="spacer"></div>
            <button class="btn primary" onclick={save} disabled={saving || !dirty} type="button">
              {saving ? '…' : t().common.save}
            </button>
          </div>
        {/if}
      </div>
    </div>
</Sheet>

<style>
  .rules-grid {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 18px;
    min-height: 460px;
  }
  @media (max-width: 1023px) {
    .rules-grid {
      grid-template-columns: 1fr;
    }
  }
  .list-col {
    border-right: 1px solid var(--border);
    padding-right: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 68vh;
    overflow-y: auto;
  }
  @media (max-width: 1023px) {
    .list-col {
      border-right: none;
      border-bottom: 1px solid var(--border);
      padding-right: 0;
      padding-bottom: 14px;
    }
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .list-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    background: transparent;
    border: 1px solid transparent;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .list-item:hover {
    background: var(--surface-2);
  }
  .list-item.active {
    background: var(--accent-soft);
    border-color: var(--accent);
  }
  .list-item.disabled .rule-name {
    text-decoration: line-through;
    color: var(--text-muted);
  }
  .row1 {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
  }
  .rule-name {
    flex: 1;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .comb-pill {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 99px;
    background: var(--surface-2);
    color: var(--text-muted);
    font-weight: 600;
    letter-spacing: 0.04em;
  }
  .comb-pill.or {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .row2 {
    font-size: 11px;
  }
  .empty-list {
    padding: 20px 8px;
    text-align: center;
  }
  .btn.ghost {
    background: transparent;
    color: var(--text-muted);
    border: 1px dashed var(--border-strong);
  }
  .btn.ghost:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  .btn.full {
    justify-content: center;
    margin-top: 6px;
  }
  .edit-col {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .empty {
    display: grid;
    place-items: center;
    height: 100%;
    color: var(--text-muted);
    font-size: 13px;
  }
  .field {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .row-3 {
    display: grid;
    grid-template-columns: 1.4fr 0.8fr 0.8fr;
    gap: 10px;
  }
  .enabled-field {
    flex-direction: column;
    align-items: flex-start;
  }
  .comb-seg button {
    flex: 1;
  }
  .conditions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    padding: 10px;
  }
  .cond-row {
    display: grid;
    grid-template-columns: 130px 130px 1fr auto;
    gap: 6px;
    align-items: center;
  }
  @media (max-width: 599px) {
    .cond-row {
      grid-template-columns: 1fr 1fr;
    }
    .cond-row .cond-value {
      grid-column: 1 / -1;
    }
  }
  .cond-value {
    display: flex;
    gap: 6px;
    min-width: 0;
  }
  .cond-value > * {
    flex: 1;
    min-width: 0;
  }
  .btn.icon.remove {
    width: 28px;
    height: 28px;
    color: var(--text-muted);
  }
  .btn.icon.remove:hover {
    color: var(--negative);
    background: var(--negative-soft);
  }
  .add-cond {
    align-self: flex-start;
    margin-top: 2px;
  }
  .preview {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: var(--surface-2);
    border-radius: var(--r-sm);
  }
  .preview-count {
    flex: 1;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }
  .btn.apply {
    color: var(--accent);
  }
  .btn.apply:hover {
    background: var(--accent-soft);
  }
  .info {
    padding: 8px 10px;
    background: var(--accent-soft);
    color: var(--accent);
    border-radius: var(--r-sm);
    font-size: 12px;
  }
  .error {
    padding: 8px 10px;
    background: var(--negative-soft);
    color: var(--negative);
    border-radius: var(--r-sm);
    font-size: 12px;
  }
  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 6px;
  }
  .actions .spacer {
    flex: 1;
  }
  .btn.delete {
    color: var(--negative);
  }
  .btn.delete:hover:not(:disabled) {
    background: var(--negative-soft);
  }
  .toggle {
    width: 38px;
    height: 22px;
    border-radius: 99px;
    background: var(--border-strong);
    position: relative;
    padding: 0;
    transition: background 0.15s;
    border: 0;
    cursor: pointer;
    margin-top: 6px;
  }
  .toggle.on {
    background: var(--accent);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    transition: left 0.15s;
  }
  .knob.on {
    left: 18px;
  }
  .muted {
    color: var(--text-muted);
  }
  .small {
    font-size: 12px;
  }
</style>
