import type { APIRequestContext } from '@playwright/test';
import { authHeader } from './auth';
import { env, PLUGIN_ID } from './env';

const BASE = `/api/plugins/${PLUGIN_ID}`;

export interface PluginResponse<T> {
  status: number;
  body: T;
  text: string;
}

export async function call<T>(
  request: APIRequestContext,
  method: 'get' | 'post' | 'put' | 'delete',
  path: string,
  token: string,
  data?: unknown,
): Promise<PluginResponse<T>> {
  const response = await request[method](`${env.apiBaseUrl}${BASE}${path}`, {
    headers: data
      ? { ...authHeader(token), 'Content-Type': 'application/json' }
      : authHeader(token),
    ...(data === undefined ? {} : { data }),
  });

  const text = await response.text();

  return {
    status: response.status(),
    body: (text ? JSON.parse(text) : null) as T,
    text,
  };
}

async function ok<T>(
  request: APIRequestContext,
  method: 'get' | 'post' | 'put' | 'delete',
  path: string,
  token: string,
  data?: unknown,
): Promise<T> {
  const response = await call<T>(request, method, path, token, data);
  if (response.status < 200 || response.status >= 300) {
    throw new Error(
      `${method.toUpperCase()} ${BASE}${path} failed: ${response.status} ${response.text}`,
    );
  }

  return response.body;
}

export interface FtpConfigPatch {
  address?: string;
  port?: number;
  passive_port_min?: number;
  passive_port_max?: number;
  public_host?: string;
  tls_enabled?: boolean;
  tls_implicit_port?: number;
}

export interface NodeSetupRequest {
  ftp?: FtpConfigPatch;
  sftp?: { port?: number };
}

export interface NodeConfig {
  ftp?: {
    address: string;
    port: number;
    passive_port_min: number;
    passive_port_max: number;
    public_host: string;
    tls_enabled: boolean;
    tls_implicit_port: number;
  };
  sftp?: { port: number };
}

// status is one of not_installed | installing | installed | failed; version,
// task_id, error_message and last_check are omitted when empty or zero.
export interface NodeStatus {
  status: 'not_installed' | 'installing' | 'installed' | 'failed';
  version?: string;
  task_id?: number;
  error_message?: string;
  last_check?: number;
}

export function setupNode(
  request: APIRequestContext,
  token: string,
  nodeId: number,
  body: NodeSetupRequest = {},
): Promise<NodeStatus> {
  return ok<NodeStatus>(request, 'post', `/nodes/${nodeId}/setup`, token, body);
}

export function nodeStatus(
  request: APIRequestContext,
  token: string,
  nodeId: number,
): Promise<NodeStatus> {
  return ok<NodeStatus>(request, 'get', `/nodes/${nodeId}/status`, token);
}

export function nodeConfig(
  request: APIRequestContext,
  token: string,
  nodeId: number,
): Promise<NodeConfig> {
  return ok<NodeConfig>(request, 'get', `/nodes/${nodeId}/config`, token);
}

export function updateNodeConfig(
  request: APIRequestContext,
  token: string,
  nodeId: number,
  body: NodeSetupRequest,
): Promise<NodeConfig> {
  return ok<NodeConfig>(request, 'put', `/nodes/${nodeId}/config`, token, body);
}

export interface AccessRule {
  path: string;
  permissions: string[];
}

export interface VirtualPath {
  virtual: string;
  target: string;
  permissions: string[];
  read_only: boolean;
}

export interface FtpUserResponse {
  username: string;
  home_dir: string;
  quota_bytes: number;
  enabled: boolean;
  description: string;
  ssh_keys_count: number;
  access_rules: AccessRule[];
  virtual_paths: VirtualPath[];
}

export function listUsers(
  request: APIRequestContext,
  token: string,
  serverId: number,
): Promise<FtpUserResponse[]> {
  return ok<FtpUserResponse[]>(
    request,
    'get',
    `/servers/${serverId}/ftp-users`,
    token,
  );
}

export function getUser(
  request: APIRequestContext,
  token: string,
  serverId: number,
  username: string,
): Promise<FtpUserResponse> {
  return ok<FtpUserResponse>(
    request,
    'get',
    `/servers/${serverId}/ftp-users/${username}`,
    token,
  );
}

export function listSshKeys(
  request: APIRequestContext,
  token: string,
  serverId: number,
  username: string,
): Promise<{ keys: string[] }> {
  return ok<{ keys: string[] }>(
    request,
    'get',
    `/servers/${serverId}/ftp-users/${username}/ssh-keys`,
    token,
  );
}

export function adminNodes(
  request: APIRequestContext,
  token: string,
): Promise<PluginResponse<unknown>> {
  return call<unknown>(request, 'get', '/admin/nodes', token);
}
