// Mock-Daten für die noch nicht backend-gestützten Screens (Budgets-Preview).
// Beträge in Euro (nicht Cent), nur für Visualisierung. Sobald entsprechende
// Backend-Tabellen existieren, werden die Mocks ersetzt.

export interface MockBudget {
  categoryId: string;
  name: string;
  icon: string;
  color: string;
  budget: number;
  spent: number;
}

export const MOCK_BUDGETS: MockBudget[] = [
  { categoryId: 'groceries', name: 'Lebensmittel', icon: 'cart', color: 'var(--c1)', budget: 480, spent: 224.97 },
  { categoryId: 'rent', name: 'Miete', icon: 'home', color: 'var(--c2)', budget: 1280, spent: 1280 },
  { categoryId: 'transport', name: 'Transport', icon: 'car', color: 'var(--c3)', budget: 180, spent: 146.1 },
  { categoryId: 'dining', name: 'Restaurant', icon: 'utensils', color: 'var(--c4)', budget: 200, spent: 82.5 },
  { categoryId: 'entertainment', name: 'Freizeit', icon: 'film', color: 'var(--c5)', budget: 120, spent: 83 },
  { categoryId: 'shopping', name: 'Shopping', icon: 'bag', color: 'var(--c6)', budget: 200, spent: 132.94 },
  { categoryId: 'utilities', name: 'Nebenkosten', icon: 'bolt', color: 'var(--c2)', budget: 140, spent: 107.99 },
  { categoryId: 'health', name: 'Gesundheit', icon: 'heart', color: 'var(--c5)', budget: 80, spent: 43.23 },
  { categoryId: 'subscriptions', name: 'Abos', icon: 'repeat', color: 'var(--c4)', budget: 60, spent: 28.98 },
  { categoryId: 'insurance', name: 'Versicherung', icon: 'shield', color: 'var(--c6)', budget: 90, spent: 22.4 },
];
