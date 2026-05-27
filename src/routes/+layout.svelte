<script lang="ts">
  import '../app.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Rail from '$lib/components/Rail.svelte';
  import MobileHeader from '$lib/components/MobileHeader.svelte';
  import BottomTabBar from '$lib/components/BottomTabBar.svelte';
  import Fab from '$lib/components/Fab.svelte';
  import { settings } from '$lib/settings.svelte';
  import { listen } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import StartupErrorModal from '$lib/components/StartupErrorModal.svelte';
  import SyncConflictModal from '$lib/components/SyncConflictModal.svelte';
  import { api, type SyncConflictFile } from '$lib/api';

  let { children } = $props();

  let startupError = $state<{ path: string } | null>(null);
  let priceRefreshStage = $state<'idle' | 'started' | 'completed' | 'failed'>('idle');
  let priceRefreshHideTimer: ReturnType<typeof setTimeout> | null = null;
  let budgetAlertCount = $state(0);
  let syncConflicts = $state<SyncConflictFile[] | null>(null);

  async function checkConflictsOnce() {
    try {
      const list = await api.checkSyncConflicts();
      if (list.length > 0) syncConflicts = list;
    } catch {
      // silently — Conflict-Check ist non-critical bei erstem Start
    }
  }

  $effect(() => {
    void checkConflictsOnce();
  });

  /** Lädt MonthOverview für aktuellen Monat und zählt Kategorien
   * deren spent >= 80% des budgets ist (und budget gesetzt). */
  async function refreshBudgetAlerts() {
    try {
      const now = new Date();
      const rows = await api.monthOverview(now.getFullYear(), now.getMonth() + 1);
      const n = rows.filter(
        (r) => r.budgetCents !== null && r.budgetCents > 0 && r.spentCents >= 0.8 * r.budgetCents
      ).length;
      budgetAlertCount = n;
    } catch {
      budgetAlertCount = 0;
    }
  }
  $effect(() => {
    void refreshBudgetAlerts();
  });

  $effect(() => {
    const unlisten = listen<{ path: string; reason: string }>('data_path_error', (e) => {
      startupError = { path: e.payload.path };
    });
    return () => { unlisten.then((u) => u()); };
  });

  $effect(() => {
    type Status = { stage: 'started' | 'completed' | 'failed' };
    const unlisten = listen<Status>('price_refresh_status', (e) => {
      if (priceRefreshHideTimer) clearTimeout(priceRefreshHideTimer);
      priceRefreshStage = e.payload.stage;
      if (e.payload.stage === 'completed' || e.payload.stage === 'failed') {
        priceRefreshHideTimer = setTimeout(() => (priceRefreshStage = 'idle'), 4000);
      }
    });
    return () => { unlisten.then((u) => u()); };
  });

  // Apply theme to documentElement
  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = settings.theme;
    }
  });

  function onFabClick() {
    goto('/transactions?new=1');
  }

  /** Globale Keyboard-Shortcuts. Ignoriert wenn Fokus in Input/Textarea/Select. */
  function onGlobalKey(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const target = e.target as HTMLElement | null;
    if (target && (
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.tagName === 'SELECT' ||
      target.isContentEditable
    )) return;
    switch (e.key) {
      case 'g':
        // Vim-style g + nächster Buchstabe für Navigation. Vereinfacht: 'g d' = Dashboard etc.
        // Hier minimal: nur direkte Hot-Keys nicht-chord.
        break;
      case 't': goto('/transactions'); break;
      case 'b': goto('/budgets'); break;
      case 'c': goto('/cashflow'); break;
      case 'p': goto('/portfolio'); break;
      case 'd': goto('/'); break;
      case '?': {
        e.preventDefault();
        alert('Keyboard-Shortcuts:\n  d = Dashboard\n  t = Transaktionen\n  b = Budgets\n  c = Cashflow\n  p = Portfolio\n  ? = diese Hilfe');
        break;
      }
    }
  }
</script>

<svelte:window onkeydown={onGlobalKey} />

{#if startupError}
  <StartupErrorModal
    badPath={startupError.path}
    onResolved={() => (startupError = null)}
  />
{/if}

{#if syncConflicts && syncConflicts.length > 0}
  <SyncConflictModal
    conflicts={syncConflicts}
    onResolved={() => { syncConflicts = null; void checkConflictsOnce(); }}
  />
{/if}

<MobileHeader />

<div class="app">
  <Sidebar {budgetAlertCount} {priceRefreshStage} />
  <Rail {budgetAlertCount} {priceRefreshStage} />
  <main class="main">
    {@render children()}
  </main>
</div>

<BottomTabBar {budgetAlertCount} />
<Fab onClick={onFabClick} />
