import type { HandlerRegistry } from '../tauri-shim';
import type { MockStore } from '../store';

export function createCategoryHandlers(store: MockStore): HandlerRegistry {
  return {
    list_categories: () => store.categories.map((c) => ({ ...c })),
  };
}
