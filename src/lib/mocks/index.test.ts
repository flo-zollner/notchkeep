import { describe, it, expect } from 'vitest';
import { createMockTauriInvoke } from './index';
import type { Account, Category, Institution } from '$lib/api';

describe('Mock-Tauri invoke — accounts/categories/institutions', () => {
  it('get_accounts returns a non-empty list of Account-shaped objects', async () => {
    const invoke = createMockTauriInvoke();
    const accounts = await invoke<Account[]>('get_accounts');

    expect(accounts.length).toBeGreaterThan(0);
    for (const a of accounts) {
      expect(typeof a.id).toBe('number');
      expect(typeof a.name).toBe('string');
      expect(typeof a.kind).toBe('string');
      expect(typeof a.currency).toBe('string');
      expect(typeof a.archived).toBe('boolean');
    }
  });

  it('seeded IBANs follow the dummy pattern (CLAUDE.md privacy rule)', async () => {
    const invoke = createMockTauriInvoke();
    const accounts = await invoke<Account[]>('get_accounts');

    const ibansWithValue = accounts
      .map((a) => a.iban)
      .filter((iban): iban is string => iban !== null);
    expect(ibansWithValue.length).toBeGreaterThan(0);

    const dummyPattern = /^DE(00[ 0]+00|12[ ]?3456)/;
    for (const iban of ibansWithValue) {
      expect(iban).toMatch(dummyPattern);
    }
  });

  it('get_account returns the matching seeded account', async () => {
    const invoke = createMockTauriInvoke();
    const all = await invoke<Account[]>('get_accounts');
    const first = all[0];

    const fetched = await invoke<Account>('get_account', { id: first.id });

    expect(fetched).toEqual(first);
  });

  it('create_account appends to the store and returns the new account', async () => {
    const invoke = createMockTauriInvoke();
    const before = await invoke<Account[]>('get_accounts');

    const created = await invoke<Account>('create_account', {
      name: 'Test-Konto',
      kind: 'checking',
      currency: 'EUR',
      parentId: null,
      iban: null,
      institutionId: null,
    });

    expect(created.name).toBe('Test-Konto');
    expect(typeof created.id).toBe('number');

    const after = await invoke<Account[]>('get_accounts');
    expect(after.length).toBe(before.length + 1);
    expect(after.map((a) => a.id)).toContain(created.id);
  });

  it('account_balance returns a number for a known account', async () => {
    const invoke = createMockTauriInvoke();
    const accounts = await invoke<Account[]>('get_accounts');

    const balance = await invoke<number>('account_balance', { id: accounts[0].id });

    expect(typeof balance).toBe('number');
  });

  it('list_categories returns Category[] with at least one parentless entry', async () => {
    const invoke = createMockTauriInvoke();
    const cats = await invoke<Category[]>('list_categories');

    expect(cats.length).toBeGreaterThan(0);
    expect(cats.some((c) => c.parent_id === null)).toBe(true);
  });

  it('list_institutions returns Institution[] with synthetic names', async () => {
    const invoke = createMockTauriInvoke();
    const instis = await invoke<Institution[]>('list_institutions', { includeArchived: false });

    expect(instis.length).toBeGreaterThan(0);
    for (const i of instis) {
      // Synthetic names: no real bank trademarks
      expect(i.name).not.toMatch(/Sparkasse|Deutsche Bank|Commerzbank|Flatex|Trade Republic/i);
    }
  });
});
