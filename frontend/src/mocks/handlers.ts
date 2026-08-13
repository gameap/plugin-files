import type {
  AccessRule,
  CreateUserRequest,
  FtpUser,
  NodeSetupConfig,
  UserFilters,
  VirtualPath,
} from '@/types';
import {
  getUsers,
  getUser,
  addUser,
  updateUser,
  deleteUser,
  getAccessRules,
  updateAccessRules,
  getVirtualPaths,
  updateVirtualPaths,
  getSshKeys,
  addSshKey,
  deleteSshKey,
  getNodeStatus,
  startNodeSetup,
  getNodeConfig,
  updateNodeConfig,
  generatePassword,
  getAllNodes,
  getAllUsersGrouped,
  getPickerServers,
} from './data';

/**
 * Register mock handlers for FTP Users API
 * Called in plugin's onInit() hook
 */
export function registerMockHandlers(): void {
  if (!window.gameapDebug) {
    return;
  }

  const { http, HttpResponse, delay } = window.gameapDebug.msw;

  window.gameapDebug.registerMockHandlers([
    // ==================== Users API ====================

    // List FTP users
    http.get('/api/plugins/files/servers/:serverId/ftp-users', async ({ params }) => {
      await delay(100);
      const serverId = parseInt(params.serverId, 10);
      const users = getUsers(serverId);
      return HttpResponse.json(users);
    }),

    // Get single user
    http.get('/api/plugins/files/servers/:serverId/ftp-users/:username', async ({ params }) => {
      await delay(100);
      const serverId = parseInt(params.serverId, 10);
      const user = getUser(serverId, params.username);

      if (!user) {
        return HttpResponse.json(
          { code: 'USER_NOT_FOUND', message: 'User not found' },
          { status: 404 }
        );
      }

      return HttpResponse.json(user);
    }),

    // Create user
    http.post('/api/plugins/files/servers/:serverId/ftp-users', async ({ params, request }) => {
      await delay(200);
      const serverId = parseInt(params.serverId, 10);
      const body = await request.json() as CreateUserRequest;

      // Check if user already exists
      if (getUser(serverId, body.username)) {
        return HttpResponse.json(
          { code: 'USER_EXISTS', message: 'User already exists' },
          { status: 409 }
        );
      }

      // Generate password if not provided
      const password = body.password || generatePassword();

      const newUser: FtpUser = {
        username: body.username,
        home_dir: body.home_dir || `/srv/gameap/servers/${serverId}`,
        quota_bytes: body.quota_bytes ?? 0,
        enabled: body.enabled ?? true,
        description: body.description || '',
        ssh_keys_count: 0,
        access_rules: [
          { path: '/**', permissions: ['read', 'write', 'delete', 'list'] },
        ],
        virtual_paths: [],
      };

      addUser(serverId, newUser);

      // Return response with password (only shown once)
      return HttpResponse.json({
        username: newUser.username,
        password: body.password ? undefined : password, // Only return if auto-generated
        home_dir: newUser.home_dir,
        quota_bytes: newUser.quota_bytes,
        enabled: newUser.enabled,
        description: newUser.description,
      });
    }),

    // Update user
    http.put('/api/plugins/files/servers/:serverId/ftp-users/:username', async ({ params, request }) => {
      await delay(150);
      const serverId = parseInt(params.serverId, 10);
      const body = await request.json() as Partial<FtpUser>;

      const updated = updateUser(serverId, params.username, body);

      if (!updated) {
        return HttpResponse.json(
          { code: 'USER_NOT_FOUND', message: 'User not found' },
          { status: 404 }
        );
      }

      return HttpResponse.json(updated);
    }),

    // Delete user
    http.delete('/api/plugins/files/servers/:serverId/ftp-users/:username', async ({ params }) => {
      await delay(150);
      const serverId = parseInt(params.serverId, 10);

      const deleted = deleteUser(serverId, params.username);

      if (!deleted) {
        return HttpResponse.json(
          { code: 'USER_NOT_FOUND', message: 'User not found' },
          { status: 404 }
        );
      }

      return HttpResponse.json({ success: true });
    }),

    // ==================== Nodes API ====================

    // Get node status
    http.get('/api/plugins/files/nodes/:nodeId/status', async ({ params }) => {
      await delay(100);
      const nodeId = parseInt(params.nodeId, 10);
      const status = getNodeStatus(nodeId);
      return HttpResponse.json(status);
    }),

    // Start node setup
    http.post('/api/plugins/files/nodes/:nodeId/setup', async ({ params }) => {
      await delay(200);
      const nodeId = parseInt(params.nodeId, 10);
      const status = startNodeSetup(nodeId);
      return HttpResponse.json(status);
    }),

    // Get node FTP/SFTP config
    http.get('/api/plugins/files/nodes/:nodeId/config', async ({ params }) => {
      await delay(100);
      const nodeId = parseInt(params.nodeId, 10);
      return HttpResponse.json(getNodeConfig(nodeId));
    }),

    // Update node FTP/SFTP config
    http.put('/api/plugins/files/nodes/:nodeId/config', async ({ params, request }) => {
      await delay(150);
      const nodeId = parseInt(params.nodeId, 10);
      const patch = (await request.json()) as NodeSetupConfig;
      return HttpResponse.json(updateNodeConfig(nodeId, patch));
    }),

    // ==================== Panel API overrides ====================

    // The FTP tab is gated by plugin:files:ftp-users-view, but the debug
    // harness's built-in abilities mock knows nothing about plugin abilities —
    // without this override the tab would never render under `npm run debug`.
    // Plugin handlers are registered via worker.use and take precedence.
    // Flip ftp-users-manage to false to exercise the read-only tab.
    http.get('/api/servers/:serverId/abilities', async () => {
      await delay(50);
      return HttpResponse.json({
        'game-server-common': true,
        'game-server-console-send': true,
        'game-server-console-view': true,
        'game-server-files': true,
        'game-server-metrics': true,
        'game-server-pause': false,
        'game-server-rcon-console': false,
        'game-server-rcon-players': false,
        'game-server-restart': true,
        'game-server-settings': true,
        'game-server-start': true,
        'game-server-stop': true,
        'game-server-tasks': true,
        'game-server-update': true,
        'plugin:files:ftp-users-view': true,
        'plugin:files:ftp-users-manage': true,
      });
    }),

    // ==================== Admin API ====================

    // List all nodes with plugin status
    http.get('/api/plugins/files/admin/nodes', async () => {
      await delay(150);
      const nodes = getAllNodes();
      return HttpResponse.json({ nodes });
    }),

    // List all users grouped by node -> server
    http.get('/api/plugins/files/admin/users', async ({ request }) => {
      await delay(200);

      // Parse query params
      const url = new URL(request.url);
      const filters: UserFilters = {};

      const search = url.searchParams.get('search');
      if (search) filters.search = search;

      const nodeId = url.searchParams.get('node_id');
      if (nodeId) filters.node_id = parseInt(nodeId, 10);

      const serverId = url.searchParams.get('server_id');
      if (serverId) filters.server_id = parseInt(serverId, 10);

      const enabled = url.searchParams.get('enabled');
      if (enabled !== null) filters.enabled = enabled === 'true';

      const result = getAllUsersGrouped(filters);
      return HttpResponse.json(result);
    }),

    // Search servers for the create-user picker
    http.get('/api/plugins/files/admin/pickers/servers', async ({ request }) => {
      await delay(150);
      const url = new URL(request.url);
      const q = url.searchParams.get('q') ?? '';
      const node = url.searchParams.get('node');
      const limit = url.searchParams.get('limit');
      return HttpResponse.json(
        getPickerServers(q, node ? parseInt(node, 10) : undefined, limit ? parseInt(limit, 10) : 20)
      );
    }),

    // ==================== Access Rules API ====================

    // Get access rules
    http.get('/api/plugins/files/servers/:serverId/ftp-users/:username/access-rules', async ({ params }) => {
      await delay(100);
      const serverId = parseInt(params.serverId, 10);
      const rules = getAccessRules(serverId, params.username);
      return HttpResponse.json({ rules });
    }),

    // Update access rules
    http.put('/api/plugins/files/servers/:serverId/ftp-users/:username/access-rules', async ({ params, request }) => {
      await delay(150);
      const serverId = parseInt(params.serverId, 10);
      const body = await request.json() as { rules: AccessRule[] };

      const rules = updateAccessRules(serverId, params.username, body.rules);
      return HttpResponse.json({ rules });
    }),

    // ==================== Virtual Paths API ====================

    // Get virtual paths
    http.get('/api/plugins/files/servers/:serverId/ftp-users/:username/virtual-paths', async ({ params }) => {
      await delay(100);
      const serverId = parseInt(params.serverId, 10);
      const paths = getVirtualPaths(serverId, params.username);
      return HttpResponse.json({ paths });
    }),

    // Update virtual paths
    http.put('/api/plugins/files/servers/:serverId/ftp-users/:username/virtual-paths', async ({ params, request }) => {
      await delay(150);
      const serverId = parseInt(params.serverId, 10);
      const body = await request.json() as { paths: VirtualPath[] };

      const paths = updateVirtualPaths(serverId, params.username, body.paths);
      return HttpResponse.json({ paths });
    }),

    // ==================== SSH Keys API ====================

    // List SSH keys
    http.get('/api/plugins/files/servers/:serverId/ftp-users/:username/ssh-keys', async ({ params }) => {
      await delay(100);
      const serverId = parseInt(params.serverId, 10);
      const keys = getSshKeys(serverId, params.username);
      return HttpResponse.json({ keys });
    }),

    // Add SSH key
    http.post('/api/plugins/files/servers/:serverId/ftp-users/:username/ssh-keys', async ({ params, request }) => {
      await delay(150);
      const serverId = parseInt(params.serverId, 10);
      const body = await request.json() as { key: string };

      const keys = addSshKey(serverId, params.username, body.key);
      return HttpResponse.json({ keys });
    }),

    // Delete SSH key
    http.delete('/api/plugins/files/servers/:serverId/ftp-users/:username/ssh-keys/:index', async ({ params }) => {
      await delay(150);
      const serverId = parseInt(params.serverId, 10);
      const index = parseInt(params.index, 10);

      const keys = deleteSshKey(serverId, params.username, index);
      return HttpResponse.json({ keys });
    }),
  ]);

  console.log('[FTP Users Plugin] Mock handlers registered');
}
