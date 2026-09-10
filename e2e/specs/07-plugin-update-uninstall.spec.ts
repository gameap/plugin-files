import { expect, test } from '@playwright/test';
import { loginViaAPI } from '../fixtures/auth';
import { env, PLUGIN_ID, REQUIRED_PERMISSIONS } from '../fixtures/env';
import { findLoadedPlugin, uninstallPlugin, updatePlugin } from '../fixtures/panel';
import { adminNodes } from '../fixtures/plugin';
import { loginViaUI } from '../fixtures/ui';

test.describe.configure({ mode: 'serial' });

test('re-uploading the same wasm updates in place and grants nothing new', async ({
  request,
}) => {
  const token = await loginViaAPI(request);
  const before = await findLoadedPlugin(request, token, PLUGIN_ID);

  const result = await updatePlugin(request, token, PLUGIN_ID, env.wasmPath);
  expect(result.status, result.body).toBeGreaterThanOrEqual(200);
  expect(result.status, result.body).toBeLessThan(300);

  const after = await findLoadedPlugin(request, token, PLUGIN_ID);
  expect(after?.version).toBe(before?.version);
  expect([...(after?.allowed_permissions ?? [])].sort()).toEqual([
    ...REQUIRED_PERMISSIONS,
  ]);
});

// The uninstall itself goes through the API: the panel's own table is the
// panel's test surface, while what this suite owns is what disappears when the
// plugin is gone.
test('uninstalling removes the routes and the admin page', async ({
  page,
  request,
}) => {
  const token = await loginViaAPI(request);

  expect(await uninstallPlugin(request, token, PLUGIN_ID)).toBe(204);
  expect(await findLoadedPlugin(request, token, PLUGIN_ID)).toBeUndefined();

  const response = await adminNodes(request, token);
  expect(response.status).toBe(404);

  await loginViaUI(page);
  await page.goto(`/plugins/${PLUGIN_ID}/`);
  await expect(page.getByTestId('ftp-nodes-refresh')).toBeHidden({
    timeout: 15_000,
  });
});
