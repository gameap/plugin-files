import { expect, test } from '@playwright/test';
import { loginViaAPI } from '../fixtures/auth';
import { env, PLUGIN_ID, REQUIRED_PERMISSIONS } from '../fixtures/env';
import {
  findLoadedPlugin,
  loadedPlugins,
  pluginDryRun,
} from '../fixtures/panel';
import { loginViaUI, openFilesAdmin, uploadPluginViaUI } from '../fixtures/ui';

test.describe.configure({ mode: 'serial' });

test('dry-run reports the manifest of the built wasm', async ({ request }) => {
  const token = await loginViaAPI(request);
  const result = await pluginDryRun(request, token, env.wasmPath);

  expect(result.id).toBe(PLUGIN_ID);
  expect(result.is_valid).toBe(true);
  expect(result.errors ?? []).toEqual([]);
  expect(result.installed).toBe(false);
  expect(result.has_frontend_bundle).toBe(true);
  expect(result.version).toMatch(/^\d+\.\d+\.\d+/);
  expect([...(result.required_permissions ?? [])].sort()).toEqual([
    ...REQUIRED_PERMISSIONS,
  ]);
});

test('uploading through the panel UI installs the plugin', async ({
  page,
  request,
}) => {
  await loginViaUI(page);
  await uploadPluginViaUI(page, env.wasmPath);

  const token = await loginViaAPI(request);
  const { data, permissions_enforced } = await loadedPlugins(request, token);

  // The assertions on missing_permissions below only mean something while the
  // panel is enforcing grants.
  expect(permissions_enforced, 'PLUGINS_PERMISSIONS_ENFORCE must be on').toBe(
    true,
  );

  const plugin = data.find((entry) => entry.id === PLUGIN_ID);
  expect(plugin, `plugin ${PLUGIN_ID} is not in ${JSON.stringify(data.map((p) => p.id))}`)
    .toBeDefined();
  expect(plugin?.status).toBe('active');
  expect(plugin?.loaded).toBe(true);
  expect(plugin?.enabled).toBe(true);
  expect(plugin?.has_frontend_bundle).toBe(true);
});

test('installing granted exactly the declared permissions', async ({
  request,
}) => {
  const token = await loginViaAPI(request);
  const plugin = await findLoadedPlugin(request, token, PLUGIN_ID);

  expect([...(plugin?.allowed_permissions ?? [])].sort()).toEqual([
    ...REQUIRED_PERMISSIONS,
  ]);
  expect(plugin?.missing_permissions ?? ['<null>']).toEqual([]);
});

test('the plugin exposes its routes and server abilities', async ({
  request,
}) => {
  const token = await loginViaAPI(request);
  const plugin = await findLoadedPlugin(request, token, PLUGIN_ID);

  const routes = (plugin?.http_routes ?? []).map((route) => route.path);
  expect(routes).toContain('/nodes/{nodeId}/setup');
  expect(routes).toContain('/nodes/{nodeId}/status');
  expect(routes).toContain('/servers/{serverId}/ftp-users');

  const abilities = (plugin?.server_abilities ?? []).map((ability) => ability.name);
  expect(abilities).toContain('ftp-users-view');
  expect(abilities).toContain('ftp-users-manage');
});

test('the wasm-embedded frontend bundle renders the admin page', async ({
  page,
}) => {
  await loginViaUI(page);
  // Waits for the page's own Refresh button, which exists only once
  // /plugins.js delivered the bundle and the route registered.
  await openFilesAdmin(page);

  // And the node the provisioning enrolled has to reach the grid, which proves
  // the plugin's API answers too.
  await expect(page.getByTestId('ftp-nodes-grid')).toBeVisible();
});
