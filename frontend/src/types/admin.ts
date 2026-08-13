import type { NodeSetupStatus } from './node';

export interface AdminNode {
  id: number;
  name: string;
  ip: string;
  plugin_status: NodeSetupStatus;
}

export interface AdminNodesResponse {
  nodes: AdminNode[];
}

export interface AdminUser {
  username: string;
  enabled: boolean;
  home_dir: string;
  quota_bytes: number;
  description: string;
}

export interface GroupedServer {
  server_id: number;
  server_name: string;
  game_id: string;
  users: AdminUser[];
}

export interface GroupedNode {
  node_id: number;
  node_name: string;
  servers: Record<number, GroupedServer>;
}

export interface AdminUsersResponse {
  grouped: Record<number, GroupedNode>;
  total: number;
}

export interface UserFilters {
  search?: string;
  node_id?: number;
  server_id?: number;
  enabled?: boolean;
}
