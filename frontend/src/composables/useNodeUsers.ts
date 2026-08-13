import { ref, reactive, computed, watch, type Ref } from 'vue';
import { adminApi, usersApi } from '@/api';
import type { GroupedNode, GroupedServer, AdminUser, UpdateUserRequest, CreateUserRequest } from '@/types';

export interface NodeUserFilters {
  search?: string;
  server_id?: number;
  enabled?: boolean;
}

export function useNodeUsers(nodeId: Ref<number>) {
  const node = ref<GroupedNode | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  // The grouped response only carries the node when it has users, so the
  // breadcrumb name is resolved from /admin/nodes independently.
  const nodeInfoName = ref('');

  const filters = reactive<NodeUserFilters>({
    search: undefined,
    server_id: undefined,
    enabled: undefined,
  });

  // Debounce timer for search
  let searchDebounce: ReturnType<typeof setTimeout> | null = null;

  async function fetchUsers() {
    loading.value = true;
    error.value = null;

    try {
      const response = await adminApi.getUsers({ node_id: nodeId.value });
      node.value = response.data.grouped[nodeId.value] || null;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch users';
    } finally {
      loading.value = false;
    }

    if (!nodeInfoName.value) {
      fetchNodeName();
    }
  }

  async function fetchNodeName() {
    try {
      const response = await adminApi.getNodes();
      const info = response.data.nodes.find((n) => n.id === nodeId.value);
      nodeInfoName.value = info?.name ?? '';
    } catch {
      nodeInfoName.value = '';
    }
  }

  // Filter users based on local filters
  function filterUsers(users: AdminUser[]): AdminUser[] {
    return users.filter((user) => {
      // Search filter
      if (filters.search) {
        const searchLower = filters.search.toLowerCase();
        const matchesUsername = user.username.toLowerCase().includes(searchLower);
        const matchesDescription = user.description?.toLowerCase().includes(searchLower);
        if (!matchesUsername && !matchesDescription) {
          return false;
        }
      }

      // Enabled filter
      if (filters.enabled !== undefined && user.enabled !== filters.enabled) {
        return false;
      }

      return true;
    });
  }

  // Computed: filtered servers with filtered users
  const filteredServers = computed<Record<number, GroupedServer>>(() => {
    if (!node.value) return {};

    const result: Record<number, GroupedServer> = {};

    for (const [serverId, server] of Object.entries(node.value.servers)) {
      // Server filter
      if (filters.server_id !== undefined && server.server_id !== filters.server_id) {
        continue;
      }

      const filteredUsers = filterUsers(server.users);

      // Only include server if it has users after filtering
      if (filteredUsers.length > 0 || !filters.search) {
        result[Number(serverId)] = {
          ...server,
          users: filteredUsers,
        };
      }
    }

    return result;
  });

  // Computed: total filtered users count
  const totalUsers = computed(() => {
    return Object.values(filteredServers.value).reduce(
      (sum, server) => sum + server.users.length,
      0
    );
  });

  // Computed: list of servers for select
  const serverOptions = computed(() => {
    if (!node.value) return [];
    return Object.values(node.value.servers).map((s) => ({
      label: s.server_name,
      value: s.server_id,
      gameId: s.game_id,
    }));
  });

  // Computed: node name
  const nodeName = computed(
    () => node.value?.node_name || nodeInfoName.value || `#${nodeId.value}`
  );

  async function createUser(serverId: number, data: CreateUserRequest) {
    error.value = null;
    try {
      const response = await usersApi.create(serverId, data);
      await fetchUsers();
      return response.data;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to create user';
      throw e;
    }
  }

  async function updateUser(serverId: number, username: string, data: UpdateUserRequest) {
    error.value = null;
    try {
      await usersApi.update(serverId, username, data);
      await fetchUsers();
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to update user';
      throw e;
    }
  }

  async function deleteUser(serverId: number, username: string) {
    error.value = null;
    try {
      await usersApi.delete(serverId, username);
      await fetchUsers();
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to delete user';
      throw e;
    }
  }

  function setSearch(value: string) {
    if (searchDebounce) {
      clearTimeout(searchDebounce);
    }
    searchDebounce = setTimeout(() => {
      filters.search = value || undefined;
    }, 300);
  }

  function setServerFilter(serverId: number | undefined) {
    filters.server_id = serverId;
  }

  function setEnabledFilter(enabled: boolean | undefined) {
    filters.enabled = enabled;
  }

  function clearFilters() {
    filters.search = undefined;
    filters.server_id = undefined;
    filters.enabled = undefined;
  }

  // Watch nodeId and refetch
  watch(nodeId, () => {
    clearFilters();
    nodeInfoName.value = '';
    fetchUsers();
  });

  return {
    node,
    nodeName,
    loading,
    error,
    filters,
    filteredServers,
    totalUsers,
    serverOptions,
    fetchUsers,
    createUser,
    updateUser,
    deleteUser,
    setSearch,
    setServerFilter,
    setEnabledFilter,
    clearFilters,
    refresh: fetchUsers,
  };
}
