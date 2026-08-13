import { ref, watch, type Ref } from 'vue';
import { usersApi } from '@/api';
import type { FtpUser, CreateUserRequest, CreateUserResponse, UpdateUserRequest } from '@/types';

export function useFtpUsers(serverId: Ref<number | null>) {
  const users = ref<FtpUser[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchUsers() {
    if (!serverId.value) {
      users.value = [];
      return;
    }

    loading.value = true;
    error.value = null;

    try {
      const response = await usersApi.list(serverId.value);
      users.value = response.data;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to load users';
      users.value = [];
    } finally {
      loading.value = false;
    }
  }

  async function createUser(data: CreateUserRequest): Promise<CreateUserResponse | null> {
    if (!serverId.value) return null;

    loading.value = true;
    error.value = null;

    try {
      const response = await usersApi.create(serverId.value, data);
      await fetchUsers();
      return response.data;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to create user';
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function updateUser(username: string, data: UpdateUserRequest): Promise<FtpUser | null> {
    if (!serverId.value) return null;

    loading.value = true;
    error.value = null;

    try {
      const response = await usersApi.update(serverId.value, username, data);
      await fetchUsers();
      return response.data;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to update user';
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function deleteUser(username: string): Promise<boolean> {
    if (!serverId.value) return false;

    loading.value = true;
    error.value = null;

    try {
      await usersApi.delete(serverId.value, username);
      await fetchUsers();
      return true;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to delete user';
      throw e;
    } finally {
      loading.value = false;
    }
  }

  watch(serverId, fetchUsers, { immediate: true });

  return {
    users,
    loading,
    error,
    refresh: fetchUsers,
    createUser,
    updateUser,
    deleteUser,
  };
}
