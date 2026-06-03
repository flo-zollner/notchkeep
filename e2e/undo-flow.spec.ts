import { test, expect } from '@playwright/test';
import { mockTauri, seedSettings } from './tauri-mock';

/**
 * Interaction proof for the Undo-Snackbar flow (guide §19): deleting a bucket
 * shows a snackbar with "Rückgängig"; clicking it restores the bucket.
 * Stateful mock layered on top of mockTauri: a single bucket that delete hides
 * and restore brings back.
 */
test('bucket delete shows undo snackbar and restores it', async ({ page }, testInfo) => {
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
    const bucket = {
      id: 1, name: 'Urlaub', icon: null, color: null, note: null,
      targetCents: null, startDate: null, targetDate: null, archived: false,
      createdAt: '2026-01-01',
    };
    const deleted = new Set<number>();
    internals.invoke = async (cmd: string, args: Record<string, unknown> = {}) => {
      if (cmd === 'list_buckets') return deleted.has(bucket.id) ? [] : [bucket];
      if (cmd === 'delete_bucket') { deleted.add(args.id as number); return true; }
      if (cmd === 'restore_bucket') { deleted.delete(args.id as number); return true; }
      return orig(cmd, args);
    };
  });

  await page.goto('/buckets', { waitUntil: 'networkidle' });

  // The bucket card is shown.
  await expect(page.getByText('Urlaub').first()).toBeVisible();

  // Open the bucket editor and delete it.
  await page.getByText('Urlaub').first().click();
  await page.getByRole('button', { name: 'Löschen', exact: true }).click();

  // Snackbar appears, bucket is gone from the list.
  await expect(page.locator('.snackbar')).toBeVisible();
  await expect(page.getByText('Gelöscht')).toBeVisible();
  await expect(page.getByText('Urlaub')).toHaveCount(0);

  // Undo restores it.
  await page.getByRole('button', { name: 'Rückgängig', exact: true }).click();
  await expect(page.getByText('Urlaub').first()).toBeVisible();
  await expect(page.locator('.snackbar')).toHaveCount(0);
});
