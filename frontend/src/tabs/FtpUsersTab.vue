<template>
  <div class="mt-2 space-y-4">
    <!-- Primary action first, node status below it -->
    <div v-if="showContent && canManage" class="flex flex-wrap gap-2">
      <GButton color="green" size="middle" @click="showCreateModal = true">
        <GIcon name="add-square" class="mr-0.5" />
        <span>{{ trans('create_user') }}</span>
      </GButton>
    </div>

    <!-- Node Status (admin only) -->
    <NodeStatusCard
      v-if="isAdmin"
      :status="nodeStatus"
      :loading="nodeLoading"
      :config="nodeConfig"
      :config-loading="configLoading"
      @setup="startNodeSetup"
      @load-config="loadNodeConfig"
      @update-config="updateNodeConfig"
    />

    <!-- Warning if node not ready (only for admins who can see actual status) -->
    <div
      v-if="!isNodeReady && isAdmin"
      class="p-3 rounded border border-warning bg-warning-soft text-warning-soft-text text-sm"
    >
      <GIcon name="warning" class="mr-2" />
      {{ trans('node_not_ready') }}
    </div>

    <!-- Main content: always show for non-admins, show for admins when node ready -->
    <template v-if="showContent">
      <!-- User List -->
      <UserList
        :users="users"
        :loading="usersLoading"
        :error="usersError"
        :can-manage="canManage"
        @create="showCreateModal = true"
        @edit="openEditModal"
        @delete="confirmDelete"
        @refresh="refreshUsers"
      />

      <!-- Create User Modal -->
      <GModal
        v-model:show="showCreateModal"
        :title="trans('create_user')"
        style="width: 900px; max-width: 92vw"
      >
        <UserCreateForm
          :loading="creating"
          @submit="createUser"
          @cancel="showCreateModal = false"
        />
      </GModal>

      <!-- Edit User Modal -->
      <GModal
        v-model:show="showEditModal"
        :title="trans('edit_user').replace('{username}', selectedUser?.username ?? '')"
        style="width: 900px; max-width: 92vw"
      >
        <template v-if="selectedUser">
          <div class="space-y-4">
            <!-- Basic Info -->
            <UserEditForm
              :user="selectedUser"
              :loading="updating"
              @submit="updateUser"
              @cancel="showEditModal = false"
            />

            <hr class="border-stone-200 dark:border-stone-700" />

            <!-- Access Rules -->
            <AccessRulesEditor
              :rules="selectedUser.access_rules"
              :saving="savingRules"
              @save="saveAccessRules"
            />

            <hr class="border-stone-200 dark:border-stone-700" />

            <!-- Virtual Paths -->
            <VirtualPathsEditor
              :paths="selectedUser.virtual_paths"
              :saving="savingPaths"
              @save="saveVirtualPaths"
            />

            <hr class="border-stone-200 dark:border-stone-700" />

            <!-- SSH Keys -->
            <SshKeysList
              :keys="sshKeys"
              :loading="keysLoading"
              @add="addSshKey"
              @delete="deleteSshKey"
            />
          </div>
        </template>
      </GModal>

      <!-- Password Modal -->
      <PasswordModal
        v-model="showPasswordModal"
        :username="createdUsername"
        :password="createdPassword"
      />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { providePluginTrans } from '@gameap/plugin-sdk';
import type { FtpUser, UpdateUserRequest, AccessRule, VirtualPath, NodeSetupConfig } from '@/types';
import type { CreateUserFormData } from '@/components/users/UserCreateForm.vue';
import { useNodeStatus, useFtpUsers, useCanManageFtp, useSafeIsAdmin } from '@/composables';
import { accessRulesApi, virtualPathsApi, sshKeysApi } from '@/api';

import { NodeStatusCard } from '@/components/node';
import { UserList, UserCreateForm, UserEditForm, PasswordModal } from '@/components/users';
import { AccessRulesEditor } from '@/components/access-rules';
import { VirtualPathsEditor } from '@/components/virtual-paths';
import { SshKeysList } from '@/components/ssh-keys';

interface ServerTabProps {
  serverId: number;
  server: {
    id: number;
    name: string;
    ds_id: number;
    [key: string]: unknown;
  };
  pluginId: string;
}

const props = defineProps<ServerTabProps>();

const { trans } = providePluginTrans(props.pluginId);
const isAdmin = useSafeIsAdmin();
const canManage = useCanManageFtp();

// Node status - use ds_id (dedicated server / node ID)
const nodeId = computed(() => props.server?.ds_id ?? null);
const {
  status: nodeStatus,
  loading: nodeLoading,
  config: nodeConfig,
  configLoading,
  startSetup: startNodeSetup,
  fetchConfig,
  updateConfig,
} = useNodeStatus(nodeId);

const isNodeReady = computed(() =>
  nodeStatus.value?.status === 'installed'
);

const showContent = computed(() => !isAdmin.value || isNodeReady.value);

