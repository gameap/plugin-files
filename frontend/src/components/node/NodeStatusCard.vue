<template>
  <n-card size="small">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div :class="statusIconClasses">
          <GIcon :name="statusIconName" />
        </div>
        <div>
          <div class="font-medium text-stone-800 dark:text-stone-100">
            {{ trans('node_status') }}
          </div>
          <div class="text-sm text-stone-500 dark:text-stone-400">
            {{ statusText }}
          </div>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <!-- Install/Retry button -->
        <GButton
          v-if="status?.status === 'not_installed' || status?.status === 'failed'"
          color="green"
          size="small"
          :loading="loading"
          @click="openInstallModal"
        >
          <GIcon :name="status?.status === 'failed' ? 'refresh' : 'download'" />
          <span class="hidden lg:inline ml-1">
            {{ status?.status === 'failed' ? trans('retry_setup') : trans('install') }}
          </span>
        </GButton>

        <!-- Version badge, update and settings buttons -->
        <template v-else-if="status?.status === 'installed'">
          <GStatusBadge status="success" :text="status.version" />
          <GButton
            color="white"
            size="small"
            :loading="loading"
            :title="trans('update_installation_hint')"
            @click="emit('update')"
          >
            <GIcon name="sync" />
            <span class="hidden lg:inline ml-1">{{ trans('update_installation') }}</span>
          </GButton>
          <GButton
            color="white"
            size="small"
            :loading="configLoading"
            @click="openConfigureModal"
          >
            <GIcon name="settings" />
          </GButton>
        </template>
      </div>
    </div>

    <!-- Installation progress -->
    <div
      v-if="status?.status === 'installing'"
      class="mt-3 flex items-center gap-2 text-sm text-stone-500 dark:text-stone-400"
    >
      <GIcon name="spinner" />
      <span>{{ trans('installing') }}…</span>
    </div>

    <!-- Error message -->
    <div
      v-if="status?.status === 'failed' && status.error_message"
      class="mt-3 p-3 rounded border border-danger bg-danger-soft text-danger-soft-text text-sm break-all"
    >
      {{ status.error_message }}
    </div>

    <!-- Setup Modal -->
    <NodeSetupModal
      v-model="showSetupModal"
      :loading="modalMode === 'install' ? loading : configLoading"
      :mode="modalMode"
      :initial-config="config"
      @confirm="handleSetupConfirm"
    />
  </n-card>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { NCard } from 'naive-ui';
import { usePluginTrans } from '@gameap/plugin-sdk';
import type { NodeSetupStatus, NodeSetupConfig, NodeConfigResponse } from '@/types';
import NodeSetupModal from './NodeSetupModal.vue';

const props = defineProps<{
  status: NodeSetupStatus | null;
  loading?: boolean;
  config?: NodeConfigResponse | null;
  configLoading?: boolean;
}>();

const emit = defineEmits<{
  setup: [config?: NodeSetupConfig];
  /** Re-run the installer with the stored configuration (upgrade). */
  update: [];
  loadConfig: [];
  updateConfig: [config: NodeSetupConfig];
}>();

const { trans } = usePluginTrans();

const showSetupModal = ref(false);
const modalMode = ref<'install' | 'configure'>('install');

function openInstallModal() {
  modalMode.value = 'install';
  showSetupModal.value = true;
}

function openConfigureModal() {
  modalMode.value = 'configure';
  emit('loadConfig');
  showSetupModal.value = true;
}

function handleSetupConfirm(config?: NodeSetupConfig) {
  showSetupModal.value = false;
  if (modalMode.value === 'install') {
    emit('setup', config);
  } else if (config) {
    emit('updateConfig', config);
  }
}

const statusIconName = computed(() => {
  switch (props.status?.status) {
    case 'installed': return 'check';
    case 'installing': return 'spinner';
    case 'failed': return 'warning';
    default: return 'server';
  }
});

const statusIconClasses = computed(() => {
  const base = 'w-10 h-10 rounded-full flex items-center justify-center text-lg';
  switch (props.status?.status) {
    case 'installed':
      return `${base} bg-success-soft text-success-soft-text`;
    case 'installing':
      return `${base} bg-info-soft text-info-soft-text`;
    case 'failed':
      return `${base} bg-danger-soft text-danger-soft-text`;
    default:
      return `${base} bg-stone-100 text-stone-500 dark:bg-stone-800 dark:text-stone-400`;
  }
});

const statusText = computed(() => {
  switch (props.status?.status) {
    case 'installed': return trans('status_installed');
    case 'installing': return trans('status_installing');
    case 'failed': return trans('status_failed');
    default: return trans('status_not_installed');
  }
});
</script>
