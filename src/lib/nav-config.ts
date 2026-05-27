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

/** Hauptnavigation in Sidebar/Rail */
export const navMain: NavItem[] = [
  { id: 'dash',      href: '/',             icon: 'overview',  labelKey: 'overview' },
  { id: 'networth',  href: '/networth',     icon: 'networth',  labelKey: 'networth' },
  { id: 'portfolio', href: '/portfolio',    icon: 'reports',   labelKey: 'portfolio' },
  { id: 'goals',     href: '/goals',        icon: 'goal',      labelKey: 'goals' },
  { id: 'buckets',   href: '/buckets',      icon: 'tag',       labelKey: 'buckets' },
  { id: 'recurring', href: '/recurring',    icon: 'repeat',    labelKey: 'recurring' },
  { id: 'tx',        href: '/transactions', icon: 'tx',        labelKey: 'transactions' },
  { id: 'budgets',   href: '/budgets',      icon: 'budget',    labelKey: 'budgets' },
  { id: 'cashflow',  href: '/cashflow',     icon: 'cashflow',  labelKey: 'cashflow' },
  { id: 'reports',   href: '/reports',      icon: 'reports',   labelKey: 'reports' },
];

/** "Verwalten"-Sektion in Sidebar/Rail */
export const navManage: NavItem[] = [
  { id: 'accounts',     href: '/accounts',  icon: 'accounts',  labelKey: 'accounts' },
  { id: 'institutions', href: '/institute', icon: 'briefcase', labelKey: 'institutions' },
  { id: 'settings',     href: '/settings',  icon: 'settings',  labelKey: 'settings' },
];

/** 5 Slots der BottomTabBar (letzter ist "Mehr") */
export const mobileTabs: MobileTab[] = [
  { id: 'dash',     href: '/',             icon: 'home',     labelKey: 'overview' },
  { id: 'tx',       href: '/transactions', icon: 'tx',       labelKey: 'transactions' },
  { id: 'budgets',  href: '/budgets',      icon: 'budget',   labelKey: 'budgets' },
  { id: 'networth', href: '/networth',     icon: 'networth', labelKey: 'networth' },
  // 5. Tab "Mehr" wird in BottomTabBar.svelte als Spezial-Eintrag gerendert
];

/** Im "Mehr"-Sheet sichtbare Routen (Hauptnav minus die 4 Bottom-Tabs) */
export const moreItems: NavItem[] = [
  { id: 'portfolio', href: '/portfolio',   icon: 'reports',   labelKey: 'portfolio' },
  { id: 'goals',     href: '/goals',       icon: 'goal',      labelKey: 'goals' },
  { id: 'buckets',   href: '/buckets',     icon: 'tag',       labelKey: 'buckets' },
  { id: 'recurring', href: '/recurring',   icon: 'repeat',    labelKey: 'recurring' },
  { id: 'cashflow',  href: '/cashflow',    icon: 'cashflow',  labelKey: 'cashflow' },
  { id: 'reports',   href: '/reports',     icon: 'reports',   labelKey: 'reports' },
  ...navManage,
];

/** Routen, auf denen der FAB (+ Tx) sichtbar ist */
export const fabRoutes = new Set<string>(['/', '/transactions']);

/** Aktiv-Check (gleiche Logik wie heute in +layout.svelte) */
export function isActive(currentPath: string, href: string): boolean {
  if (href === '/') return currentPath === '/';
  return currentPath === href || currentPath.startsWith(href + '/');
}
