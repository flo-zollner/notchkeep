<script lang="ts">
  import { api, type Category, errMsg} from '$lib/api';
  import Icon from './Icon.svelte';
  import { t } from '$lib/settings.svelte';
  import Sheet from './Sheet.svelte';

  interface Props {
    onClose: () => void;
  }
  let { onClose }: Props = $props();

  const ICON_CHOICES = [
    'utensils', 'home', 'car', 'plane',
    'film', 'heart', 'shield', 'repeat',
    'bag', 'briefcase', 'cart', 'bolt',
    'globe', 'target', 'wallet', 'tag',
  ];
  const COLOR_CHOICES = [
    '#10b981', '#0ea5e9', '#6366f1',
    '#f59e0b', '#ef4444', '#a855f7',
  ];

  let cats = $state<Category[]>([]);
  let loading = $state(true);
  let selectedId = $state<number | null>(null);
  let dirty = $state(false);
  let saving = $state(false);
  let confirmingDelete = $state(false);
  let merging = $state(false);
  let mergeTarget = $state<number>(0);
  let mergeBusy = $state(false);
  let mergeError = $state<string | null>(null);

  async function runMerge() {
    if (selectedId === null || mergeTarget === 0) return;
    mergeBusy = true;
    mergeError = null;
    try {
      await api.mergeCategories(selectedId, mergeTarget);
      await load();
      merging = false;
      mergeTarget = 0;
      selectedId = null;
    } catch (e) {
      mergeError = String(e);
    } finally {
      mergeBusy = false;
    }
  }
  let error = $state<string | null>(null);

  // Edit form fields (mirror of selected category, edited locally)
  let formName = $state('');
  let formIcon = $state<string | null>(null);
  let formColor = $state<string | null>(null);
  let formParentId = $state<number | null>(null);

  async function load() {
    loading = true;
    try {
      cats = await api.listCategories();
    } finally {
      loading = false;
    }
  }
  load();

  // Tree: parents first, then children grouped under them
  const tree = $derived.by(() => {
    const parents = cats.filter((c) => c.parent_id === null);
    return parents.map((p) => ({
      parent: p,
      children: cats.filter((c) => c.parent_id === p.id),
    }));
  });

  const orphanChildren = $derived(
    cats.filter(
      (c) => c.parent_id !== null && !cats.some((p) => p.id === c.parent_id)
    )
  );

  function startNew(parentId: number | null = null) {
    selectedId = null;
    formName = '';
    formIcon = 'tag';
    formColor = COLOR_CHOICES[0];
    formParentId = parentId;
    confirmingDelete = false;
    error = null;
    dirty = true;
  }

  function select(c: Category) {
    selectedId = c.id;
    formName = c.name;
    formIcon = c.icon ?? null;
    formColor = c.color ?? null;
    formParentId = c.parent_id;
    confirmingDelete = false;
    error = null;
    dirty = false;
  }

  async function save() {
    error = null;
    if (!formName.trim()) {
      error = t().common.name;
      return;
    }
    saving = true;
    try {
      if (selectedId === null) {
        const created = await api.createCategory({
          name: formName.trim(),
          parentId: formParentId,
          icon: formIcon,
          color: formColor,
          rolloverEnabled: false,
        });
        await load();
        select(created);
      } else {
        const existing = cats.find((c) => c.id === selectedId);
        if (!existing) return;
        await api.updateCategory({
          ...existing,
          name: formName.trim(),
          parent_id: formParentId,
          icon: formIcon,
          color: formColor,
        });
        await load();
        const refreshed = cats.find((c) => c.id === selectedId);
        if (refreshed) select(refreshed);
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
      await api.deleteCategory(selectedId);
      selectedId = null;
      dirty = false;
      await load();
    } catch (e) {
      error = errMsg(e);
    }
  }

  function markDirty() {
    dirty = true;
  }


</script>

<Sheet open={true} {onClose} title={t().common.catModalTitle}>
    <div class="cat-grid">
      <!-- Tree -->
      <div class="tree-col">
        {#if loading}
          <div class="muted small">…</div>
        {:else}
          <ul class="tree">
            {#each tree as node (node.parent.id)}
              <li>
                <button
                  class="tree-item"
                  class:active={selectedId === node.parent.id}
                  onclick={() => select(node.parent)}
                >
                  <span class="dot" style:background={node.parent.color ?? 'var(--text-muted)'}>
                    <Icon name={node.parent.icon || 'tag'} size={11} />
                  </span>
                  <span class="tree-name">{node.parent.name}</span>
                </button>
                {#if node.children.length > 0}
                  <ul class="children">
                    {#each node.children as child (child.id)}
                      <li>
                        <button
                          class="tree-item child"
                          class:active={selectedId === child.id}
                          onclick={() => select(child)}
                        >
                          <span class="dot small" style:background={child.color ?? 'var(--text-muted)'}>
                            <Icon name={child.icon || 'tag'} size={9} />
                          </span>
                          <span class="tree-name">{child.name}</span>
                        </button>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </li>
            {/each}
            {#each orphanChildren as c (c.id)}
              <li>
                <button
                  class="tree-item"
                  class:active={selectedId === c.id}
                  onclick={() => select(c)}
                >
                  <span class="dot" style:background={c.color ?? 'var(--text-muted)'}>
                    <Icon name={c.icon || 'tag'} size={11} />
                  </span>
                  <span class="tree-name">{c.name}</span>
                </button>
              </li>
            {/each}
          </ul>
          <button class="btn ghost full" onclick={() => startNew(null)}>
            <Icon name="plus" size={12} /> {t().common.newCategory}
          </button>
        {/if}
      </div>

      <!-- Editor -->
      <div class="edit-col">
        {#if !dirty && selectedId === null}
          <div class="empty">
            <div class="muted">{t().common.newCategory} …</div>
          </div>
        {:else}
          <label class="field">
            <span class="field-label">{t().common.name}</span>
            <input
              class="input"
              type="text"
              bind:value={formName}
              oninput={markDirty}
              placeholder="z.B. Wochenmarkt"
              autocomplete="off"
            />
          </label>

          <div class="field">
            <span class="field-label">{t().common.icon}</span>
            <div class="icon-grid">
              {#each ICON_CHOICES as ic (ic)}
                <button
                  class="icon-swatch"
                  class:on={formIcon === ic}
                  onclick={() => { formIcon = ic; markDirty(); }}
                  aria-label={ic}
                >
                  <Icon name={ic} size={14} />
                </button>
              {/each}
            </div>
          </div>

          <div class="field">
            <span class="field-label">{t().common.color}</span>
            <div class="color-row">
              {#each COLOR_CHOICES as col (col)}
                <button
                  class="color-swatch"
                  class:on={formColor === col}
                  style:background={col}
                  onclick={() => { formColor = col; markDirty(); }}
                  aria-label={col}
                ></button>
              {/each}
            </div>
          </div>

          <label class="field">
            <span class="field-label">{t().common.parent}</span>
            <select
              class="input"
              bind:value={formParentId}
              onchange={markDirty}
            >
              <option value={null}>{t().common.topLevel}</option>
              {#each cats.filter((c) => c.parent_id === null && c.id !== selectedId) as p (p.id)}
                <option value={p.id}>{p.name}</option>
              {/each}
            </select>
          </label>

          {#if error}
            <div class="error">{error}</div>
          {/if}

          <div class="actions">
            {#if selectedId !== null}
              <button class="btn delete" onclick={remove}>
                {confirmingDelete ? t().common.confirmDelete : t().common.delete}
              </button>
              <button class="btn" type="button" onclick={() => (merging = true)}>
                ⇄ Merge
              </button>
            {/if}
            <div class="spacer"></div>
            <button class="btn primary" onclick={save} disabled={saving || !dirty}>
              {saving ? '…' : t().common.save}
            </button>
          </div>

          {#if merging && selectedId !== null}
            <div class="merge-box">
              <p class="muted">
                Wähle eine Ziel-Kategorie. Alle Transaktionen + Regeln + Sub-Kategorien
                wandern dorthin. Diese Kategorie wird danach gelöscht.
              </p>
              <select bind:value={mergeTarget}>
                <option value={0}>— wählen —</option>
                {#each cats.filter((c) => c.id !== selectedId) as c (c.id)}
                  <option value={c.id}>{c.name}</option>
                {/each}
              </select>
              {#if mergeError}<p class="error">{mergeError}</p>{/if}
              <div style="display: flex; gap: 8px; margin-top: 6px;">
                <button class="btn ghost" type="button" onclick={() => { merging = false; mergeTarget = 0; mergeError = null; }}>
                  Abbrechen
                </button>
                <button class="btn primary" type="button" onclick={runMerge} disabled={mergeBusy || mergeTarget === 0}>
                  {mergeBusy ? 'Merge läuft…' : 'Mergen'}
                </button>
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>
</Sheet>

<style>
  .cat-grid {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 18px;
    min-height: 380px;
  }
  .tree-col {
    border-right: 1px solid var(--border);
    padding-right: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 60vh;
    overflow-y: auto;
  }
  @media (max-width: 1023px) {
    .cat-grid {
      grid-template-columns: 1fr;
    }
    .tree-col {
      border-right: none;
      border-bottom: 1px solid var(--border);
      padding-right: 0;
      padding-bottom: 14px;
    }
  }
  .tree {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .children {
    list-style: none;
    padding-left: 18px;
    margin: 2px 0 4px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .tree-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    border-radius: var(--r-sm);
    background: transparent;
    border: 0;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .tree-item:hover {
    background: var(--surface-2);
  }
  .tree-item.active {
    background: var(--accent-soft);
    color: var(--text);
  }
  .tree-item .dot {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: white;
    flex-shrink: 0;
  }
  .tree-item .dot.small {
    width: 16px;
    height: 16px;
  }
  .tree-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }
  .tree-item.child .tree-name {
    font-size: 12px;
    color: var(--text-muted);
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
  .icon-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 4px;
  }
  .icon-swatch {
    display: grid;
    place-items: center;
    aspect-ratio: 1;
    background: var(--surface-2);
    border: 1px solid transparent;
    border-radius: var(--r-sm);
    cursor: pointer;
    color: var(--text-muted);
  }
  .icon-swatch:hover {
    color: var(--text);
  }
  .icon-swatch.on {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .color-row {
    display: flex;
    gap: 8px;
  }
  .color-swatch {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    padding: 0;
  }
  .color-swatch.on {
    border-color: var(--text);
    box-shadow: 0 0 0 2px var(--surface);
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
  .merge-box {
    margin-top: 8px;
    padding: 8px 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
  }
  .merge-box select {
    width: 100%;
    background: var(--surface); border: 1px solid var(--border); border-radius: 4px;
    padding: 4px 8px; font: inherit; color: var(--text); margin-top: 6px;
  }
  .btn.delete {
    color: var(--negative);
  }
  .btn.delete:hover:not(:disabled) {
    background: var(--negative-soft);
  }
  .muted {
    color: var(--text-muted);
  }
  .small {
    font-size: 12px;
  }
</style>
