import { describe, it, expect, beforeEach } from 'vitest';
import { settings, _reloadForTests } from '$lib/settings.svelte';
import {
  onboarding,
  evaluateAutoStart,
  startOnboarding,
  finishOnboarding,
  startTour,
  finishTour,
  tourNext,
  tourPrev,
} from './onboarding.svelte';

beforeEach(() => {
  localStorage.clear();
  _reloadForTests();
  onboarding.wizardActive = false;
  onboarding.tourActive = false;
  onboarding.wizardStep = 0;
  onboarding.tourStep = 0;
});

describe('evaluateAutoStart', () => {
  it('opens the wizard for a fresh install (not completed, no accounts)', () => {
    expect(evaluateAutoStart({ completed: false, accountCount: 0, force: false })).toBe('open-wizard');
  });

  it('does nothing when onboarding already completed', () => {
    expect(evaluateAutoStart({ completed: true, accountCount: 0, force: false })).toBe('noop');
  });

  it('silently marks completed for an existing user with accounts but no flag (upgrade case)', () => {
    expect(evaluateAutoStart({ completed: false, accountCount: 3, force: false })).toBe('mark-completed');
  });

  it('force always opens the wizard, even when completed', () => {
    expect(evaluateAutoStart({ completed: true, accountCount: 5, force: true })).toBe('open-wizard');
  });
});

describe('wizard lifecycle', () => {
  it('startOnboarding activates the wizard and resets to step 0', () => {
    onboarding.wizardStep = 4;
    startOnboarding();
    expect(onboarding.wizardActive).toBe(true);
    expect(onboarding.wizardStep).toBe(0);
  });

  it('finishOnboarding deactivates and persists onboardingCompleted', () => {
    startOnboarding();
    finishOnboarding();
    expect(onboarding.wizardActive).toBe(false);
    expect(settings.onboardingCompleted).toBe(true);
    expect(JSON.parse(localStorage.getItem('saldo.settings')!).onboardingCompleted).toBe(true);
  });
});

describe('tour lifecycle', () => {
  it('startTour activates the tour at step 0', () => {
    onboarding.tourStep = 3;
    startTour();
    expect(onboarding.tourActive).toBe(true);
    expect(onboarding.tourStep).toBe(0);
  });

  it('tourNext advances within bounds', () => {
    startTour();
    tourNext(3);
    expect(onboarding.tourStep).toBe(1);
  });

  it('tourNext on the last step finishes the tour and persists tourCompleted', () => {
    startTour();
    onboarding.tourStep = 2;
    tourNext(3);
    expect(onboarding.tourActive).toBe(false);
    expect(settings.tourCompleted).toBe(true);
  });

  it('tourPrev does not go below 0', () => {
    startTour();
    tourPrev();
    expect(onboarding.tourStep).toBe(0);
  });

  it('finishTour deactivates and persists', () => {
    startTour();
    finishTour();
    expect(onboarding.tourActive).toBe(false);
    expect(settings.tourCompleted).toBe(true);
  });
});
