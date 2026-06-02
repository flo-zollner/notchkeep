import type { NavStrings } from '$lib/i18n/strings';

export interface NavItem {
  id: string;
  href: string;
  icon: string;
  labelKey: keyof NavStrings;
}

export interface MobileTab {
  id: string;
  href: string;
  icon: string;
  labelKey: keyof NavStrings;
}

/** Main navigation in sidebar/rail. */
export const navMain: NavItem[] = [
  { id: 'dash',      href: '/',             icon: 'overview',  labelKey: 'overview' },
  { id: 'networth',  href: '/networth',     icon: 'networth',  labelKey: 'networth' },
  { id: 'portfolio', href: '/portfolio',    icon: 'reports',   labelKey: 'portfolio' },
  { id: 'buckets',   href: '/buckets',      icon: 'tag',       labelKey: 'buckets' },
  { id: 'recurring', href: '/recurring',    icon: 'repeat',    labelKey: 'recurring' },
  { id: 'tx',        href: '/transactions', icon: 'tx',        labelKey: 'transactions' },
  { id: 'budgets',   href: '/budgets',      icon: 'budget',    labelKey: 'budgets' },
  { id: 'reports',   href: '/reports',      icon: 'reports',   labelKey: 'reports' },
];

/** "Manage" section in sidebar/rail. */
export const navManage: NavItem[] = [
  { id: 'accounts',     href: '/accounts',  icon: 'accounts',  labelKey: 'accounts' },
  { id: 'institutions', href: '/institute', icon: 'briefcase', labelKey: 'institutions' },
  { id: 'settings',     href: '/settings',  icon: 'settings',  labelKey: 'settings' },
];

/** 5 slots of the BottomTabBar (last one is "More"). */
export const mobileTabs: MobileTab[] = [
  { id: 'dash',     href: '/',             icon: 'home',     labelKey: 'overview' },
  { id: 'tx',       href: '/transactions', icon: 'tx',       labelKey: 'transactions' },
  { id: 'budgets',  href: '/budgets',      icon: 'budget',   labelKey: 'budgets' },
  { id: 'networth', href: '/networth',     icon: 'networth', labelKey: 'networth' },
  // 5th tab "More" is rendered as a special entry in BottomTabBar.svelte
];

/** Routes visible in the "More" sheet (main nav minus the 4 bottom tabs). */
export const moreItems: NavItem[] = [
  { id: 'portfolio', href: '/portfolio',   icon: 'reports',   labelKey: 'portfolio' },
  { id: 'buckets',   href: '/buckets',     icon: 'tag',       labelKey: 'buckets' },
  { id: 'recurring', href: '/recurring',   icon: 'repeat',    labelKey: 'recurring' },
  { id: 'reports',   href: '/reports',     icon: 'reports',   labelKey: 'reports' },
  ...navManage,
];

/** FAB actions per route: label shown as aria-label, query param appended on click. */
export const fabActions: Record<string, { label: string; query: string }> = {
  '/':             { label: 'Neue Transaktion', query: 'new=1' },
  '/transactions': { label: 'Neue Transaktion', query: 'new=1' },
  '/buckets':      { label: 'Neuer Topf',        query: 'new=1' },
  '/recurring':    { label: 'Neue Zahlung',       query: 'new=1' },
  '/portfolio':    { label: 'Trade hinzufügen',   query: 'new=1' },
};

/** Routes on which the FAB is visible. */
export const fabRoutes = new Set(Object.keys(fabActions));

/** Active check (same logic as in +layout.svelte). */
export function isActive(currentPath: string, href: string): boolean {
  if (href === '/') return currentPath === '/';
  return currentPath === href || currentPath.startsWith(href + '/');
}
