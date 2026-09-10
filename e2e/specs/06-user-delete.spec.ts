import { expect, test } from '@playwright/test';
import { loginViaAPI } from '../fixtures/auth';
import { env, ports } from '../fixtures/env';
import { expectLoginRejected } from '../fixtures/ftp';
import { nodeTarget, userDropIn } from '../fixtures/node/index';
import { listUsers } from '../fixtures/plugin';
import { readState } from '../fixtures/state';
import { confirmDialog, loginViaUI, openServerFtpTab } from '../fixtures/ui';

test.describe.configure({ mode: 'serial' });

test('deleting the user from the tab removes it everywhere', async ({
  page,
  request,
}) => {
  const state = readState();
  const serverId = state.serverId as number;
  const username = state.ftpUser as string;

  await loginViaUI(page);
  await openServerFtpTab(page, serverId);

  await page.getByTestId(`ftp-user-delete-${username}`).click();
  await confirmDialog(page);

  await expect(page.getByTestId(`ftp-user-row-${username}`)).toBeHidden({
    timeout: 30_000,
  });

  const token = await loginViaAPI(request);
  const users = await listUsers(request, token, serverId);
  expect(users.map((user) => user.username)).not.toContain(username);
});

test('the node drop-in is gone', async () => {
  const node = nodeTarget();
  const username = readState().ftpUser as string;

  await expect
    .poll(() => node.exists(userDropIn(username)), { timeout: 30_000 })
    .toBe(false);
});

test('the deleted user can no longer log in', async () => {
  const state = readState();

  const error = await expectLoginRejected({
    host: env.nodeHost,
    port: ports.ftp,
    user: state.ftpUser as string,
    password: state.ftpPassword as string,
  });

  expect(error).toMatch(/530|login|authentication/i);
});
