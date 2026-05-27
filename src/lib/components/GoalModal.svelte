<script lang="ts">
  import Icon from './Icon.svelte';
  import Sheet from './Sheet.svelte';
  import DateField from './DateField.svelte';
  import { t } from '$lib/settings.svelte';
  import {
    api,
    parseEur,
    fmtEurInput,
    type Goal,
    type Category,
    type NewGoalPayload,
    type UpdateGoalPayload,
  } from '$lib/api';

  interface Props {
    goal?: Goal;
    categories: Category[];
    onClose: () => void;
    onSaved: (g: Goal) => void;
    onDeleted?: (id: number) => void;
  }
  let { goal, categories, onClose, onSaved, onDeleted }: Props = $props();

  const tg = $derived(t().goals);
  const tc = $derived(t().common);

  const ICONS = ['goal', 'target', 'star', 'piggy', 'plane', 'home', 'briefcase', 'heart'];
  const COLORS = [
    'var(--c1)',
    'var(--c2)',
    'var(--c3)',
    'var(--c4)',
    'var(--c5)',
    'var(--c6)',
  ];

  // Form state initialised from prop once at mount; modal is conditionally
  // mounted by parent, so prop reactivity is not desired.
  /* svelte-ignore state_referenced_locally */
  let name = $state(goal?.name ?? '');
  /* svelte-ignore state_referenced_locally */
  let categoryId = $state<number | null>(goal?.categoryId ?? null);
  /* svelte-ignore state_referenced_locally */
  let targetEur = $state(goal ? fmtEurInput(goal.targetCents) : '');
  /* svelte-ignore state_referenced_locally */
  let startDate = $state(goal?.startDate ?? new Date().toISOString().slice(0, 10));
  /* svelte-ignore state_referenced_locally */
  let targetDate = $state(goal?.targetDate ?? '');
  /* svelte-ignore state_referenced_locally */
  let icon = $state<string | null>(goal?.icon ?? null);
  /* svelte-ignore state_referenced_locally */
  let color = $state<string | null>(goal?.color ?? null);
  /* svelte-ignore state_referenced_locally */
  let note = $state(goal?.note ?? '');
  /* svelte-ignore state_referenced_locally */
  let archived = $state(goal?.archived ?? false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let confirmingDelete = $state(false);

  const isEdit = $derived(!!goal);

  // Tree-Aufbau: Top-Level-Kategorien mit ihren Kindern gruppiert.
  // Waisen (parent_id zeigt auf nicht-existente Kategorie) hängen unten dran.
  const catTree = $derived.by(() => {
    const parents = categories.filter((c) => c.parent_id === null);
    return parents.map((p) => ({
      parent: p,
      children: categories
        .filter((c) => c.parent_id === p.id)
        .sort((a, b) => a.name.localeCompare(b.name)),
    }));
  });
  const orphans = $derived(
    categories.filter(
      (c) => c.parent_id !== null && !categories.some((p) => p.id === c.parent_id),
    ),
  );

  // Falls categories später nachrutschen oder das Modal vor dem Load geöffnet
  // wurde, einen sinnvollen Default für categoryId setzen.
  $effect(() => {
    if (categoryId === null && categories.length > 0 && !isEdit) {
      categoryId = categories[0].id;
    }
  });

  async function save() {
    error = null;
    if (!name.trim()) { error = tg.errNameRequired; return; }
    if (categoryId === null) { error = tg.errCategoryRequired; return; }
    const target = Math.round(parseEur(targetEur) * 100);
    if (!Number.isFinite(target) || target <= 0) {
      error = tg.errTargetInvalid;
      return;
    }
    if (targetDate && targetDate < startDate) { error = tg.errDateOrder; return; }

    saving = true;
    try {
      let saved: Goal;
      if (isEdit && goal) {
        const payload: UpdateGoalPayload = {
          name,
          categoryId,
          targetCents: target,
          startDate,
          targetDate: targetDate || null,
          icon,
          color,
          note: note || null,
          archived,
        };
        saved = await api.updateGoal(goal.id, payload);
      } else {
        const payload: NewGoalPayload = {
          name,
          categoryId,
          targetCents: target,
          startDate,
          targetDate: targetDate || null,
          icon,
          color,
          note: note || null,
        };
        saved = await api.createGoal(payload);
      }
      onSaved(saved);
      onClose();
    } catch (e) {
      error = (e as Error).message ?? String(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!goal) return;
    saving = true;
    try {
      await api.deleteGoal(goal.id);
      onDeleted?.(goal.id);
      onClose();
    } catch (e) {
      error = (e as Error).message ?? String(e);
      confirmingDelete = false;
    } finally {
      saving = false;
    }
  }
</script>

<Sheet open={true} {onClose} title={isEdit ? tg.edit : tg.new}>
  {#snippet footer()}
    <div class="footer-actions">
      {#if confirmingDelete}
        <div class="confirm-row">
          <span class="confirm-msg">{tg.confirmDelete}</span>
          <div class="spacer"></div>
          <button type="button" class="btn" onclick={() => (confirmingDelete = false)} disabled={saving}>
            {tc.cancel}
          </button>
          <button type="button" class="btn danger" onclick={remove} disabled={saving}>
            {tg.delete}
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
            {tg.delete}
          </button>
        {/if}
        <button type="button" class="btn" onclick={onClose} disabled={saving}>{tc.cancel}</button>
        <button
          type="button"
          class="btn primary"
          onclick={save}
          disabled={saving || (categories.length === 0 && !isEdit)}
        >{tc.save}</button>
      {/if}
    </div>
  {/snippet}

  {#if categories.length === 0 && !isEdit}
    <p class="hint">{tg.noCategories}</p>
  {/if}

  <div class="grid">
    <label class="full">
      <span>{tc.name}</span>
      <input bind:value={name} type="text" />
    </label>

    <div class="full">
      <span class="picker-label">{tg.category}</span>
      {#if categories.length === 0}
        <p class="hint-inline">{tg.noCategories}</p>
      {:else}
        <div class="cat-tree">
          {#each catTree as group (group.parent.id)}
            <div class="cat-group">
              <button
                type="button"
                class="cat-chip"
                class:on={categoryId === group.parent.id}
                onclick={() => (categoryId = group.parent.id)}
              >
                <span class="cat-dot" style:color={group.parent.color || 'var(--text-muted)'}>
                  <Icon name={group.parent.icon || 'tag'} size={12} />
                </span>
                <span class="cat-name">{group.parent.name}</span>
              </button>
              {#if group.children.length > 0}
                <span class="cat-arrow">›</span>
                {#each group.children as child (child.id)}
                  <button
                    type="button"
                    class="cat-chip child"
                    class:on={categoryId === child.id}
                    onclick={() => (categoryId = child.id)}
                  >
                    <span class="cat-dot" style:color={child.color || group.parent.color || 'var(--text-muted)'}>
                      <Icon name={child.icon || group.parent.icon || 'tag'} size={11} />
                    </span>
                    <span class="cat-name">{child.name}</span>
                  </button>
                {/each}
              {/if}
            </div>
          {/each}
          {#if orphans.length > 0}
            <div class="cat-group">
              {#each orphans as c (c.id)}
                <button
                  type="button"
                  class="cat-chip"
                  class:on={categoryId === c.id}
                  onclick={() => (categoryId = c.id)}
                >
                  <span class="cat-dot" style:color={c.color || 'var(--text-muted)'}>
                    <Icon name={c.icon || 'tag'} size={12} />
                  </span>
                  <span class="cat-name">{c.name}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <label>
      <span>{tg.target}</span>
      <input bind:value={targetEur} type="text" inputmode="decimal" placeholder="0,00" />
    </label>

    <label>
      <span>{tg.startDate}</span>
      <DateField bind:value={startDate} />
    </label>

    <label>
      <span>{tg.targetDate}</span>
      <DateField bind:value={targetDate} />
    </label>

    <div class="full">
      <span class="picker-label">{tg.pickIcon}</span>
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
          aria-label={tg.noIcon}
        >–</button>
      </div>
    </div>

    <div class="full">
      <span class="picker-label">{tg.pickColor}</span>
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
          aria-label={tg.noColor}
        >–</button>
      </div>
    </div>

    <label class="full">
      <span>{tg.note}</span>
      <textarea bind:value={note} rows="2"></textarea>
    </label>

    {#if isEdit}
      <label class="full check">
        <input bind:checked={archived} type="checkbox" />
        <span>{tg.archived}</span>
      </label>
    {/if}
  </div>

  {#if error}
    <p class="err">{error}</p>
  {/if}
</Sheet>

<style>
  .hint {
    margin: 0;
    padding: 10px 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-muted);
  }
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
  .cat-tree { display: flex; flex-direction: column; gap: 8px; }
  .cat-group {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }
  .cat-arrow {
    color: var(--text-faint);
    font-size: 14px;
    margin: 0 2px 0 4px;
    line-height: 1;
  }
  .cat-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px 5px 6px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    line-height: 1;
  }
  .cat-chip:hover { background: var(--surface-hover); }
  .cat-chip.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .cat-chip.child {
    font-size: 11.5px;
    padding: 4px 9px 4px 5px;
    color: var(--text-muted);
  }
  .cat-chip.child.on { color: var(--text); }
  .cat-chip.child .cat-dot { width: 16px; height: 16px; }
  .cat-dot {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--surface);
    display: grid;
    place-items: center;
  }
  .cat-name { padding-right: 2px; }
  .hint-inline { margin: 0; font-size: 12px; color: var(--text-muted); }
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
