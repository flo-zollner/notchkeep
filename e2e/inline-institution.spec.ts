import { test, expect } from '@playwright/test';
import { mockTauri, seedSettings } from './tauri-mock';

/**
 * §11 proof: creating an institution from the account-edit form happens INLINE
 * (one sheet), not as a second stacked modal. Stateful mock for one account +
 * institutions + create_institution.
 */
test('institution creation in the account form is inline, not a nested modal', async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== 'desktop', 'one viewport is enough');

  await seedSettings(page, {
    onboardingCompleted: true,
    tourCompleted: true,
    lang: 'de',
    updateConsent: 'declined',
  });
  await mockTauri(page);
  await page.addInitScript(() => {
    const internals = (window as unknown as Record<string, { invoke: (c: string, a?: Record<string, unknown>) => Promise<unknown> }>)
      .__TAURI_INTERNALS__;
    const orig = internals.invoke;
    const account = {
      id: 1, name: 'Girokonto', kind: 'bank', currency: 'EUR', icon: null, color: null,
      note: null, last4: null, archived: false, parent_id: null, iban: null,
      institution_id: null, created_at: '2026-01-01',
    };
    const institutions: Record<string, unknown>[] = [];
    let nextId = 10;
    internals.invoke = async (cmd: string, args: Record<string, unknown> = {}) => {
      switch (cmd) {
        case 'get_accounts': return [account];
        case 'account_balance': return 0;
        case 'account_monthly_cashflow': return [];
        case 'list_institutions': return institutions;
        case 'list_institutions_with_summary':
          return institutions.map((i) => ({ ...i, accountCount: 0, balanceCents: 0 }));
        case 'create_institution': {
          const inst = {
            id: nextId++, name: (args.payload as { name: string })?.name ?? 'X',
            icon: null, color: null, bic: null, country: null, note: null,
            archived: false, createdAt: '2026-01-01',
          };
          institutions.push(inst);
          return inst;
        }
        default: return orig(cmd, args);
      }
    };
  });

  await page.goto('/accounts', { waitUntil: 'networkidle' });

  // Open the account editor.
  await expect(page.getByText('Girokonto').first()).toBeVisible();
  await page.getByText('Girokonto').first().hover();
  await page.locator('[aria-label="edit"]').first().click();

  // Exactly one sheet panel (the account editor).
  const sheets = page.locator('.sheet-panel');
  await expect(sheets).toHaveCount(1);

  // Pick "+ Neues Institut anlegen…".
  const instSelect = page.locator('select:has(option[value="__create__"])');
  await instSelect.selectOption('__create__');

  // An inline row appears — and STILL exactly one sheet (no nested modal).
  await expect(page.locator('.inst-create')).toBeVisible();
  await expect(sheets).toHaveCount(1);

  // Create inline.
  await page.locator('.inst-create input').fill('Meine Bank');
  await page.locator('.inst-create button.primary').click();

  // Inline row closes; the new institution is selected.
  await expect(page.locator('.inst-create')).toHaveCount(0);
  await expect(instSelect.locator('option', { hasText: 'Meine Bank' })).toHaveCount(1);
  await expect(instSelect).toHaveValue('10');
});
