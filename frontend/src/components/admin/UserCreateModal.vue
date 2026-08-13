<template>
  <GModal
    :show="open"
    :title="trans('create_ftp_user')"
    style="width: 900px; max-width: 92vw"
    @update:show="(v: boolean) => !v && handleClose()"
  >
    <div class="space-y-4">
      <!-- Server selector -->
      <FormField :label="trans('select_server')">
        <NSelect
          v-model:value="selectedServerId"
          :placeholder="trans('select_server_placeholder')"
          :options="serverOptions"
          :render-label="renderServerLabel"
          :disabled="saving"
          style="width: 100%"
        />
      </FormField>

      <!-- User form - shown only when server is selected -->
      <template v-if="selectedServerId">
        <hr class="border-stone-200 dark:border-stone-700" />
        <UserCreateForm
          :loading="saving"
          @submit="handleSubmit"
          @cancel="handleClose"
        />
      </template>

      <!-- Cancel button when no server selected -->
      <div v-else class="flex justify-end pt-4">
        <GButton color="white" @click="handleClose">
          {{ trans('cancel') }}
        </GButton>
      </div>
    </div>
  </GModal>

  <!-- Password modal -->
  <PasswordModal
    v-model="showPasswordModal"
    :username="createdUsername"
    :password="createdPassword"
  />
</template>

<script setup lang="ts">
import { ref, computed, watch, h, resolveComponent } from 'vue';
import { NSelect, type SelectOption, type SelectRenderLabel } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import { usersApi, accessRulesApi, virtualPathsApi, sshKeysApi } from '@/api';
import FormField from '@/components/form/FormField.vue';
import UserCreateForm from '@/components/users/UserCreateForm.vue';
import { PasswordModal } from '@/components/users';
import type { CreateUserFormData } from '@/components/users/UserCreateForm.vue';

interface ServerOption {
  label: string;
  value: number;
  gameId: string;
}

const props = defineProps<{
  open: boolean;
  servers: ServerOption[];
}>();

const GGameIcon = resolveComponent('GGameIcon');

const renderServerLabel: SelectRenderLabel = (option) => {
  const server = props.servers.find((s) => s.value === option.value);
  return h('div', { class: 'flex items-center gap-2' }, [
    h(GGameIcon, { game: server?.gameId, size: 'small' }),
    h('span', option.label as string),
  ]);
};

const emit = defineEmits<{
  close: [];
  created: [];
}>();

const { trans } = usePluginTrans();

const selectedServerId = ref<number | null>(null);
const saving = ref(false);

// Password modal state
const showPasswordModal = ref(false);
const createdUsername = ref('');
const createdPassword = ref('');

const serverOptions = computed<SelectOption[]>(() =>
  props.servers.map((server) => ({ label: server.label, value: server.value }))
);

// Reset form when modal opens/closes
watch(() => props.open, (isOpen) => {
  if (isOpen) {
    selectedServerId.value = null;
    saving.value = false;
  }
});

function handleClose() {
  if (saving.value) return;
  emit('close');
}

async function handleSubmit(data: CreateUserFormData) {
  if (!selectedServerId.value) return;

  saving.value = true;
  try {
    const serverId = selectedServerId.value;

    // Create user
    const response = await usersApi.create(serverId, {
      username: data.username,
      password: data.password,
      home_dir: data.home_dir,
      quota_bytes: data.quota_bytes,
      enabled: data.enabled,
      description: data.description,
    });

    const createdUser = response.data;

    // Set access rules if provided
    if (data.accessRules && data.accessRules.length > 0) {
      await accessRulesApi.update(serverId, createdUser.username, data.accessRules);
    }

    // Set virtual paths if provided
    if (data.virtualPaths && data.virtualPaths.length > 0) {
      await virtualPathsApi.update(serverId, createdUser.username, data.virtualPaths);
    }

    // Add SSH keys if provided
    if (data.sshKeys && data.sshKeys.length > 0) {
      for (const key of data.sshKeys) {
        await sshKeysApi.add(serverId, createdUser.username, key);
      }
    }

    // Show password modal if password was generated
    if (createdUser.password) {
      createdUsername.value = createdUser.username;
      createdPassword.value = createdUser.password;
      showPasswordModal.value = true;
    }

    emit('created');
    emit('close');
  } catch (e) {
    console.error('Failed to create user:', e);
  } finally {
    saving.value = false;
  }
}
</script>
