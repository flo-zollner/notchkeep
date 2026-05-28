import { createMockInvoke, type HandlerRegistry } from './tauri-shim';
import { createMockStore } from './store';
import { createAccountHandlers } from './handlers/accounts';
import { createCategoryHandlers } from './handlers/categories';
import { createInstitutionHandlers } from './handlers/institutions';
import { createChartHandlers } from './handlers/charts';
import { createTransactionHandlers } from './handlers/transactions';
import { createPortfolioHandlers } from './handlers/portfolio';
import { createExtraHandlers } from './handlers/extras';

/**
 * Baut eine `invoke`-Funktion, die alle bekannten Tauri-Commands an
 * In-Memory-Handler routet. Pro Aufruf eigener Store — d.h. Tests sind
 * isoliert, in der Browser-Runtime gibt es genau einen Aufruf (siehe
 * vite-Alias auf tauri-shim).
 */
export function createMockTauriInvoke() {
  const store = createMockStore();
  const registry: HandlerRegistry = {
    ...createAccountHandlers(store),
    ...createCategoryHandlers(store),
    ...createInstitutionHandlers(store),
    ...createChartHandlers(store),
    ...createTransactionHandlers(store),
    ...createPortfolioHandlers(store),
    ...createExtraHandlers(store),
  };
  return createMockInvoke(registry);
}

/**
 * Re-Export der Standard-`invoke`-Funktion für den Vite-Alias.
 * Nutzt einen Modul-singleton-Store, damit alle Komponenten denselben State
 * sehen, solange das Modul nicht neu geladen wird.
 */
const singletonInvoke = createMockTauriInvoke();
export const invoke = singletonInvoke;
