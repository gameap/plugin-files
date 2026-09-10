import { expect, test, type APIRequestContext } from '@playwright/test';
import { loginViaAPI } from '../fixtures/auth';
import { env, ports } from '../fixtures/env';
import {
  CONFIG_PATH,
  FILES_SERVICE,
  nodeTarget,
} from '../fixtures/node/index';
import {
  enrolledNode,
  getDaemonTask,
  getDaemonTaskOutput,
} from '../fixtures/panel';
import { nodeStatus } from '../fixtures/plugin';
import { writeState } from '../fixtures/state';
import { fillNumber, nodeStatusTag, loginViaUI, openFilesAdmin } from '../fixtures/ui';
import { readScalar } from '../fixtures/yaml';

test.describe.configure({ mode: 'serial' });

// Deliberately below the plugin's own INSTALLING_TIMEOUT_SECS of 900, so a
// failure here reads as "the install did not finish" rather than as the
// plugin timing itself out.
const INSTALL_DEADLINE_MS = 10 * 60_000;

async function attachTaskOutput(
  request: APIRequestContext,
  token: string,
  taskId: number,
  testInfo: { attach: (name: string, options: { body: string; contentType: string }) => Promise<void> },
): Promise<void> {
  const output = await getDaemonTaskOutput(request, token, taskId);
  await testInfo.attach(`daemon-task-${taskId}`, {
    body: output,
    contentType: 'text/plain',
  });
}

test('the enrolled node is the OS this leg is testing', async ({ request }) => {
  const token = await loginViaAPI(request);
  const node = await enrolledNode(request, token, env.nodeOs);

  writeState({ nodeId: node.id });

  const status = await nodeStatus(request, token, node.id);
  expect(status.status).toBe('not_installed');
});

test('Install on the node card runs the installer to completion', async ({
  page,
  request,
}, testInfo) => {
  const token = await loginViaAPI(request);
  const node = await enrolledNode(request, token, env.nodeOs);

  await loginViaUI(page);
  await openFilesAdmin(page);

  await expect(nodeStatusTag(page, node.id)).toBeVisible();
  await page.getByTestId(`ftp-node-install-${node.id}`).click();

  const modal = page.getByRole('dialog').last();
  await fillNumber(modal, 'node-setup-ftp-port', ports.ftp);
  await fillNumber(modal, 'node-setup-passive-min', ports.passiveMin);
  await fillNumber(modal, 'node-setup-passive-max', ports.passiveMax);
  // Pinning the public host makes passive-mode data connections land on an
  // address the test host can reach, whatever the node's own idea of itself is.
  await modal.getByTestId('node-setup-public-host').fill(env.nodeHost);
  await fillNumber(modal, 'node-setup-sftp-port', ports.sftp);

  const started = page.waitForResponse(
    (response) =>
      response.url().includes('/nodes/') &&
      response.url().endsWith('/setup') &&
      response.request().method() === 'POST',
  );
  await modal.getByTestId('node-setup-submit').click();
  expect((await started).status()).toBe(200);

  // The admin page does not poll on its own, so the wait runs against the API
  // and the UI is re-checked once a terminal state is reached.
  let last = await nodeStatus(request, token, node.id);
  const deadline = Date.now() + INSTALL_DEADLINE_MS;
  while (last.status === 'installing' && Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, 5_000));
    last = await nodeStatus(request, token, node.id);
  }

  if (last.task_id) {
    await attachTaskOutput(request, token, last.task_id, testInfo);
  }

  expect(
    last.status,
    `install ended as ${last.status}: ${last.error_message ?? 'no message'}`,
  ).toBe('installed');
  expect(last.version).toMatch(/^\d+\.\d+\.\d+/);

  writeState({ filesVersion: last.version });
  await testInfo.attach('gameap-files-version', {
    body: JSON.stringify({ version: last.version, os: env.nodeOs }),
    contentType: 'application/json',
  });

  expect(last.task_id, 'the status keeps the install task id').toBeTruthy();
  const task = await getDaemonTask(request, token, last.task_id as number);
  expect(task.status).toBe('success');

  await page.getByTestId('ftp-nodes-refresh').click();
  await expect(nodeStatusTag(page, node.id)).toContainText(last.version ?? '');
});

test('gameap-files runs on the node as a service', () => {
  expect(nodeTarget().serviceState(FILES_SERVICE)).toBe('running');
});

test('the node binary reports the version the panel shows', async ({
  request,
}) => {
  const token = await loginViaAPI(request);
  const node = await enrolledNode(request, token, env.nodeOs);
  const status = await nodeStatus(request, token, node.id);

  expect(nodeTarget().filesBinaryVersion()).toContain(status.version ?? '');
});

test('the installer wrote the configuration the setup form asked for', () => {
  const node = nodeTarget();
  const config = node.readFile(CONFIG_PATH);

  expect(readScalar(config, 'ftp.listen_addr')).toBe(`:${ports.ftp}`);
  expect(readScalar(config, 'ftp.passive_port_min')).toBe(String(ports.passiveMin));
  expect(readScalar(config, 'ftp.passive_port_max')).toBe(String(ports.passiveMax));
  expect(readScalar(config, 'ftp.public_host')).toBe(env.nodeHost);
  expect(readScalar(config, 'sftp.listen_addr')).toBe(`:${ports.sftp}`);
  expect(readScalar(config, 'server.data_dir')).toBe(node.workPath);
});

test('the Windows service is registered through shawl', () => {
  test.skip(nodeTarget().os !== 'windows', 'Windows service wrapper');

  const result = nodeTarget().shell(`sc.exe qc ${FILES_SERVICE}`);
  expect(result.stdout.toLowerCase()).toContain('shawl.exe');
});
