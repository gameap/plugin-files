import { connect, type Socket } from 'node:net';
import { createRequire } from 'node:module';
import { Client as FtpClient } from 'basic-ftp';
import SftpClient from 'ssh2-sftp-client';

const require_ = createRequire(import.meta.url);

// ssh2's own key generator is used rather than node:crypto so the private key
// comes back in a format ssh2 itself parses without conversion. Loaded through
// createRequire because @types/ssh2 does not describe this helper.
const { utils } = require_('ssh2') as {
  utils: {
    generateKeyPairSync(
      type: 'ed25519' | 'rsa' | 'ecdsa',
    ): { public: string; private: string };
  };
};

export interface KeyPair {
  publicKey: string;
  privateKey: string;
}

export function generateSshKeyPair(comment: string): KeyPair {
  const pair = utils.generateKeyPairSync('ed25519');

  return {
    // The plugin validates the "ssh-ed25519 " prefix; the comment is cosmetic.
    publicKey: `${pair.public.trim()} ${comment}`,
    privateKey: pair.private,
  };
}

export async function waitForPort(
  host: string,
  port: number,
  timeoutMs = 60_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;

  while (Date.now() < deadline) {
    try {
      await new Promise<void>((resolve, reject) => {
        const socket: Socket = connect({ host, port });
        socket.setTimeout(5_000);
        socket.once('connect', () => {
          socket.destroy();
          resolve();
        });
        socket.once('timeout', () => {
          socket.destroy();
          reject(new Error('timeout'));
        });
        socket.once('error', (error) => {
          socket.destroy();
          reject(error);
        });
      });

      return;
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 1_000));
    }
  }

  throw new Error(`${host}:${port} never accepted a connection: ${lastError}`);
}

export async function expectPortClosed(
  host: string,
  port: number,
  timeoutMs = 30_000,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    const open = await new Promise<boolean>((resolve) => {
      const socket: Socket = connect({ host, port });
      socket.setTimeout(3_000);
      socket.once('connect', () => {
        socket.destroy();
        resolve(true);
      });
      socket.once('timeout', () => {
        socket.destroy();
        resolve(false);
      });
      socket.once('error', () => {
        socket.destroy();
        resolve(false);
      });
    });

    if (!open) {
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, 1_000));
  }

  throw new Error(`${host}:${port} is still accepting connections`);
}

export interface FtpCredentials {
  host: string;
  port: number;
  user: string;
  password: string;
}

export async function withFtp<T>(
  credentials: FtpCredentials,
  body: (client: FtpClient) => Promise<T>,
): Promise<T> {
  const client = new FtpClient(30_000);
  try {
    await client.access({
      host: credentials.host,
      port: credentials.port,
      user: credentials.user,
      password: credentials.password,
      secure: false,
    });

    return await body(client);
  } finally {
    client.close();
  }
}

export interface SftpCredentials {
  host: string;
  port: number;
  username: string;
  password?: string;
  privateKey?: string;
}

export async function withSftp<T>(
  credentials: SftpCredentials,
  body: (client: SftpClient) => Promise<T>,
): Promise<T> {
  const client = new SftpClient();
  try {
    await client.connect({
      host: credentials.host,
      port: credentials.port,
      username: credentials.username,
      ...(credentials.password ? { password: credentials.password } : {}),
      ...(credentials.privateKey ? { privateKey: credentials.privateKey } : {}),
      readyTimeout: 30_000,
    });

    return await body(client);
  } finally {
    await client.end();
  }
}

/** Resolves to the error a rejected login produced, or throws when it succeeded. */
export async function expectLoginRejected(
  credentials: FtpCredentials,
): Promise<string> {
  try {
    await withFtp(credentials, async (client) => client.list());
  } catch (error) {
    return String(error);
  }

  throw new Error(
    `FTP login as ${credentials.user} succeeded but should have been rejected`,
  );
}
