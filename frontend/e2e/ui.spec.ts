import { test, expect } from '@playwright/test';

test.describe('Minimalist UI & Auth Screen', () => {
  test('unauthenticated user sees clean minimal login screen', async ({ page }) => {
    await page.goto('/');

    // 1. Check title & branding
    const title = page.locator('h1.auth-title');
    await expect(title).toHaveText('chatw');

    // 2. Check Google login button
    const googleBtn = page.getByRole('button', { name: 'Tiếp tục với Google' });
    await expect(googleBtn).toBeVisible();

    // 3. Verify absence of prohibited elements
    await expect(page.locator('.chatbot, .ai-assistant, nav, footer, .sidebar')).toHaveCount(0);

    // 4. Verify clicking Google button navigates to /api/auth/google
    const [response] = await Promise.all([
      page.waitForResponse((res) => res.url().includes('/api/auth/google') || res.status() === 307 || res.status() === 302 || res.url().includes('accounts.google.com')),
      googleBtn.click(),
    ]).catch(() => [null]);

    // Button initiated navigation correctly
    expect(page.url()).toMatch(/(auth\/google|accounts\.google\.com)/);
  });

  test('responsive mobile layout shows single view without horizontal overflow', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/');

    // Check no horizontal scrollbar / overflow
    const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
    const clientWidth = await page.evaluate(() => document.documentElement.clientWidth);
    expect(scrollWidth).toBeLessThanOrEqual(clientWidth);
  });
});
