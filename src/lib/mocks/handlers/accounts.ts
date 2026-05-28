import type { Account } from '$lib/api';
import type { HandlerRegistry } from '../tauri-shim';
import type { MockStore } from '../store';

interface CreateAccountArgs {
  name: string;
  kind: string;
  currency?: string | null;
  parentId?: number | null;
  iban?: string | null;
  institutionId?: number | null;
}

/**
 * Deterministic fake balance based on account id, in cents.
 * Spread across positive/negative to exercise sign-aware UI.
 * Shared with the institution summary handler so both views agree.
 */
export function mockAccountBalanceCents(id: number): number {
  const balances: Record<number, number> = {
    1: 342_15,
    2: 12_480_00,
    3: 1_823_47,
    4: 0,
    5: -218_94,
  };
  return balances[id] ?? 0;
}

export function createAccountHandlers(store: MockStore): HandlerRegistry {
  return {
    // Snapshot statt Live-Array: Tauri-IPC ist eine Serialisierungs-Grenze,
    // so dass das Frontend nie eine Referenz auf das Backend-Modell hält.
    get_accounts: () => store.accounts.map((a) => ({ ...a })),

    get_account: (raw) => {
      const { id } = raw as { id: number };
      const found = store.accounts.find((a) => a.id === id);
      if (!found) throw { message: `Account ${id} not found` };
      return { ...found };
    },

    create_account: (raw) => {
      const args = raw as CreateAccountArgs;
      const account: Account = {
        id: store.nextAccountId++,
        name: args.name,
        kind: args.kind,
        currency: args.currency ?? 'EUR',
        icon: null,
        color: null,
        note: null,
        last4: null,
        archived: false,
        parent_id: args.parentId ?? null,
        iban: args.iban ?? null,
        institution_id: args.institutionId ?? null,
        created_at: new Date().toISOString(),
      };
      store.accounts.push(account);
      return { ...account };
    },

    update_account: (raw) => {
      const { account } = raw as { account: Account };
      const idx = store.accounts.findIndex((a) => a.id === account.id);
      if (idx === -1) throw { message: `Account ${account.id} not found` };
      store.accounts[idx] = account;
      return null;
    },

    account_balance: (raw) => {
      const { id } = raw as { id: number };
      return mockAccountBalanceCents(id);
    },
  };
}
