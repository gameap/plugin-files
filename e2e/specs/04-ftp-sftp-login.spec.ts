import { Readable, Writable } from 'node:stream';
import { expect, test } from '@playwright/test';
import { env, ports } from '../fixtures/env';
import {
  expectLoginRejected,
  waitForPort,
  withFtp,
  withSftp,
} from '../fixtures/ftp';
import { nodeTarget } from '../fixtures/node/index';
import { readState } from '../fixtures/state';

test.describe.configure({ mode: 'serial' });

const REMOTE_NAME = 'e2e-upload.txt';
const PAYLOAD = `uploaded by the plugin-files e2e suite, run ${env.runId}\n`;

function credentials() {
  const state = readState();

  return {
    host: env.nodeHost,
    user: state.ftpUser as string,
    password: state.ftpPassword as string,
  };
}

function collect(chunks: Buffer[]): Writable {
  return new Writable({
    write(chunk, _encoding, done) {
      chunks.push(Buffer.from(chunk));
      done();
    },
  });
}

test('the panel-created user can log in over FTP and transfer a file', async () => {
  await waitForPort(env.nodeHost, ports.ftp);

  const { host, user, password } = credentials();
  const downloaded: Buffer[] = [];

  const listing = await withFtp(
    { host, port: ports.ftp, user, password },
    async (client) => {
      await client.uploadFrom(Readable.from([PAYLOAD]), REMOTE_NAME);
      await client.downloadTo(collect(downloaded), REMOTE_NAME);

      return client.list();
    },
  );

  expect(listing.map((entry) => entry.name)).toContain(REMOTE_NAME);
  expect(Buffer.concat(downloaded).toString('utf8')).toBe(PAYLOAD);
});

test('the uploaded file is on the node filesystem inside the server directory', () => {
  const node = nodeTarget();
  const serverDir = readState().serverDir as string;

  expect(node.readFile(`${serverDir}/${REMOTE_NAME}`)).toBe(PAYLOAD);
});

test('a wrong password is refused', async () => {
  const { host, user } = credentials();

  const error = await expectLoginRejected({
    host,
    port: ports.ftp,
    user,
    password: 'definitely-not-the-password',
  });

  expect(error).toMatch(/530|login|authentication/i);
});

test('the same user can log in over SFTP with a password', async () => {
  await waitForPort(env.nodeHost, ports.sftp);

  const { host, user, password } = credentials();
  const listing = await withSftp(
    { host, port: ports.sftp, username: user, password },
    async (client) => client.list('.'),
  );

  expect(listing.map((entry) => entry.name)).toContain(REMOTE_NAME);
});

test('the SSH key added in the panel authenticates without a password', async () => {
  const state = readState();
  expect(state.sshPrivateKey, 'spec 03 has to add the key first').toBeTruthy();

  const listing = await withSftp(
    {
      host: env.nodeHost,
      port: ports.sftp,
      username: state.ftpUser as string,
      privateKey: state.sshPrivateKey as string,
    },
    async (client) => client.list('.'),
  );

  expect(listing.map((entry) => entry.name)).toContain(REMOTE_NAME);
});
