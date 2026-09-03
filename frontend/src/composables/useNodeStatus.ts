import { ref, watch, onUnmounted, type Ref } from 'vue';
import { nodesApi, showApiError } from '@/api';
import type { NodeSetupStatus, NodeSetupConfig, NodeConfigResponse } from '@/types';

export function useNodeStatus(nodeId: Ref<number | null>) {
  const status = ref<NodeSetupStatus | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const config = ref<NodeConfigResponse | null>(null);
  const configLoading = ref(false);
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  async function fetchStatus() {
    if (!nodeId.value) {
      status.value = null;
      return;
    }

    loading.value = true;
    error.value = null;

    try {
      const response = await nodesApi.getStatus(nodeId.value);
      status.value = response.data;

      // Start polling if installing
      if (status.value.status === 'installing' && !pollInterval) {
        pollInterval = setInterval(fetchStatus, 5000);
      } else if (status.value.status !== 'installing' && pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch node status';
      // If error, assume not installed
      status.value = {
        status: 'not_installed',
        last_check: Date.now(),
      };
    } finally {
      loading.value = false;
    }
  }

  async function startSetup(setupConfig?: NodeSetupConfig) {
    if (!nodeId.value) return;

    loading.value = true;
    error.value = null;

    try {
      const response = await nodesApi.setup(nodeId.value, setupConfig);
      status.value = response.data;

      // Start polling
      if (!pollInterval) {
        pollInterval = setInterval(fetchStatus, 5000);
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to start setup';
      // The plugin refuses unsupported node OSes with a 400 whose message is
      // the only explanation the operator gets.
      showApiError(e, 'Failed to start setup');
    } finally {
      loading.value = false;
    }
  }

  async function fetchConfig() {
    if (!nodeId.value) {
      config.value = null;
      return;
    }

    configLoading.value = true;
    error.value = null;

    try {
      const response = await nodesApi.getConfig(nodeId.value);
      config.value = response.data;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch config';
    } finally {
      configLoading.value = false;
    }
  }

  async function updateConfig(newConfig: NodeSetupConfig) {
    if (!nodeId.value) return;

    configLoading.value = true;
    error.value = null;

    try {
      const response = await nodesApi.updateConfig(nodeId.value, newConfig);
      config.value = response.data;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to update config';
      throw e;
    } finally {
      configLoading.value = false;
    }
  }

  watch(nodeId, fetchStatus, { immediate: true });

  onUnmounted(() => {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  });

  return {
    status,
    loading,
    error,
    config,
    configLoading,
    startSetup,
    fetchConfig,
    updateConfig,
    refresh: fetchStatus,
  };
}
