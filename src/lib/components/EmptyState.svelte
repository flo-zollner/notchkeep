<script lang="ts">
  import Icon from './Icon.svelte';

  /**
   * Consistent empty state (UX guide §19): icon + one-line reason + optional
   * primary action. Use type to keep wording honest:
   *  - 'new'    → nothing created yet (offer the create action)
   *  - 'filter' → a filter/search hid everything (offer "reset filter")
   *  - 'error'  → loading failed (offer "retry")
   */
  interface Props {
    icon?: string;
    title: string;
    description?: string;
    actionLabel?: string;
    onAction?: () => void;
  }
  let { icon = 'tag', title, description, actionLabel, onAction }: Props = $props();
</script>

<div class="empty-state">
  <div class="empty-icon"><Icon name={icon} size={28} aria-hidden="true" /></div>
  <p class="empty-title">{title}</p>
  {#if description}<p class="empty-desc">{description}</p>{/if}
  {#if actionLabel && onAction}
    <button type="button" class="btn primary empty-action" onclick={onAction}>{actionLabel}</button>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
    padding: 48px 24px;
    color: var(--text-muted);
  }
  .empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 56px;
    height: 56px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-faint);
    margin-bottom: 4px;
  }
  .empty-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    margin: 0;
  }
  .empty-desc {
    font-size: 13px;
    max-width: 38ch;
    margin: 0;
  }
  .empty-action {
    margin-top: 8px;
  }
</style>
