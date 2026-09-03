<template>
  <n-card
    size="small"
    :bordered="true"
    :segmented="{ content: true }"
    :class="isInstalled ? 'cursor-pointer transition-shadow hover:shadow-lg' : ''"
    :role="isInstalled ? 'button' : undefined"
    @click="navigateToUsers"
  >
    <template #header>
      <div class="flex items-center gap-2 min-w-0">
        <GIcon name="server" class="text-lg flex-none text-stone-400" />
        <span class="font-semibold truncate">{{ node.name }}</span>
      </div>
    </template>

    <template #header-extra>
      <n-tag v-if="status === 'installed'" type="success" size="small" round :bordered="false">
        {{ node.plugin_status?.version || trans('installed_short') }}
      </n-tag>
      <n-tag v-else-if="status === 'installing'" type="info" size="small" round :bordered="false">
        <template #icon>
          <GIcon name="spinner" />
        </template>
        {{ trans('installing') }}
      </n-tag>
      <n-tag v-else-if="status === 'failed'" type="error" size="small" round :bordered="false">
        {{ trans('failed_short') }}
      </n-tag>
      <n-tag v-else size="small" round :bordered="false">
        {{ trans('not_installed_short') }}
      </n-tag>
    </template>

    <div class="text-sm text-stone-500 dark:text-stone-400 mb-3 font-mono truncate">
      {{ node.ip || '—' }}
    </div>

    <div
      v-if="status === 'failed' && node.plugin_status?.error_message"
      class="text-xs text-red-500 dark:text-red-400 mb-3 break-all"
      style="display:-webkit-box;-webkit-line-clamp:3;line-clamp:3;-webkit-box-orient:vertical;overflow:hidden"
    >
      {{ node.plugin_status.error_message }}
    </div>

    <div class="flex flex-wrap gap-2">
      <GButton
        v-if="status === 'not_installed' || status === 'failed'"
        color="green"
        size="small"
        :loading="isOperating"
        @click="emit('setup', node.id)"
      >
        <GIcon :name="status === 'failed' ? 'refresh' : 'download'" />
        <span class="ml-1">{{ status === 'failed' ? trans('retry_setup') : trans('install') }}</span>
      </GButton>

      <template v-if="status === 'installed'">
        <GButton color="white" size="small" @click="goToUsers">
          <GIcon name="users" />
          <span class="ml-1">{{ trans('view_users') }}</span>
        </GButton>
        <GButton
          color="white"
          size="small"
          :loading="isOperating"
          :title="trans('update_installation_hint')"
          @click="emit('update', node.id)"
        >
          <GIcon name="sync" />
          <span class="ml-1">{{ trans('update_installation') }}</span>
        </GButton>
        <GButton color="white" size="small" :loading="isOperating" @click="emit('configure', node.id)">
          <GIcon name="settings" />
          <span class="ml-1">{{ trans('settings') }}</span>
        </GButton>
      </template>

      <span
        v-if="status === 'installing'"
        class="text-xs text-stone-400 dark:text-stone-500 italic py-1"
      >
        {{ trans('status_installing') }}
      </span>
    </div>
  </n-card>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NCard, NTag } from 'naive-ui';
import { useRouter } from 'vue-router';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { AdminNode } from '@/types';

const props = defineProps<{
  node: AdminNode;
  operatingNodeId?: number | null;
}>();

const emit = defineEmits<{
  setup: [nodeId: number];
  /** Re-run the installer with the stored configuration (upgrade). */
  update: [nodeId: number];
  configure: [nodeId: number];
}>();

const router = useRouter();
const { trans } = usePluginTrans();

const status = computed(() => props.node.plugin_status?.status ?? 'not_installed');
const isOperating = computed(() => props.operatingNodeId === props.node.id);
const isInstalled = computed(() => status.value === 'installed');

function goToUsers() {
  router.push(`/plugins/files/nodes/${props.node.id}/users`);
}

function navigateToUsers(event: MouseEvent) {
  // Don't navigate if clicking on buttons/links inside the card.
  const target = event.target as HTMLElement;
  if (target.closest('button') || target.closest('a')) return;

  if (isInstalled.value) {
    goToUsers();
  }
}
</script>
