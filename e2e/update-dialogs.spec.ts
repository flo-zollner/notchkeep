/**
 * Playwright E2E specs for the auto-update UI dialogs.
 *
 * Tests run across three viewport projects (mobile 360px, tablet 768px, desktop 1280px)
 * via playwright.config.ts. Each test:
 *  1. Seeds localStorage before the page loads (via addInitScript).
 *  2. Mocks window.__TAURI_INTERNALS__ so IPC calls don't reject in a plain browser.
 *  3. Navigates and asserts dialog state.
 */

import { test, expect } from '@playwright/test';
import { mockTauri, seedSettings } from './tauri-mock';

test.describe('Update activation dialog', () => {
  /**
   * Activation dialog appears on first start.
   *
   * Seed: onboardingCompleted=true, updateConsent='unset', no update available.
   * Expected: role="dialog" is visible AND fits within the viewport.
   */
  test('appears when onboarding is done and consent is unset', async ({ page }) => {
    await mockTauri(page, {});
    await seedSettings(page, { onboardingCompleted: true, updateConsent: 'unset' });

    await page.goto('/');

    // Wait for the dialog to appear — the startup flow is async so we wait.
    const dialog = page.getByRole('dialog').first();
    await expect(dialog).toBeVisible({ timeout: 15_000 });

    // Verify dialog contains the activation title text.
    await expect(dialog).toContainText('Automatische Updates', { timeout: 10_000 });

    // Cross-viewport size check: dialog bounding box must be within viewport.
    const viewport = page.viewportSize();
    if (viewport) {
      const box = await dialog.boundingBox();
      expect(box).not.toBeNull();
      if (box) {
        expect(box.x).toBeGreaterThanOrEqual(0);
        expect(box.y).toBeGreaterThanOrEqual(0);
        expect(box.x + box.width).toBeLessThanOrEqual(viewport.width + 1); // +1 for sub-pixel
        expect(box.y + box.height).toBeLessThanOrEqual(viewport.height + 1);
      }
    }
  });

  /**
   * Clicking "Später" (Later) closes the activation dialog.
   */
  test('"Später" button closes the activation dialog', async ({ page }) => {
    await mockTauri(page, {});
    await seedSettings(page, { onboardingCompleted: true, updateConsent: 'unset' });

    await page.goto('/');

    const dialog = page.getByRole('dialog').first();
    await expect(dialog).toBeVisible({ timeout: 15_000 });

    // Click "Später" (Later).
    await page.getByRole('button', { name: /später/i }).click();

    // Dialog should no longer be visible.
    await expect(dialog).not.toBeVisible({ timeout: 5_000 });
  });

  /**
   * Activation dialog does NOT appear when onboarding hasn't been completed
   * (i.e., the wizard would show first instead).
   *
   * This verifies the onboardingCompleted gate is working.
   */
  test('does not appear before onboarding is completed', async ({ page }) => {
    await mockTauri(page, {});
    // onboardingCompleted defaults to false — updater flow is not triggered.
    await seedSettings(page, { onboardingCompleted: false, updateConsent: 'unset' });

    await page.goto('/');

    // Give the page time to settle; the dialog should NOT appear.
    await page.waitForTimeout(3_000);

    // No update activation dialog should be visible.
    // The onboarding wizard may open instead, but NOT the update activation dialog.
    // We identify the update activation dialog by its specific text.
    const updateDialogVisible = await page.getByText('Automatische Updates?').isVisible();
    expect(updateDialogVisible).toBe(false);
  });
});

test.describe('Update-available dialog', () => {
  /**
   * Update-available dialog shows the version when an update is present.
   *
   * Seed: onboardingCompleted=true, updateConsent='enabled', mock returns version 0.2.3.
   * Expected: text matching /0\.2\.3/ becomes visible in the dialog.
   */
  test('shows version number when update is available', async ({ page }) => {
    await mockTauri(page, { updateVersion: '0.2.3' });
    await seedSettings(page, { onboardingCompleted: true, updateConsent: 'enabled' });

    await page.goto('/');

    // The update-available dialog shows "Update verfügbar: Version 0.2.3"
    // which comes from the i18n key: availableTitle: (v) => `Update verfügbar: Version ${v}`
    await expect(page.getByText(/0\.2\.3/)).toBeVisible({ timeout: 15_000 });

    // Also verify the dialog role is present.
    const dialog = page.getByRole('dialog').first();
    await expect(dialog).toBeVisible({ timeout: 5_000 });
  });

  /**
   * Cross-viewport size check for the update-available dialog.
   * The panel should fit within the viewport on all screen sizes.
   */
  test('update-available dialog fits within viewport', async ({ page }) => {
    await mockTauri(page, { updateVersion: '0.2.3' });
    await seedSettings(page, { onboardingCompleted: true, updateConsent: 'enabled' });

    await page.goto('/');

    const dialog = page.getByRole('dialog').first();
    await expect(dialog).toBeVisible({ timeout: 15_000 });

    const viewport = page.viewportSize();
    if (viewport) {
      const box = await dialog.boundingBox();
      expect(box).not.toBeNull();
      if (box) {
        expect(box.x).toBeGreaterThanOrEqual(0);
        expect(box.y).toBeGreaterThanOrEqual(0);
        expect(box.x + box.width).toBeLessThanOrEqual(viewport.width + 1);
        expect(box.y + box.height).toBeLessThanOrEqual(viewport.height + 1);
      }
    }
  });

  /**
   * Closing the update-available dialog (via the close button) hides it.
   */
  test('close button hides the update-available dialog', async ({ page }) => {
    await mockTauri(page, { updateVersion: '0.2.3' });
    await seedSettings(page, { onboardingCompleted: true, updateConsent: 'enabled' });

    await page.goto('/');

    const dialog = page.getByRole('dialog').first();
    await expect(dialog).toBeVisible({ timeout: 15_000 });

    // The close button in the update-available dialog uses t().common.close text.
    // In German: "Schließen". Use a broader matcher for robustness.
    const closeBtn = page.getByRole('button', { name: /schlie[sß]en/i });
    await expect(closeBtn).toBeVisible({ timeout: 5_000 });
    await closeBtn.click();

    await expect(dialog).not.toBeVisible({ timeout: 5_000 });
  });
});
