import type { NodeOs } from '../env';
import { env } from '../env';
import { DockerNode } from './docker';
import { WindowsNode } from './windows';

export interface CommandResult {
  code: number;
  stdout: string;
  stderr: string;
}

export type ServiceState = 'running' | 'stopped' | 'absent';

// The one seam between the specs and the two kinds of node. Everything else the
// suite touches is OS-neutral because the plugin is: the same REST paths, the
// same work-path-relative node paths, the same service name.
export interface NodeTarget {
  readonly os: NodeOs;
  /** Address the FTP and SFTP clients connect to. */
  readonly host: string;
  /** Daemon work path: /srv/gameap or C:\gameap. */
  readonly workPath: string;
  /** Mirrors the plugin's join_node_path for a work-path-relative entry. */
  join(relative: string): string;
  /** Escape hatch: bash on Linux, PowerShell on Windows. */
  shell(script: string): CommandResult;
  readFile(relative: string): string;
  exists(relative: string): boolean;
  listDir(relative: string): string[];
  /** POSIX mode as three digits, null on Windows. */
  mode(relative: string): string | null;
  serviceState(name: string): ServiceState;
  filesBinaryVersion(): string;
  listeningPorts(): number[];
}

let cached: NodeTarget | undefined;

export function nodeTarget(): NodeTarget {
  if (!cached) {
    cached =
      env.nodeOs === 'windows'
        ? new WindowsNode(env.nodeHost, env.nodeWorkPath)
        : new DockerNode(env.nodeContainer, env.nodeHost, env.nodeWorkPath);
  }

  return cached;
}

export const FILES_SERVICE = 'gameap-files';
export const CONFIG_PATH = '.plugins/files/config.yaml';
export const USERS_DIR = '.plugins/files/users.d';

export function userDropIn(username: string): string {
  return `${USERS_DIR}/${username}.yaml`;
}

export { isAbsoluteNodePath, joinNodePath } from './paths';
