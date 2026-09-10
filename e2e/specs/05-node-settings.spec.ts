import { expect, request as apiRequest, test } from '@playwright/test';
import { loginViaAPI } from '../fixtures/auth';
import { env, ports } from '../fixtures/env';
import { expectPortClosed, waitForPort, withFtp } from '../fixtures/ftp';
import { CONFIG_PATH, nodeTarget } from '../fixtures/node/index';
import { enrolledNode } from '../fixtures/panel';
import { updateNodeConfig } from '../fixtures/plugin';
import { readState } from '../fixtures/state';
import { fillNumber, loginViaUI, openFilesAdmin } from '../fixtures/ui';
import { readScalar, scalarKeys } from '../fixtures/yaml';

test.describe.configure({ mode: 'serial' });

// Everything in config.yaml the plugin is not allowed to touch. Anything else
// the installer wrote has to survive a settings change untouched.
const OWNED_KEYS = new Set([
  'ftp.listen_addr',
  'ftp.passive_port_min',
  'ftp.passive_port_max',
  'ftp.public_host',
  'ftp.tls.enabled',
  'ftp.tls.implicit_port',
  'sftp.listen_addr',
]);

let before = '';

async function changePorts(
  page: import('@playwright/test').Page,
  nodeId: number,
  ftpPort: number,
  sftpPort: number,
): Promise<void> {
  await openFilesAdmin(page);
  await page.getByTestId(`ftp-node-settings-${nodeId}`).click();

  const modal = page.getByRole('dialog').last();
  await expect(modal.getByTestId('node-setup-ftp-port')).toHaveValue(/\d+/);

  await fillNumber(modal, 'node-setup-ftp-port', ftpPort);
  await fillNumber(modal, 'node-setup-sftp-port', sftpPort);

  const saved = page.waitForResponse(
    (response) =>
      response.url().includes('/nodes/') &&
      response.url().endsWith('/config') &&
      response.request().method() === 'PUT',
  );
  await modal.getByTestId('node-setup-submit').click();
  expect((await saved).status()).toBe(200);
}

// The restore has to be independent of whether the rollback test below ran or
// passed: the specs after this one connect on the original ports, and a locally
// provisioned node is reused between runs.
test.afterAll(async () => {
  const context = await apiRequest.newContext();
  try {
    const token = await loginViaAPI(context);
    const node = await enrolledNode(context, token, env.nodeOs);
    await updateNodeConfig(context, token, node.id, {
      ftp: { port: ports.ftp },
      sftp: { port: ports.sftp },
    });
    await waitForPort(env.nodeHost, ports.ftp);
  } catch (error) {
    console.warn(`could not restore the node ports: ${error}`);
  } finally {
    await context.dispose();
  }
});

test('Settings moves the listeners to new ports', async ({ page, request }) => {
  const token = await loginViaAPI(request);
  const node = await enrolledNode(request, token, env.nodeOs);

  before = nodeTarget().readFile(CONFIG_PATH);

  await loginViaUI(page);
  await changePorts(page, node.id, ports.ftpChanged, ports.sftpChanged);

  const after = nodeTarget().readFile(CONFIG_PATH);
  expect(readScalar(after, 'ftp.listen_addr')).toBe(`:${ports.ftpChanged}`);
  expect(readScalar(after, 'sftp.listen_addr')).toBe(`:${ports.sftpChanged}`);
});

test('keys the plugin does not own survive the patch', () => {
  const after = nodeTarget().readFile(CONFIG_PATH);

  const unchanged = scalarKeys(before).filter((key) => !OWNED_KEYS.has(key));
  expect(unchanged.length, 'the installer writes more than the owned keys')
    .toBeGreaterThan(5);

  for (const key of unchanged) {
    expect(readScalar(after, key), `${key} must not be rewritten`).toBe(
      readScalar(before, key),
    );
  }
});

test('the restarted service answers on the new port and not the old one', async () => {
  const state = readState();

  await waitForPort(env.nodeHost, ports.ftpChanged);
  await expectPortClosed(env.nodeHost, ports.ftp);

  const listing = await withFtp(
    {
      host: env.nodeHost,
      port: ports.ftpChanged,
      user: state.ftpUser as string,
      password: state.ftpPassword as string,
    },
    async (client) => client.list(),
  );

  expect(listing.length).toBeGreaterThanOrEqual(1);
});

// The rollback is asserted through the same dialog an operator would use; the
// afterAll above is the safety net for when this test never gets that far.
test('the change can be rolled back from the same dialog', async ({
  page,
  request,
}) => {
  const token = await loginViaAPI(request);
  const node = await enrolledNode(request, token, env.nodeOs);

  await loginViaUI(page);
  await changePorts(page, node.id, ports.ftp, ports.sftp);

  await waitForPort(env.nodeHost, ports.ftp);
  await waitForPort(env.nodeHost, ports.sftp);

  const after = nodeTarget().readFile(CONFIG_PATH);
  expect(readScalar(after, 'ftp.listen_addr')).toBe(`:${ports.ftp}`);
  expect(readScalar(after, 'sftp.listen_addr')).toBe(`:${ports.sftp}`);
});
