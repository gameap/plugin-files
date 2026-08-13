import type {
  FtpUser,
  NodeSetupStatus,
  NodeSetupConfig,
  NodeConfigResponse,
  AccessRule,
  VirtualPath,
  AdminNode,
  AdminUser,
  GroupedNode,
  PickerServer,
  UserFilters,
} from '@/types';

/**
 * Mock nodes list (simulates GameAP nodes)
 */
export const mockNodes: Array<{ id: number; name: string; ip: string }> = [
  { id: 1, name: 'Main Server', ip: '192.168.1.100' },
  { id: 2, name: 'EU Node', ip: '10.0.0.50' },
  { id: 3, name: 'US West', ip: '172.16.0.10' },
  { id: 4, name: 'Testing Node', ip: '192.168.100.1' },
];

/**
 * Mock servers list (simulates GameAP servers)
 * Maps node_id -> servers
 */
export const mockServers: Record<number, Array<{ id: number; name: string; game_id: string }>> = {
  // Node 1 - Main Server: 4 game servers (Palworld has no FTP users, so it
  // only ever appears through the picker, never in the grouped users tree)
  1: [
    { id: 1, name: 'Minecraft Survival', game_id: 'minecraft' },
    { id: 2, name: 'CS2 Competitive', game_id: 'cs2' },
    { id: 3, name: 'Rust Official', game_id: 'rust' },
    { id: 7, name: 'Palworld Fresh', game_id: 'palworld' },
  ],
  // Node 2 - EU Node: 2 game servers
  2: [
    { id: 4, name: 'ARK: Survival', game_id: 'ark' },
    { id: 5, name: 'Valheim Nordic', game_id: 'valheim' },
  ],
  // Node 3 - US West: 1 game server
  3: [
    { id: 6, name: 'Team Fortress 2', game_id: 'tf2' },
  ],
  // Node 4 - Testing: no servers
  4: [],
};

/**
 * Mock FTP users per server
 */
export const mockUsers: Record<number, FtpUser[]> = {
  // Server 1 (Minecraft) - 2 users
  1: [
    {
      username: 'mc_admin',
      home_dir: '/srv/gameap/servers/1',
      quota_bytes: 10737418240, // 10GB
      enabled: true,
      description: 'Main server administrator',
      ssh_keys_count: 2,
      access_rules: [
        { path: '/**', permissions: ['read', 'write', 'delete', 'list'] },
      ],
      virtual_paths: [
        {
          virtual: '/shared',
          target: '/srv/gameap/shared',
          permissions: ['read', 'list'],
          read_only: true,
        },
      ],
    },
    {
      username: 'mc_builder',
      home_dir: '/srv/gameap/servers/1/plugins',
      quota_bytes: 1073741824, // 1GB
      enabled: true,
      description: 'Plugin developer access',
      ssh_keys_count: 0,
      access_rules: [
        { path: '/plugins/**', permissions: ['read', 'write', 'list'] },
        { path: '/config/**', permissions: ['read', 'list'] },
      ],
      virtual_paths: [],
    },
  ],

  // Server 2 (CS2) - 1 disabled user
  2: [
    {
      username: 'cs2_admin',
      home_dir: '/srv/gameap/servers/2',
      quota_bytes: 0, // Unlimited
      enabled: false,
      description: 'Disabled account',
      ssh_keys_count: 1,
      access_rules: [
        { path: '/**', permissions: ['read', 'write', 'delete', 'list'] },
      ],
      virtual_paths: [],
    },
  ],

  // Server 3 (Rust) - 1 user
  3: [
    {
      username: 'rust_admin',
      home_dir: '/srv/gameap/servers/3',
      quota_bytes: 5368709120, // 5GB
      enabled: true,
      description: 'Rust server admin',
      ssh_keys_count: 0,
      access_rules: [
        { path: '/**', permissions: ['read', 'write', 'delete', 'list'] },
      ],
      virtual_paths: [],
    },
  ],

  // Server 4 (ARK) - 2 users on Node 2
  4: [
    {
      username: 'ark_owner',
      home_dir: '/srv/gameap/servers/4',
      quota_bytes: 0, // Unlimited
      enabled: true,
      description: 'ARK server owner',
      ssh_keys_count: 1,
      access_rules: [
        { path: '/**', permissions: ['read', 'write', 'delete', 'list'] },
      ],
      virtual_paths: [],
    },
    {
      username: 'ark_mod',
      home_dir: '/srv/gameap/servers/4/mods',
      quota_bytes: 2147483648, // 2GB
      enabled: true,
      description: 'Mod manager',
      ssh_keys_count: 0,
      access_rules: [
        { path: '/mods/**', permissions: ['read', 'write', 'list'] },
      ],
      virtual_paths: [],
    },
  ],

  // Server 5 (Valheim) - 1 disabled user on Node 2
  5: [
    {
      username: 'valheim_backup',
      home_dir: '/srv/gameap/servers/5',
      quota_bytes: 1073741824, // 1GB
      enabled: false,
      description: 'Backup account (disabled)',
      ssh_keys_count: 0,
      access_rules: [
        { path: '/**', permissions: ['read', 'list'] },
      ],
      virtual_paths: [],
    },
  ],

  // Server 6 (TF2) - 1 user on Node 3
  6: [
    {
      username: 'tf2_admin',
      home_dir: '/srv/gameap/servers/6',
      quota_bytes: 3221225472, // 3GB
      enabled: true,
      description: 'TF2 server administrator',
      ssh_keys_count: 0,
      access_rules: [
        { path: '/**', permissions: ['read', 'write', 'delete', 'list'] },
      ],
      virtual_paths: [],
    },
  ],
};

