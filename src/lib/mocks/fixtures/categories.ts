import type { Category } from '$lib/api';

/**
 * Kategorien mit Parent-Child-Struktur — genug Variation, dass Categorie-Editor,
 * Budgets-Übersicht und Kategorien-Donut nicht leer wirken.
 */
export const SEED_CATEGORIES: Category[] = [
  // Top-Level
  { id: 1, parent_id: null, name: 'Lebensmittel', color: 'var(--c1)', icon: 'cart', rollover_enabled: false },
  { id: 2, parent_id: null, name: 'Wohnen', color: 'var(--c2)', icon: 'home', rollover_enabled: false },
  { id: 3, parent_id: null, name: 'Transport', color: 'var(--c3)', icon: 'car', rollover_enabled: false },
  { id: 4, parent_id: null, name: 'Freizeit', color: 'var(--c5)', icon: 'film', rollover_enabled: true },
  { id: 5, parent_id: null, name: 'Gesundheit', color: 'var(--c6)', icon: 'heart', rollover_enabled: false },
  { id: 6, parent_id: null, name: 'Abos', color: 'var(--c4)', icon: 'repeat', rollover_enabled: false },
  { id: 7, parent_id: null, name: 'Einkommen', color: 'var(--c2)', icon: 'arrow-down', rollover_enabled: false },
  { id: 8, parent_id: null, name: 'Investitionen', color: 'var(--c4)', icon: 'chart-line', rollover_enabled: false },
  // Sub-Categories
  { id: 11, parent_id: 1, name: 'Supermarkt', color: 'var(--c1)', icon: null, rollover_enabled: false },
  { id: 12, parent_id: 1, name: 'Restaurant', color: 'var(--c1)', icon: 'utensils', rollover_enabled: false },
  { id: 21, parent_id: 2, name: 'Miete', color: 'var(--c2)', icon: null, rollover_enabled: false },
  { id: 22, parent_id: 2, name: 'Nebenkosten', color: 'var(--c2)', icon: 'bolt', rollover_enabled: false },
  { id: 31, parent_id: 3, name: 'ÖPNV', color: 'var(--c3)', icon: null, rollover_enabled: false },
  { id: 32, parent_id: 3, name: 'Tanken', color: 'var(--c3)', icon: null, rollover_enabled: false },
];
