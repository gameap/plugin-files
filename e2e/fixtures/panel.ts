import { readFileSync } from 'node:fs';
import { basename } from 'node:path';
import type { APIRequestContext } from '@playwright/test';
import { authHeader } from './auth';
import { env } from './env';

async function json<T>(
  request: APIRequestContext,
  method: 'get' | 'post' | 'put' | 'delete',
  path: string,
  token: string,
  data?: unknown,
): Promise<T> {
  const response = await request[method](`${env.apiBaseUrl}${path}`, {
    headers: data
      ? { ...authHeader(token), 'Content-Type': 'application/json' }
      : authHeader(token),
    ...(data === undefined ? {} : { data }),
  });

  if (!response.ok()) {
    throw new Error(
      `${method.toUpperCase()} ${path} failed: ${response.status()} ${await response.text()}`,
    );
  }

  const text = await response.text();

  return (text ? JSON.parse(text) : null) as T;
}

export interface NodeRecord {
  id: number;
  enabled: boolean;
  name: string;
  os: string;
  ip: string[];
}

export function listNodes(
  request: APIRequestContext,
  token: string,
): Promise<NodeRecord[]> {
  return json<NodeRecord[]>(request, 'get', '/api/nodes', token);
}

export interface NodesSummary {
  total: number;
  online: number;
  offline: number;
}

export function nodesSummary(
  request: APIRequestContext,
  token: string,
): Promise<NodesSummary> {
  return json<NodesSummary>(request, 'get', '/api/nodes/summary', token);
}

// The node the daemon enrolled itself as. There is no create-node endpoint, so
// the suite always works with whatever the provisioning step enrolled.
export async function enrolledNode(
  request: APIRequestContext,
  token: string,
  os: string,
): Promise<NodeRecord> {
  const nodes = await listNodes(request, token);
  const node = nodes.find((n) => n.enabled && n.os === os);
  if (!node) {
    throw new Error(
      `no enabled ${os} node enrolled: ${JSON.stringify(nodes)}`,
    );
  }

  return node;
}

export interface GameDefinition {
  code: string;
  name: string;
  engine: string;
  engine_version?: string;
}

export async function seedGame(
  request: APIRequestContext,
  token: string,
  game: GameDefinition,
): Promise<void> {
  const response = await request.post(`${env.apiBaseUrl}/api/games`, {
    headers: { ...authHeader(token), 'Content-Type': 'application/json' },
    data: game,
  });

  if (response.ok() || response.status() === 409) {
    return;
  }

  throw new Error(
    `seed game failed: ${response.status()} ${await response.text()}`,
  );
}

export interface GameModRecord {
  id: number;
  game_code: string;
  name: string;
}

export function listGameMods(
  request: APIRequestContext,
  token: string,
  gameCode: string,
): Promise<GameModRecord[]> {
  return json<GameModRecord[]>(
    request,
    'get',
    `/api/game_mods/get_list_for_game/${gameCode}`,
    token,
  );
}

// POST /api/game_mods answers {status:"ok"} without an id, so the id is always
// read back from the list; that also makes a re-run tolerate the duplicate.
export async function createGameMod(
  request: APIRequestContext,
  token: string,
  mod: { game_code: string; name: string; start_cmd_linux?: string },
): Promise<number> {
  const response = await request.post(`${env.apiBaseUrl}/api/game_mods`, {
    headers: { ...authHeader(token), 'Content-Type': 'application/json' },
    data: mod,
  });

  const existing = await listGameMods(request, token, mod.game_code);
  const found = existing.find((m) => m.name === mod.name);
  if (found) {
    return found.id;
  }

  throw new Error(
    `create game mod failed: ${response.status()} ${await response.text()}`,
  );
}

export interface ServerRecord {
  id: number;
  name: string;
  dir: string;
  ds_id: number;
  game_id: string;
  game_mod_id: number;
  installed: number;
  server_ip?: string;
  internal_server_ip?: string;
  server_port: number;
  query_port: number;
  rcon_port: number;
}

export async function createServer(
  request: APIRequestContext,
  token: string,
  input: {
    name: string;
    ds_id: number;
    game_id: string;
    game_mod_id: number;
    server_ip: string;
    server_port: number;
  },
): Promise<number> {
  const body = await json<{ result?: { serverId?: number } }>(
    request,
    'post',
    '/api/servers',
    token,
    { ...input, install: false },
  );

  const serverId = body.result?.serverId;
  if (typeof serverId !== 'number') {
    throw new Error(`create server returned no serverId: ${JSON.stringify(body)}`);
  }

  return serverId;
}

export function getServer(
  request: APIRequestContext,
  token: string,
  id: number,
): Promise<ServerRecord> {
  return json<ServerRecord>(request, 'get', `/api/servers/${id}`, token);
}