/**
 * Mock SSH keys per user (serverId -> username -> keys)
 */
export const mockSshKeys: Record<number, Record<string, string[]>> = {
  1: {
    mc_admin: [
      'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx admin@workstation',
      'ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABgQCxxxxxxxxxxxxxxxxx admin@laptop',
    ],
    mc_builder: [],
  },
  2: {
    cs2_admin: [
      'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy cs2@server',
    ],
  },
  3: { rust_admin: [] },
  4: {
    ark_owner: [
      'ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz ark@home',
    ],
    ark_mod: [],
  },
  5: { valheim_backup: [] },
  6: { tf2_admin: [] },
};

/**
 * Mock node statuses
 */
export const mockNodeStatus: Record<number, NodeSetupStatus> = {
  // Node 1 (Main Server) - installed
  1: {
    status: 'installed',
    version: '1.2.0',
    last_check: Date.now(),
  },

  // Node 2 (EU Node) - installed (older version)
  2: {
    status: 'installed',
    version: '1.1.5',
    last_check: Date.now(),
  },

  // Node 3 (US West) - not installed
  3: {
    status: 'not_installed',
    last_check: Date.now(),
  },

  // Node 4 (Testing Node) - failed installation
  4: {
    status: 'failed',
    error_message: 'Failed to install gameap-files: connection timeout',
    last_check: Date.now(),
  },
};

// Mutable state for CRUD operations
let usersState = JSON.parse(JSON.stringify(mockUsers)) as Record<number, FtpUser[]>;
let sshKeysState = JSON.parse(JSON.stringify(mockSshKeys)) as Record<number, Record<string, string[]>>;
let nodeStatusState = JSON.parse(JSON.stringify(mockNodeStatus)) as Record<number, NodeSetupStatus>;

/**
 * Get users for a server
 */
export function getUsers(serverId: number): FtpUser[] {
  return usersState[serverId] || [];
}

/**
 * Get a specific user
 */
export function getUser(serverId: number, username: string): FtpUser | undefined {
  return getUsers(serverId).find(u => u.username === username);
}

/**
 * Add a new user
 */
export function addUser(serverId: number, user: FtpUser): void {
  if (!usersState[serverId]) {
    usersState[serverId] = [];
  }
  usersState[serverId].push(user);
}

/**
 * Update a user
 */
export function updateUser(serverId: number, username: string, updates: Partial<FtpUser>): FtpUser | undefined {
  const users = getUsers(serverId);
  const index = users.findIndex(u => u.username === username);
  if (index === -1) return undefined;

  users[index] = { ...users[index], ...updates };
  return users[index];
}

/**
 * Delete a user
 */
