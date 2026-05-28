import type { Institution } from '$lib/api';
import type { HandlerRegistry } from '../tauri-shim';
import type { MockStore } from '../store';
import { mockAccountBalanceCents } from './accounts';

export function createInstitutionHandlers(store: MockStore): HandlerRegistry {
  return {
    list_institutions: (raw) => {
      const args = (raw ?? {}) as { includeArchived?: boolean };
      const source = args.includeArchived
        ? store.institutions
        : store.institutions.filter((i) => !i.archived);
      return source.map((i) => ({ ...i }));
    },

    list_institutions_with_summary: () =>
      store.institutions
        .filter((i) => !i.archived)
        .map((i) => {
          const accounts = store.accounts.filter(
            (a) => a.institution_id === i.id && !a.archived,
          );
          return {
            ...i,
            accountCount: accounts.length,
            balanceCents: accounts.reduce(
              (sum, a) => sum + mockAccountBalanceCents(a.id),
              0,
            ),
          };
        }),

    get_institution: (raw) => {
      const { id } = raw as { id: number };
      const found = store.institutions.find((i) => i.id === id);
      if (!found) throw { message: `Institution ${id} not found` };
      return { ...found };
    },
  };
}
