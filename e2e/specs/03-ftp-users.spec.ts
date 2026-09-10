import { expect, test } from '@playwright/test';
import { loginViaAPI } from '../fixtures/auth';
import { env, names } from '../fixtures/env';
import { generateSshKeyPair } from '../fixtures/ftp';
import { nodeTarget, userDropIn } from '../fixtures/node/index';
import {
  createGameMod,
  createServer,
  enrolledNode,
  getServer,
  markServerInstalled,
  seedGame,
} from '../fixtures/panel';
import { getUser, listSshKeys, listUsers } from '../fixtures/plugin';
import { readState, writeState } from '../fixtures/state';
import { readScalar } from '../fixtures/yaml';
import { loginViaUI, openServerFtpTab } from '../fixtures/ui';

test.describe.configure({ mode: 'serial' });

// The game server is provisioned through the panel API rather than the UI: the
// plugin only needs a server record with a dir on the node, and a real game
// install would add ten minutes and a steamcmd dependency for nothing.
test('a game server exists on the node', async ({ request }) => {
  const token = await loginViaAPI(request);
  const node = await enrolledNode(request, token, env.nodeOs);

  await seedGame(request, token, {
    code: names.gameCode,
    name: names.gameName,
    engine: 'other',
    engine_version: '1',
  });

  const modId = await createGameMod(request, token, {
    game_code: names.gameCode,
    name: names.modName,
    start_cmd_linux: './start.sh',
  });

  const serverId = await createServer(request, token, {
    name: names.serverName,
    ds_id: node.id,
    game_id: names.gameCode,
    game_mod_id: modId,
    server_ip: '127.0.0.1',
    server_port: 27015,
  });

  await markServerInstalled(request, token, serverId);
  const server = await getServer(request, token, serverId);

  expect(server.dir).toBeTruthy();
  writeState({ serverId, serverDir: server.dir, nodeId: node.id });
});

test('creating a user from the server tab mirrors it to the node', async ({
  page,
}) => {
  const { serverId, serverDir } = readState();
  expect(serverId).toBeDefined();

  await loginViaUI(page);
  await openServerFtpTab(page, serverId as number);

  await expect(page.getByTestId('node-status-card')).toBeVisible();
  await page.getByTestId('ftp-create-user').click();

  const modal = page.getByRole('dialog').last();
  await modal.getByTestId('ftp-user-username').fill(names.ftpUser);
  // An explicit password keeps it out of the one-shot PasswordModal, which is
  // the only place a generated one is ever shown.
  await modal.getByTestId('ftp-user-password').fill(names.ftpPassword);
  await modal.getByTestId('ftp-user-submit').click();

  await expect(page.getByTestId(`ftp-user-row-${names.ftpUser}`)).toBeVisible({
    timeout: 30_000,
  });

  writeState({ ftpUser: names.ftpUser, ftpPassword: names.ftpPassword });

  const node = nodeTarget();
  const dropIn = userDropIn(names.ftpUser);
  await expect
    .poll(() => node.exists(dropIn), { timeout: 30_000 })
    .toBe(true);

  const content = node.readFile(dropIn);
  expect(readScalar(content, 'username')).toBe(names.ftpUser);
  expect(readScalar(content, 'password_hash')).toMatch(/^\$argon2id\$/);
  // A Windows path may come back as a double-quoted YAML scalar with escaped
  // separators, so the backslashes are collapsed before comparing.
  const homeDir = (readScalar(content, 'home_dir') ?? '').replace(/\\\\/g, '\\');
  expect(homeDir).toBe(node.join(serverDir as string));
});

test('the drop-in is only readable by the service account', () => {
  const node = nodeTarget();
  test.skip(node.mode(userDropIn(names.ftpUser)) === null, 'POSIX mode');

  expect(node.mode(userDropIn(names.ftpUser))).toBe('600');
});

test('the new user got the default access rules', async ({ request }) => {
  const token = await loginViaAPI(request);
  const serverId = readState().serverId as number;

  const user = await getUser(request, token, serverId, names.ftpUser);

  expect(user.enabled).toBe(true);
  expect(user.access_rules).toEqual([
    { path: '/**', permissions: ['read', 'write', 'delete', 'list'] },
  ]);
});

test('an SSH public key added in the edit modal reaches the node', async ({
  page,
  request,
}) => {
  const serverId = readState().serverId as number;
  const pair = generateSshKeyPair(`${names.ftpUser}@e2e`);

  await loginViaUI(page);
  await openServerFtpTab(page, serverId);

  await page.getByTestId(`ftp-user-edit-${names.ftpUser}`).click();

  const modal = page.getByRole('dialog').last();
  await modal.getByTestId('ssh-key-add').click();
  await modal.getByTestId('ssh-key-input').fill(pair.publicKey);
  await modal.getByTestId('ssh-key-submit').click();

  const token = await loginViaAPI(request);
  await expect
    .poll(
      async () =>
        (await listSshKeys(request, token, serverId, names.ftpUser)).keys.length,
      { timeout: 30_000 },
    )
    .toBe(1);

  writeState({ sshPublicKey: pair.publicKey, sshPrivateKey: pair.privateKey });

  const content = nodeTarget().readFile(userDropIn(names.ftpUser));
  expect(content).toContain(pair.publicKey.split(' ')[1]);
});

test('the user is listed for the server', async ({ request }) => {
  const token = await loginViaAPI(request);
  const serverId = readState().serverId as number;

  const users = await listUsers(request, token, serverId);

  expect(users.map((user) => user.username)).toContain(names.ftpUser);
});
