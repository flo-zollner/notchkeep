<script lang="ts">
  import Icon from './Icon.svelte';
  import { settings, t } from '$lib/settings.svelte';
  import type { Goal, GoalProgress, Category } from '$lib/api';

  interface Props {
    goal: Goal;
    progress: GoalProgress;
    category?: Category;
    onEdit?: () => void;
  }
  let { goal, progress, category, onEdit }: Props = $props();

  const tg = $derived(t().goals);

  function fmtEur(cents: number) {
    return settings.hide
      ? '••• €'
      : (cents / 100).toLocaleString(settings.lang, {
          style: 'currency',
          currency: 'EUR',
          maximumFractionDigits: 0,
        });
  }

  const ratio = $derived(Math.max(0, Math.min(1, progress.currentCents / goal.targetCents)));
  const reached = $derived(progress.currentCents >= goal.targetCents);

  const iconName = $derived(goal.icon ?? category?.icon ?? 'goal');
  const color = $derived(goal.color ?? category?.color ?? 'var(--accent)');

  function fmtMonth(yyyymmdd: string) {
    const [y, m] = yyyymmdd.split('-');
    const i = Number(m) - 1;
    return `${t().months[i]} ${y}`;
  }

  const forecastLabel = $derived.by(() => {
    if (!progress.forecastDate) return '—';
    if (reached) return tg.reached;
    return `${tg.forecast} ${fmtMonth(progress.forecastDate)}`;
  });
</script>

<article class="goal-card">
  <header>
    <span class="icon" style:color>
      <Icon name={iconName} size={16} />
    </span>
    <h4>{goal.name}</h4>
    {#if onEdit}
      <button class="edit" type="button" onclick={onEdit} aria-label={tg.edit}>
        <Icon name="pencil" size={14} />
      </button>
    {/if}
  </header>
  <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax={goal.targetCents} aria-valuenow={progress.currentCents}>
    <div class="fill" class:reached style:width={`${ratio * 100}%`} style:background={color}></div>
  </div>
  <div class="amounts num">
    <span>{fmtEur(progress.currentCents)}</span>
    <span class="sep">/</span>
    <span class="target">{fmtEur(goal.targetCents)}</span>
  </div>
  <footer>
    <span class="forecast">{forecastLabel}</span>
    {#if progress.onTrack === true && !reached}
      <span class="badge ok">{tg.onTrack}</span>
    {:else if progress.onTrack === false && !reached}
      <span class="badge bad">{tg.behind}</span>
    {:else if reached}
      <span class="badge ok">{tg.reached}</span>
    {/if}
  </footer>
</article>

<style>
  .goal-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 16px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 12px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .icon {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    background: var(--surface-2);
    display: grid;
    place-items: center;
  }
  h4 {
    flex: 1;
    margin: 0;
    font-size: 14px;
    font-weight: 500;
  }
  .edit {
    background: none;
    border: 0;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
  }
  .edit:hover { color: var(--text); background: var(--surface-2); }
  .bar {
    height: 8px;
    background: var(--surface-2);
    border-radius: 999px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 999px;
    transition: width .25s ease;
  }
  .fill.reached { background: var(--success, #22c55e) !important; }
  .amounts {
    font-size: 13px;
  }
  .amounts .sep { color: var(--text-faint); margin: 0 4px; }
  .amounts .target { color: var(--text-muted); }
  footer {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-muted);
  }
  .badge {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 11px;
  }
  .badge.ok { background: var(--surface-2); color: var(--success, #22c55e); }
  .badge.bad { background: var(--surface-2); color: var(--danger, #ef4444); }
</style>
