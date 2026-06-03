<script lang="ts">
  import '../app.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Rail from '$lib/components/Rail.svelte';
  import MobileHeader from '$lib/components/MobileHeader.svelte';
  import BottomTabBar from '$lib/components/BottomTabBar.svelte';
  import Fab from '$lib/components/Fab.svelte';
  import { settings } from '$lib/settings.svelte';
  import { applySystemAccent } from '$lib/system-accent';
  import { applyMaterialYou } from '$lib/md3/dynamic-color';
  import { applyPlatform } from '$lib/platform';
  import { onMount } from 'svelte';

  // Resolve the runtime platform before the first paint so the Material Design 3
  // layer (scoped to `html[data-platform="android"]`) is in place from the start.
  applyPlatform();

  function applyColors() {
    if (typeof document !== 'undefined' && document.documentElement.dataset.platform === 'android') {
      applyMaterialYou();
    } else {
      applySystemAccent();
    }
  }
  import { listen } from '@tauri-apps/api/event';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import StartupErrorModal from '$lib/components/StartupErrorModal.svelte';
  import SyncConflictModal from '$lib/components/SyncConflictModal.svelte';
  import OnboardingWizard from '$lib/components/OnboardingWizard.svelte';
  import TourOverlay from '$lib/components/TourOverlay.svelte';
  import { settings as appSettings, setOnboardingCompleted } from '$lib/settings.svelte';
  import {
    evaluateAutoStart,
    startOnboarding,
    startTour,
    onboarding,
  } from '$lib/onboarding/onboarding.svelte';
  import UpdateActivationDialog from '$lib/components/UpdateActivationDialog.svelte';
  import UpdateAvailableDialog from '$lib/components/UpdateAvailableDialog.svelte';
  import {
    runStartupFlow, enableUpdates, declineUpdates,
    downloadAndInstall, skipCurrent, restart,
  } from '$lib/updater/updater.svelte';
  import { api, type SyncConflictFile } from '$lib/api';

  let { children } = $props();

  /** First-run trigger: decide whether to open the onboarding wizard or the
   *  feature tour. `?onboarding=force` / `?tour=force` override (used by the
   *  settings re-launch buttons and by tests). */
  async function initOnboarding() {
    const params = new URLSearchParams(window.location.search);
    if (params.get('tour') === 'force') startTour();
    const force = params.get('onboarding') === 'force';
    let accountCount = 0;
    try {
      accountCount = (await api.listAccounts()).length;
    } catch {
      accountCount = 0;
    }
    const action = evaluateAutoStart({
      completed: appSettings.onboardingCompleted,
      accountCount,
      force,
    });
    if (action === 'open-wizard') startOnboarding();
    else if (action === 'mark-completed') setOnboardingCompleted(true);
  }

  let isAndroid = $state(false);
  let showActivation = $state(false);
  let showUpdate = $state(false);

  async function startUpdaterFlow() {
    const action = await runStartupFlow();
    if (action === 'show-activation') showActivation = true;
    else if (action === 'show-update') showUpdate = true;
  }

  function onActivationEnable() { showActivation = false; enableUpdates(); void startUpdaterFlow(); }
  function onActivationLater() { showActivation = false; }
  function onActivationNever() { showActivation = false; declineUpdates(); }

  async function onInstall() { await downloadAndInstall(); }
  function onSkip() { skipCurrent(); showUpdate = false; }
  function onCloseUpdate() { showUpdate = false; }
  async function onRestart() { await restart(); }

  // Trigger the updater flow once the wizard closes after first-run onboarding.
  // We track whether the wizard was previously active so we fire only on the
  // transition active→inactive, not on every reactive re-run.
  let wizardWasPreviouslyActive = false;
  $effect(() => {
    const isActive = onboarding.wizardActive;
    if (wizardWasPreviouslyActive && !isActive && appSettings.onboardingCompleted) {
      void startUpdaterFlow();
    }
    wizardWasPreviouslyActive = isActive;
  });

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
      // silently — conflict check is non-critical on first startup
    }
  }

  $effect(() => {
    void checkConflictsOnce();
  });

  /** Loads MonthOverview for the current month and counts categories
   * whose spent >= 80% of their budget (and budget is set). */
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
      // After theme switch, accent must be re-derived (different fg contrast)
      applyColors();
    }
  });

  onMount(() => {
    isAndroid = document.documentElement.dataset.platform === 'android';
    void initOnboarding();
    if (appSettings.onboardingCompleted) void startUpdaterFlow();
    applyColors();
    const reapply = () => applyColors();
    window.addEventListener('focus', reapply);
    document.addEventListener('visibilitychange', reapply);
    // On Android the MD3 scheme is written as inline custom properties on <html>,
    // which override the CSS `prefers-color-scheme` dark block. Re-derive the scheme
    // when the system colour scheme flips so an 'auto' theme still follows dark mode.
    const darkMq = window.matchMedia('(prefers-color-scheme: dark)');
    darkMq.addEventListener('change', reapply);
    return () => {
      window.removeEventListener('focus', reapply);
      document.removeEventListener('visibilitychange', reapply);
      darkMq.removeEventListener('change', reapply);
    };
  });

  function onFabClick() {
    const path = page.url.pathname;
    if (path === '/') { goto('/transactions?new=1'); return; }
    goto(`${path}?new=1`);
  }

  /** Global keyboard shortcuts. Ignored when focus is in Input/Textarea/Select. */
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
        // Vim-style g + next letter for navigation. Simplified: 'g d' = Dashboard etc.
        // Minimal here: only direct hot-keys, no chords.
        break;
      case 't': goto('/transactions'); break;
      case 'b': goto('/budgets'); break;
      case 'r': goto('/reports'); break;
      case 'p': goto('/portfolio'); break;
      case 'd': goto('/'); break;
      case '?': {
        e.preventDefault();
        alert('Keyboard-Shortcuts:\n  d = Dashboard\n  t = Transaktionen\n  b = Budgets\n  r = Reports\n  p = Portfolio\n  ? = diese Hilfe');
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

{#if isAndroid}
  <BottomTabBar {budgetAlertCount} />
  <Fab onClick={onFabClick} />
{/if}

<OnboardingWizard />
<TourOverlay />

<div id="live-announcer" aria-live="polite" aria-atomic="true" style="position:absolute;width:1px;height:1px;padding:0;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0"></div>

{#if showActivation}
  <UpdateActivationDialog onEnable={onActivationEnable} onLater={onActivationLater} onNever={onActivationNever} />
{/if}
{#if showUpdate}
  <UpdateAvailableDialog onInstall={onInstall} onSkip={onSkip} onClose={onCloseUpdate} onRestart={onRestart} />
{/if}