// PUT /api/servers/{id} is a full replace, so the record is read back and sent
// again with only `installed` changed. Without it the server page hides its tabs
// and the plugin's FTP tab never renders.
export async function markServerInstalled(
  request: APIRequestContext,
  token: string,
  serverId: number,
): Promise<void> {
  const server = await getServer(request, token, serverId);

  await json(request, 'put', `/api/servers/${serverId}`, token, {
    name: server.name,
    ds_id: server.ds_id,
    game_id: server.game_id,
    game_mod_id: server.game_mod_id,
    server_ip: server.internal_server_ip ?? server.server_ip,
    server_port: server.server_port,
    query_port: server.query_port,
    rcon_port: server.rcon_port,
    dir: server.dir,
    enabled: true,
    installed: 1,
  });
}

export type DaemonTaskStatus =
  | 'waiting'
  | 'working'
  | 'error'
  | 'success'
  | 'canceled';

export interface DaemonTaskRecord {
  id: number;
  status: DaemonTaskStatus;
  task: string;
  server_id: number | null;
}

export function getDaemonTask(
  request: APIRequestContext,
  token: string,
  id: number,
): Promise<DaemonTaskRecord> {
  return json<DaemonTaskRecord>(request, 'get', `/api/gdaemon_tasks/${id}`, token);
}

// /api/gdaemon_tasks/{id} omits `output`; the captured stdout of the installer
// lives only on the admin-only sibling endpoint.
export async function getDaemonTaskOutput(
  request: APIRequestContext,
  token: string,
  id: number,
): Promise<string> {
  const response = await request.get(
    `${env.apiBaseUrl}/api/gdaemon_tasks/${id}/output`,
    { headers: authHeader(token) },
  );

  if (!response.ok()) {
    return `<failed to fetch task output: ${response.status()}>`;
  }

  const body = (await response.json()) as { output?: string | null };

  return body.output ?? '(empty)';
}

export interface LoadedPlugin {
  id: string;
  name: string;
  version: string;
  status: string;
  enabled: boolean;
  loaded: boolean;
  has_frontend_bundle: boolean;
  required_permissions: string[] | null;
  allowed_permissions: string[] | null;
  used_permissions: string[] | null;
  missing_permissions: string[] | null;
  http_routes?: { path: string; methods: string[] }[];
  server_abilities?: { name: string; title: string }[];
}

export async function loadedPlugins(
  request: APIRequestContext,
  token: string,
): Promise<{ data: LoadedPlugin[]; permissions_enforced: boolean }> {
  return json(request, 'get', '/api/admin/plugins/loaded', token);
}

export async function findLoadedPlugin(
  request: APIRequestContext,
  token: string,
  pluginId: string,
): Promise<LoadedPlugin | undefined> {
  const { data } = await loadedPlugins(request, token);

  return data.find((p) => p.id === pluginId);
}

function wasmMultipart(wasmPath: string) {
  return {
    file: {
      name: basename(wasmPath),
      mimeType: 'application/wasm',
      buffer: readFileSync(wasmPath),
    },
  };
}

export interface DryRunResult {
  id: string;
  name: string;
  version: string;
  api_version: string;
  required_permissions: string[] | null;
  has_frontend_bundle: boolean;
  installed: boolean;
  is_valid: boolean;
  errors: string[] | null;
}

export async function pluginDryRun(
  request: APIRequestContext,
  token: string,
  wasmPath: string,
): Promise<DryRunResult> {
  const response = await request.post(
    `${env.apiBaseUrl}/api/admin/plugins/upload/dry-run`,
    { headers: authHeader(token), multipart: wasmMultipart(wasmPath) },
  );

  if (!response.ok()) {
    throw new Error(
      `plugin dry-run failed: ${response.status()} ${await response.text()}`,
    );
  }

  return (await response.json()) as DryRunResult;
}

// Installing grants exactly the permissions the manifest declares. Uploading
// over an installed plugin grants nothing new — that contract is what
// 07-plugin-update-uninstall asserts.
export async function installPlugin(
  request: APIRequestContext,
  token: string,
  wasmPath: string,
): Promise<{ status: number; body: string }> {
  const response = await request.post(
    `${env.apiBaseUrl}/api/admin/plugins/upload/install`,
    { headers: authHeader(token), multipart: wasmMultipart(wasmPath) },
  );

  return { status: response.status(), body: await response.text() };
}

export async function updatePlugin(
  request: APIRequestContext,
  token: string,
  pluginId: string,
  wasmPath: string,
): Promise<{ status: number; body: string }> {
  const response = await request.post(
    `${env.apiBaseUrl}/api/admin/plugins/${pluginId}/upload`,
    { headers: authHeader(token), multipart: wasmMultipart(wasmPath) },
  );

  return { status: response.status(), body: await response.text() };
}

export async function uninstallPlugin(
  request: APIRequestContext,
  token: string,
  pluginId: string,
): Promise<number> {
  const response = await request.delete(
    `${env.apiBaseUrl}/api/admin/plugins/${pluginId}`,
    { headers: authHeader(token) },
  );

  return response.status();
}

export function setPluginPermissions(
  request: APIRequestContext,
  token: string,
  pluginId: string,
  permissions: string[],
): Promise<unknown> {
  return json(
    request,
    'put',
    `/api/admin/plugins/${pluginId}/permissions`,
    token,
    { allowed_permissions: permissions },
  );
}
