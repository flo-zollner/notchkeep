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

/**
 * Minimal stub of `@tauri-apps/api/core`'s `Channel`. It exists so the mock
 * alias provides every named export the app imports from `core` (the Android
 * updater backend imports `Channel`); the optimizer needs it resolvable. The
 * Android updater path never executes in the browser/mock runtime, so a
 * constructable no-op with an `onmessage` slot is sufficient.
 */
export class Channel<T = unknown> {
  id = 0;
  onmessage: (message: T) => void = () => {};
  toJSON() {
    return `__CHANNEL__:${this.id}`;
  }
}

/**
 * Minimal stub of `@tauri-apps/api/core`'s `Resource` base class. The updater
 * plugin's `Update` extends it; like `Channel`, it only needs to be a
 * constructable named export so the mock-aliased `core` resolves every import
 * the bundled plugins pull in (otherwise esbuild's dep scan fails and the dev
 * optimizer never settles → 504 "Outdated Optimize Dep"). Never instantiated
 * in the browser/mock runtime.
 */
export class Resource {
  #rid: number;
  constructor(rid = 0) {
    this.#rid = rid;
  }
  get rid() {
    return this.#rid;
  }
  close(): Promise<void> {
    return Promise.resolve();
  }
}
