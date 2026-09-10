import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { env } from './env';

// Spec files share one worker but not one module registry across a worker
// restart, and an SSH key pair cannot be re-derived from the run id. What has
// to survive between specs is written here instead.
export interface SuiteState {
  serverId?: number;
  serverDir?: string;
  nodeId?: number;
  ftpUser?: string;
  ftpPassword?: string;
  sshPublicKey?: string;
  sshPrivateKey?: string;
  filesVersion?: string;
}

const statePath = join(env.stateDir, 'state.json');

export function readState(): SuiteState {
  if (!existsSync(statePath)) {
    return {};
  }

  return JSON.parse(readFileSync(statePath, 'utf8')) as SuiteState;
}

export function writeState(patch: SuiteState): SuiteState {
  const merged = { ...readState(), ...patch };
  mkdirSync(dirname(statePath), { recursive: true });
  writeFileSync(statePath, `${JSON.stringify(merged, null, 2)}\n`, {
    mode: 0o600,
  });

  return merged;
}

export function requireState<K extends keyof SuiteState>(
  key: K,
): NonNullable<SuiteState[K]> {
  const value = readState()[key];
  if (value === undefined || value === null) {
    throw new Error(
      `${String(key)} is not in the suite state; the spec that sets it has to run first`,
    );
  }

  return value as NonNullable<SuiteState[K]>;
}