export function deleteUser(serverId: number, username: string): boolean {
  const users = getUsers(serverId);
  const index = users.findIndex(u => u.username === username);
  if (index === -1) return false;

  users.splice(index, 1);
  // Also remove SSH keys
  if (sshKeysState[serverId]) {
    delete sshKeysState[serverId][username];
  }
  return true;
}

/**
 * Get access rules for a user
 */
export function getAccessRules(serverId: number, username: string): AccessRule[] {
  return getUser(serverId, username)?.access_rules || [];
}

/**
 * Update access rules for a user
 */
export function updateAccessRules(serverId: number, username: string, rules: AccessRule[]): AccessRule[] {
  const user = getUser(serverId, username);
  if (user) {
    user.access_rules = rules;
  }
  return rules;
}

/**
 * Get virtual paths for a user
 */
export function getVirtualPaths(serverId: number, username: string): VirtualPath[] {
  return getUser(serverId, username)?.virtual_paths || [];
}

/**
 * Update virtual paths for a user
 */
export function updateVirtualPaths(serverId: number, username: string, paths: VirtualPath[]): VirtualPath[] {
  const user = getUser(serverId, username);
  if (user) {
    user.virtual_paths = paths;
  }
  return paths;
}

/**
 * Get SSH keys for a user
 */
export function getSshKeys(serverId: number, username: string): string[] {
  return sshKeysState[serverId]?.[username] || [];
}

/**
 * Add SSH key for a user
 */
export function addSshKey(serverId: number, username: string, key: string): string[] {
  if (!sshKeysState[serverId]) {
    sshKeysState[serverId] = {};
  }
  if (!sshKeysState[serverId][username]) {
    sshKeysState[serverId][username] = [];
  }
  sshKeysState[serverId][username].push(key);

  // Update ssh_keys_count on user
  const user = getUser(serverId, username);
  if (user) {
    user.ssh_keys_count = sshKeysState[serverId][username].length;
  }

  return sshKeysState[serverId][username];
}

/**
 * Delete SSH key by index
 */
export function deleteSshKey(serverId: number, username: string, index: number): string[] {
  const keys = getSshKeys(serverId, username);
  if (index >= 0 && index < keys.length) {
    keys.splice(index, 1);

    // Update ssh_keys_count on user
    const user = getUser(serverId, username);
    if (user) {
      user.ssh_keys_count = keys.length;
    }
  }
  return keys;
}

/**
 * Get node status
 */
export function getNodeStatus(nodeId: number): NodeSetupStatus {
  return nodeStatusState[nodeId] || { status: 'not_installed', last_check: Date.now() };
}

/**
 * Start node setup (simulates installation)
 */
export function startNodeSetup(nodeId: number): NodeSetupStatus {
  nodeStatusState[nodeId] = {
    status: 'installing',
    task_id: Math.floor(Math.random() * 1000),
    last_check: Date.now(),
  };

  // Simulate installation completing after 10 seconds
  setTimeout(() => {
    nodeStatusState[nodeId] = {
      status: 'installed',
      version: '1.2.0',
      last_check: Date.now(),
    };
  }, 10000);

  return nodeStatusState[nodeId];
}

/**
 * Mock node FTP/SFTP config state (GET/PUT /nodes/{id}/config)
 */
function defaultNodeConfig(): NodeConfigResponse {
  return {
    ftp: {
      address: '',
      port: 21,
      passive_port_min: 30000,
      passive_port_max: 30100,
      public_host: '',
      tls_enabled: false,
      tls_implicit_port: 990,
    },
    sftp: { port: 2222 },
  };
}

let nodeConfigState: Record<number, NodeConfigResponse> = {};

export function getNodeConfig(nodeId: number): NodeConfigResponse {
  if (!nodeConfigState[nodeId]) {
    nodeConfigState[nodeId] = defaultNodeConfig();
  }
  return nodeConfigState[nodeId];
}

export function updateNodeConfig(nodeId: number, patch: NodeSetupConfig): NodeConfigResponse {
  const config = getNodeConfig(nodeId);
  if (patch.ftp) Object.assign(config.ftp, patch.ftp);
  if (patch.sftp) Object.assign(config.sftp, patch.sftp);
  return config;
}

/**
 * Generate a random password
 */