// Config handlers
async function loadNodeConfig() {
  await fetchConfig();
}

async function updateNodeConfig(config: NodeSetupConfig) {
  await updateConfig(config);
}

// Users
const serverId = computed(() => props.serverId);
const {
  users,
  loading: usersLoading,
  error: usersError,
  refresh: refreshUsers,
  createUser: apiCreateUser,
  updateUser: apiUpdateUser,
  deleteUser: apiDeleteUser,
} = useFtpUsers(serverId);

// Create user
const showCreateModal = ref(false);
const creating = ref(false);
const showPasswordModal = ref(false);
const createdUsername = ref('');
const createdPassword = ref('');

async function createUser(data: CreateUserFormData) {
  creating.value = true;
  try {
    // 1. Create the user
    const result = await apiCreateUser({
      username: data.username,
      password: data.password,
      home_dir: data.home_dir,
      quota_bytes: data.quota_bytes,
      enabled: data.enabled,
      description: data.description,
    });

    if (!result) return;

    // 2. Set access rules (if any)
    if (data.accessRules && data.accessRules.length > 0) {
      await accessRulesApi.update(props.serverId, result.username, data.accessRules);
    }

    // 3. Set virtual paths (if any)
    if (data.virtualPaths && data.virtualPaths.length > 0) {
      await virtualPathsApi.update(props.serverId, result.username, data.virtualPaths);
    }

    // 4. Add SSH keys (if any)
    for (const key of data.sshKeys || []) {
      await sshKeysApi.add(props.serverId, result.username, key);
    }

    // Refresh user list to get updated data
    await refreshUsers();

    showCreateModal.value = false;

    if (result.password) {
      createdUsername.value = result.username;
      createdPassword.value = result.password;
      showPasswordModal.value = true;
    }
  } catch {
    // Error handled in composable
  } finally {
    creating.value = false;
  }
}

// Edit user
const showEditModal = ref(false);
const selectedUser = ref<FtpUser | null>(null);
const updating = ref(false);
const savingRules = ref(false);
const savingPaths = ref(false);
const sshKeys = ref<string[]>([]);
const keysLoading = ref(false);

function openEditModal(user: FtpUser) {
  selectedUser.value = user;
  sshKeys.value = [];
  showEditModal.value = true;
  loadSshKeys();
}

async function updateUser(data: UpdateUserRequest) {
  if (!selectedUser.value) return;

  updating.value = true;
  try {
    const updated = await apiUpdateUser(selectedUser.value.username, data);
    if (updated) {
      selectedUser.value = updated;
    }
  } catch {
    // Error handled in composable
  } finally {
    updating.value = false;
  }
}

async function saveAccessRules(rules: AccessRule[]) {
  if (!selectedUser.value) return;

  savingRules.value = true;
  try {
    const response = await accessRulesApi.update(
      props.serverId,
      selectedUser.value.username,
      rules
    );
    selectedUser.value.access_rules = response.data.rules;
    await refreshUsers();
  } catch {
    // Error handling
  } finally {
    savingRules.value = false;
  }
}

async function saveVirtualPaths(paths: VirtualPath[]) {
  if (!selectedUser.value) return;

  savingPaths.value = true;
  try {
    const response = await virtualPathsApi.update(
      props.serverId,
      selectedUser.value.username,
      paths
    );
    selectedUser.value.virtual_paths = response.data.paths;
    await refreshUsers();
  } catch {
    // Error handling
  } finally {
    savingPaths.value = false;
  }
}

async function loadSshKeys() {
  if (!selectedUser.value) return;

  keysLoading.value = true;
  try {
    const response = await sshKeysApi.list(props.serverId, selectedUser.value.username);
    sshKeys.value = response.data.keys || [];
  } catch {
    sshKeys.value = [];
  } finally {
    keysLoading.value = false;
  }
}

async function addSshKey(key: string) {
  if (!selectedUser.value) return;

  try {
    const response = await sshKeysApi.add(props.serverId, selectedUser.value.username, key);
    sshKeys.value = response.data.keys || [];
  } catch {
    // Error handling
  }
}

async function deleteSshKey(index: number) {
  if (!selectedUser.value) return;

  try {
    const response = await sshKeysApi.delete(props.serverId, selectedUser.value.username, index);
    sshKeys.value = response.data.keys || [];
  } catch {
    // Error handling
  }
}

// Delete user
function confirmDelete(user: FtpUser) {
  const username = user.username;
  window.$dialog?.warning({
    title: trans('delete_user'),
    content: trans('delete_user_confirm').replace('{username}', username),
    positiveText: trans('delete'),
    negativeText: trans('cancel'),
    closable: false,
    onPositiveClick: async () => {
      await apiDeleteUser(username);
    },
  });
}

// Close edit modal when user is deleted
watch(users, () => {
  if (selectedUser.value && !users.value.find(u => u.username === selectedUser.value?.username)) {
    showEditModal.value = false;
    selectedUser.value = null;
  }
});
</script>
