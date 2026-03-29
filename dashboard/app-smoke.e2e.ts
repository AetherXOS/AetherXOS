import { test, expect } from '@playwright/test';

test('dashboard shows global guidance shell', async ({ page }) => {
	await page.goto('/executive');
	await expect(page.getByText(/Now/i)).toBeVisible();
	await expect(page.getByText(/Next/i)).toBeVisible();
	await expect(page.getByText(/Blocker/i)).toBeVisible();
});

test('operations exposes runbook architecture and filters', async ({ page }) => {
	await page.goto('/operations');
	await expect(page.getByText(/Runbook|Architecture/i)).toBeVisible();
	await expect(page.getByText(/active:/i)).toBeVisible();
});
