import type { OnboardingStrings } from '$lib/i18n/strings';

/**
 * Workflow explainer cards shown in the wizard's "how it works" step.
 * `id` maps to `t().onboarding.cards[id]`; `icon` is an Icon.svelte name.
 */
export interface ExplainerCard {
  id: keyof OnboardingStrings['cards'];
  icon: string;
}

export const EXPLAINER_CARDS: ExplainerCard[] = [
  { id: 'accounts', icon: 'accounts' },
  { id: 'transactions', icon: 'tx' },
  { id: 'budgets', icon: 'budget' },
  { id: 'buckets', icon: 'target' },
  { id: 'portfolio', icon: 'trending' },
  { id: 'import', icon: 'download' },
];

/**
 * Interactive feature-tour step. The overlay navigates to `route`, then
 * highlights the first matching, visible element from `selectors`. If none
 * match (e.g. a desktop-only element on a phone), the tooltip is centred.
 * `.topbar` is page-unique because the SPA only mounts the active route.
 */
export interface TourStep {
  id: keyof OnboardingStrings['tourSteps'];
  route: string;
  selectors: string[];
}

export const TOUR_STEPS: TourStep[] = [
  { id: 'overview', route: '/', selectors: ['[data-tour="dashboard-kpis"]'] },
  { id: 'nav', route: '/', selectors: ['[data-tour="nav"]', '[data-tour="mobilenav"]'] },
  { id: 'transactions', route: '/transactions', selectors: ['.topbar'] },
  // addTx points at the desktop "new transaction" control, falling back to the
  // mobile FAB; both live on the transactions route so there is no extra nav.
  { id: 'addTx', route: '/transactions', selectors: ['[data-tour="new-tx"]', '[data-tour="fab"]'] },
  { id: 'budgets', route: '/budgets', selectors: ['.topbar'] },
  { id: 'import', route: '/', selectors: ['[data-tour="import"]'] },
];
