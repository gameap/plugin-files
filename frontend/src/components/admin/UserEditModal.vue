<template>
  <GModal
    :show="open"
    :title="modalTitle"
    style="width: 900px; max-width: 92vw"
    @update:show="(v: boolean) => !v && emit('close')"
  >
    <Loading v-if="loading" />

    <template v-else>
      <UserEditForm
        v-if="user"
        :user="user"
        :loading="saving"
        @submit="handleSubmit"
        @cancel="emit('close')"
      />
    </template>
  </GModal>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue';
import { usePluginTrans } from '@gameap/plugin-sdk';
import { usersApi } from '@/api';
import type { FtpUser, UpdateUserRequest } from '@/types';
import UserEditForm from '@/components/users/UserEditForm.vue';

const props = defineProps<{
  open: boolean;
  serverId: number | null;
  username: string | null;
}>();

const emit = defineEmits<{
  close: [];
  saved: [];
}>();

const { trans } = usePluginTrans();

const modalTitle = computed(() => {
  return `${trans('edit_user')}: ${props.username || ''}`;
});

const loading = ref(false);
const saving = ref(false);
const user = ref<FtpUser | null>(null);

watch(
  () => [props.open, props.serverId, props.username],
  async ([isOpen, serverId, username]) => {
    if (isOpen && serverId && username) {
      await fetchUser(serverId as number, username as string);
    } else {
      user.value = null;
    }
  },
  { immediate: true }
);

async function fetchUser(serverId: number, username: string) {
  loading.value = true;
  try {
    const response = await usersApi.get(serverId, username);
    user.value = response.data;
  } catch (e) {
    console.error('Failed to fetch user:', e);
    user.value = null;
  } finally {
    loading.value = false;
  }
}

async function handleSubmit(data: UpdateUserRequest) {
  if (!props.serverId || !props.username) return;

  saving.value = true;
  try {
    await usersApi.update(props.serverId, props.username, data);
    emit('saved');
    emit('close');
  } catch (e) {
    console.error('Failed to update user:', e);
  } finally {
    saving.value = false;
  }
}
</script>
