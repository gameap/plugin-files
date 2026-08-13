<template>
  <GBreadcrumbs :items="breadcrumbs" class="mb-4" />

  <div class="space-y-3">
    <!-- Toolbar: primary action left, filters pushed right -->
    <div class="flex flex-wrap items-center gap-2">
      <GButton color="green" size="middle" @click="openCreateModal">
        <GIcon name="add-square" class="mr-0.5" />
        <span>{{ trans('create_ftp_user') }}</span>
      </GButton>
      <NodeUsersFilters
        class="ml-auto"
        :filters="filters"
        :servers="serverOptions"
        @update:search="setSearch"
        @update:server-id="setServerFilter"
        @update:enabled="setEnabledFilter"
      />
    </div>

    <!-- Loading state -->
    <Loading v-if="loading && Object.keys(filteredServers).length === 0" />

    <!-- Tree -->
    <NodeUsersTree
      v-else
      :servers="filteredServers"
      @edit-user="openUserEditModal"
      @delete-user="confirmDeleteUser"
    />
  </div>

  <!-- Create User Modal -->
  <UserCreateModal
    :open="createModalOpen"
    :node-id="nodeId"
    @close="createModalOpen = false"
    @created="onUserCreated"
  />

  <!-- Edit User Modal -->
  <UserEditModal
    :open="userEditModalOpen"
    :server-id="editingUser?.serverId ?? null"
    :username="editingUser?.username ?? null"
    @close="closeUserEditModal"
    @saved="onUserSaved"
  />
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useRoute } from 'vue-router';
import { usePluginTrans } from '@gameap/plugin-sdk';
import { useNodeUsers } from '@/composables';
import { NodeUsersFilters, NodeUsersTree, UserEditModal, UserCreateModal } from '@/components/admin';

const route = useRoute();
const { trans } = usePluginTrans();

const nodeId = computed(() => Number(route.params.nodeId));

const {
  nodeName,
  loading,
  filters,
  filteredServers,
  serverOptions,
  fetchUsers,
  deleteUser,
  setSearch,
  setServerFilter,
  setEnabledFilter,
} = useNodeUsers(nodeId);

const breadcrumbs = computed(() => [
  { route: '/', text: 'GameAP', icon: 'gicon gicon-gameap' },
  { route: '/plugins/files', text: trans('ftp_users_admin') },
  { text: nodeName.value || '' },
]);

// Create modal state
const createModalOpen = ref(false);

// User edit modal state
const userEditModalOpen = ref(false);
const editingUser = ref<{ serverId: number; username: string } | null>(null);

onMounted(() => {
  fetchUsers();
});

function openCreateModal() {
  createModalOpen.value = true;
}

function onUserCreated() {
  fetchUsers();
}

function openUserEditModal(data: { serverId: number; username: string }) {
  editingUser.value = data;
  userEditModalOpen.value = true;
}

function closeUserEditModal() {
  userEditModalOpen.value = false;
  editingUser.value = null;
}

function onUserSaved() {
  fetchUsers();
}

function confirmDeleteUser(data: { serverId: number; username: string }) {
  window.$dialog?.warning({
    title: trans('delete_user'),
    content: `${trans('confirm_delete_user')} ${data.username}?`,
    positiveText: trans('delete'),
    negativeText: trans('cancel'),
    closable: false,
    onPositiveClick: async () => {
      try {
        await deleteUser(data.serverId, data.username);
      } catch (e) {
        console.error('Failed to delete user:', e);
      }
    },
  });
}
</script>