export function generatePassword(): string {
  const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*';
  let password = '';
  for (let i = 0; i < 16; i++) {
    password += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return password;
}

/**
 * Reset all mock data to initial state
 */
export function resetMockData(): void {
  usersState = JSON.parse(JSON.stringify(mockUsers));
  sshKeysState = JSON.parse(JSON.stringify(mockSshKeys));
  nodeStatusState = JSON.parse(JSON.stringify(mockNodeStatus));
  nodeConfigState = {};
}

// ==================== Admin API Functions ====================

/**
 * Get all nodes with their plugin status
 */
export function getAllNodes(): AdminNode[] {
  return mockNodes.map((node) => ({
    id: node.id,
    name: node.name,
    ip: node.ip,
    plugin_status: getNodeStatus(node.id),
  }));
}

/**
 * Get server -> node mapping
 */
function getServerNodeId(serverId: number): number | undefined {
  for (const [nodeId, servers] of Object.entries(mockServers)) {
    if (servers.some((s) => s.id === serverId)) {
      return parseInt(nodeId, 10);
    }
  }
  return undefined;
}

/**
 * Get server info by ID
 */
function getServerInfo(serverId: number): { name: string; game_id: string } {
  for (const servers of Object.values(mockServers)) {
    const server = servers.find((s) => s.id === serverId);
    if (server) return { name: server.name, game_id: server.game_id };
  }
  return { name: `Server ${serverId}`, game_id: '' };
}

/**
 * Get all users grouped by node -> server with optional filters
 */
export function getAllUsersGrouped(filters?: UserFilters): { grouped: Record<number, GroupedNode>; total: number } {
  const grouped: Record<number, GroupedNode> = {};
  let total = 0;

  // Iterate through all servers and their users
  for (const [serverIdStr, users] of Object.entries(usersState)) {
    const serverId = parseInt(serverIdStr, 10);
    const nodeId = getServerNodeId(serverId);

    if (!nodeId) continue;

    // Apply node filter
    if (filters?.node_id && nodeId !== filters.node_id) continue;

    // Apply server filter
    if (filters?.server_id && serverId !== filters.server_id) continue;

    // Filter users
    const filteredUsers = users.filter((user) => {
      // Search filter
      if (filters?.search) {
        const search = filters.search.toLowerCase();
        if (!user.username.toLowerCase().includes(search)) {
          return false;
        }
      }

      // Enabled filter
      if (filters?.enabled !== undefined && user.enabled !== filters.enabled) {
        return false;
      }

      return true;
    });

    if (filteredUsers.length === 0) continue;

    // Initialize node group if needed
    if (!grouped[nodeId]) {
      const node = mockNodes.find((n) => n.id === nodeId);
      grouped[nodeId] = {
        node_id: nodeId,
        node_name: node?.name || `Node ${nodeId}`,
        servers: {},
      };
    }

    // Add server with users
    const adminUsers: AdminUser[] = filteredUsers.map((user) => ({
      username: user.username,
      enabled: user.enabled,
      home_dir: user.home_dir,
      quota_bytes: user.quota_bytes,
      description: user.description,
    }));

    const serverInfo = getServerInfo(serverId);
    grouped[nodeId].servers[serverId] = {
      server_id: serverId,
      server_name: serverInfo.name,
      game_id: serverInfo.game_id,
      users: adminUsers,
    };

    total += adminUsers.length;
  }

  return { grouped, total };
}

/**
 * Search servers for the create-user picker (GET /admin/pickers/servers)
 */
export function getPickerServers(
  q: string,
  nodeId?: number,
  limit = 20
): { items: PickerServer[]; total: number } {
  const query = q.toLowerCase();
  const all: PickerServer[] = Object.entries(mockServers).flatMap(([nid, servers]) =>
    servers.map((s) => ({
      id: s.id,
      name: s.name,
      node_id: parseInt(nid, 10),
      enabled: true,
      game_id: s.game_id,
    }))
  );
  const matched = all
    .filter((s) => (!nodeId || s.node_id === nodeId) && (!query || s.name.toLowerCase().includes(query)))
    .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id - b.id);
  return { items: matched.slice(0, Math.min(limit, 100)), total: matched.length };
}
