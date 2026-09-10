import { expect, type Locator, type Page } from '@playwright/test';
import { env, PLUGIN_ID } from './env';

// The SPA renders raw i18n keys when window.i18n is unpopulated, so every text
// matcher covers English, Russian and the raw key — the same convention the
// panel's own e2e fixtures use.
const SIGN_IN = /sign.?in|login|вход|войти|auth\.sign_in/i;
const UPLOAD = /upload|загруз|plugins\.upload/i;
const VALIDATE = /validate|проверить|plugins\.validate/i;
const CONFIRM = /delete|удалить|yes|да/i;
const CLOSE = /close|закрыть|main\.close/i;

// The server page mirrors its active tab in the location hash, and a plugin tab
// is named plugin-<pluginId>-<slotName>.
export const FTP_TAB_HASH = `#plugin-${PLUGIN_ID}-ftp-users`;

export async function loginViaUI(page: Page): Promise<void> {
  await page.goto('/login');
  await page.locator('#email').fill(env.adminUser);
  await page.locator('#password').fill(env.adminPassword);

  const login = page.waitForResponse(
    (response) =>
      response.url().includes('/api/auth/login') &&
      response.request().method() === 'POST',
  );
  await page.getByRole('button', { name: SIGN_IN }).click();

  const response = await login;
  expect(response.status(), `login as ${env.adminUser}`).toBe(200);

  await expect(page).not.toHaveURL(/\/login/, { timeout: 15_000 });
  await expect
    .poll(async () => page.evaluate(() => localStorage.getItem('auth_token')))
    .toBeTruthy();
}

// naive-ui's notification $dialog blocks the page until its single Close action
// is used. Scoped to the top-most dialog so a card-header close elsewhere on the
// page cannot match.
export async function dismissTopDialog(page: Page): Promise<void> {
  const close = page
    .getByRole('dialog')
    .last()
    .getByRole('button', { name: CLOSE });
  await expect(close).toBeVisible({ timeout: 10_000 });
  await close.click();
  await expect(close).toBeHidden({ timeout: 10_000 });
}

// naive-ui renders its dialog in a portal; scope to the top-most one so an
// unrelated close button elsewhere on the page cannot match.
export async function confirmDialog(page: Page): Promise<void> {
  const confirm = page
    .getByRole('dialog')
    .last()
    .getByRole('button', { name: CONFIRM });
  await expect(confirm).toBeVisible({ timeout: 10_000 });
  await confirm.click();
}

export async function uploadPluginViaUI(
  page: Page,
  wasmPath: string,
): Promise<void> {
  await page.goto('/admin/plugins');
  await page.getByRole('button', { name: UPLOAD }).first().click();

  // Only one dialog is open at this point, so `last()` is unambiguous here —
  // but not after the install, which is why the assertions below are anchored
  // on content of the upload modal instead.
  const modal = page.getByRole('dialog').last();
  await expect(modal).toBeVisible();

  // n-upload's dragger carries no test hook; the real file input inside it does
  // the work.
  await modal.locator('input[type="file"]').setInputFiles(wasmPath);
  await modal.getByRole('button', { name: VALIDATE }).click();

  const permissions = page.getByTestId('dry-run-permissions');
  await expect(permissions).toBeVisible({ timeout: 60_000 });

  const installed = page.waitForResponse(
    (response) =>
      response.url().includes('/api/admin/plugins/upload/install') &&
      response.request().method() === 'POST',
  );
  await page.getByTestId('upload-install-button').click();

  const response = await installed;
  expect(response.ok(), `install: ${response.status()} ${await response.text()}`)
    .toBe(true);

  // Installing closes the upload modal and stacks a success $dialog on top of
  // the page, so waiting on whichever dialog is last would wait on that one
  // forever.
  await expect(permissions).toBeHidden({ timeout: 60_000 });
  await dismissTopDialog(page);
}

export function nodeCard(page: Page, nodeId: number): Locator {
  return page.getByTestId(`ftp-node-card-${nodeId}`);
}

export function nodeStatusTag(page: Page, nodeId: number): Locator {
  return page.getByTestId(`ftp-node-status-${nodeId}`);
}

export async function openFilesAdmin(page: Page): Promise<void> {
  await page.goto(`/plugins/${PLUGIN_ID}/`);
  await expect(page.getByTestId('ftp-nodes-refresh')).toBeVisible({
    timeout: 30_000,
  });
}

export async function openServerFtpTab(
  page: Page,
  serverId: number,
): Promise<void> {
  await page.goto(`/servers/${serverId}${FTP_TAB_HASH}`);
  await expect(page.getByTestId('ftp-users-tab')).toBeVisible({
    timeout: 30_000,
  });
}

// naive-ui number inputs keep the typed value in a plain <input>; fill + blur is
// what commits it to the model.
export async function fillNumber(
  scope: Page | Locator,
  testId: string,
  value: number,
): Promise<void> {
  const input = scope.getByTestId(testId);
  await input.fill(String(value));
  await input.blur();
}
