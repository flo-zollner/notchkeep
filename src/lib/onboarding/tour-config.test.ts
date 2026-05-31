import { describe, it, expect } from 'vitest';
import { EXPLAINER_CARDS, TOUR_STEPS } from './tour-config';
import { I18N } from '$lib/i18n/strings';

describe('EXPLAINER_CARDS', () => {
  it('covers all six workflows in a stable order', () => {
    expect(EXPLAINER_CARDS.map((c) => c.id)).toEqual([
      'accounts', 'transactions', 'budgets', 'buckets', 'portfolio', 'import',
    ]);
  });

  it('every card id resolves to text in both languages', () => {
    for (const card of EXPLAINER_CARDS) {
      for (const lang of ['de', 'en'] as const) {
        const entry = I18N[lang].onboarding.cards[card.id];
        expect(entry.title.length, `${lang}/${card.id}/title`).toBeGreaterThan(0);
        expect(entry.body.length, `${lang}/${card.id}/body`).toBeGreaterThan(0);
      }
    }
  });

  it('every card has a non-empty icon name', () => {
    for (const card of EXPLAINER_CARDS) {
      expect(card.icon.length).toBeGreaterThan(0);
    }
  });
});

describe('TOUR_STEPS', () => {
  it('every step has at least one selector and an absolute route', () => {
    for (const step of TOUR_STEPS) {
      expect(step.selectors.length, step.id).toBeGreaterThan(0);
      expect(step.route.startsWith('/'), step.id).toBe(true);
    }
  });

  it('every step id resolves to text in both languages', () => {
    for (const step of TOUR_STEPS) {
      for (const lang of ['de', 'en'] as const) {
        const entry = I18N[lang].onboarding.tourSteps[step.id];
        expect(entry.title.length, `${lang}/${step.id}/title`).toBeGreaterThan(0);
        expect(entry.body.length, `${lang}/${step.id}/body`).toBeGreaterThan(0);
      }
    }
  });
});
