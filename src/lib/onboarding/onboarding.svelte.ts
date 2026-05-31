import { setOnboardingCompleted, setTourCompleted } from '$lib/settings.svelte';

/**
 * Shared, reactive onboarding UI state. Two independent surfaces:
 *  - the first-run setup wizard (`wizard*`)
 *  - the interactive feature tour / coach-marks (`tour*`)
 * Both can be (re-)started from settings, hence module-level singleton state
 * rather than route-local state.
 */
export const onboarding = $state({
  wizardActive: false,
  tourActive: false,
  wizardStep: 0,
  tourStep: 0,
});

export type AutoStartAction = 'open-wizard' | 'mark-completed' | 'noop';

/**
 * Pure decision for what should happen on app start. Kept side-effect free so
 * the trigger logic is unit-testable; the layout performs the resulting action.
 *
 * - `force`: query-param override (`?onboarding=force`) → always show.
 * - already completed → nothing to do.
 * - fresh install (no accounts) → show the wizard.
 * - existing user with data but no flag (app upgraded into onboarding) → mark
 *   completed silently so we never interrupt someone who is already set up.
 */
export function evaluateAutoStart(opts: {
  completed: boolean;
  accountCount: number;
  force: boolean;
}): AutoStartAction {
  if (opts.force) return 'open-wizard';
  if (opts.completed) return 'noop';
  if (opts.accountCount === 0) return 'open-wizard';
  return 'mark-completed';
}

export function startOnboarding(): void {
  onboarding.wizardStep = 0;
  onboarding.wizardActive = true;
}

export function finishOnboarding(): void {
  onboarding.wizardActive = false;
  setOnboardingCompleted(true);
}

export function startTour(): void {
  onboarding.tourStep = 0;
  onboarding.tourActive = true;
}

export function finishTour(): void {
  onboarding.tourActive = false;
  setTourCompleted(true);
}

/** Advance the tour; finishing it once past the last step. */
export function tourNext(total: number): void {
  if (onboarding.tourStep < total - 1) {
    onboarding.tourStep += 1;
  } else {
    finishTour();
  }
}

export function tourPrev(): void {
  if (onboarding.tourStep > 0) onboarding.tourStep -= 1;
}
