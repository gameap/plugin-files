import { ref, onUnmounted } from 'vue';
import { adminApi, nodesApi, showApiError } from '@/api';
import type { AdminNode, NodeSetupConfig, NodeConfigResponse } from '@/types';

/** Matches the interval useNodeStatus polls with on the server tab. */
const POLL_INTERVAL_MS = 5000;

export function useAdminNodes() {
  const nodes = ref<AdminNode[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // For individual node operations
  const operatingNodeId = ref<number | null>(null);
  const nodeConfig = ref<NodeConfigResponse | null>(null);
  const configLoading = ref(false);

  let pollInterval: ReturnType<typeof setInterval> | null = null;

  function stopPolling() {
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  }

  async function fetchNodes() {
    loading.value = true;
    error.value = null;

    try {
      const response = await adminApi.getNodes();
      nodes.value = response.data.nodes;

      // An installation is a chain of daemon tasks that finishes without the
      // page asking, so the list keeps refreshing itself while any node is
      // mid-install. Without this the card sits on "Installing" until someone
      // presses Refresh.
      const installing = nodes.value.some(
        (node) => node.plugin_status?.status === 'installing'
      );
      if (installing && !pollInterval) {
        pollInterval = setInterval(fetchNodes, POLL_INTERVAL_MS);
      } else if (!installing) {
        stopPolling();
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch nodes';
    } finally {
      loading.value = false;
    }
  }

  async function setupNode(nodeId: number, config?: NodeSetupConfig) {
    operatingNodeId.value = nodeId;
    error.value = null;

    try {
      await nodesApi.setup(nodeId, config);
      // Refresh nodes list to get updated status
      await fetchNodes();
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to setup node';
      showApiError(e, 'Failed to setup node');
      throw e;
    } finally {
      operatingNodeId.value = null;
    }
  }

  async function fetchNodeConfig(nodeId: number) {
    configLoading.value = true;
    error.value = null;

    try {
      const response = await nodesApi.getConfig(nodeId);
      nodeConfig.value = response.data;
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch config';
    } finally {
      configLoading.value = false;
    }
  }

  async function updateNodeConfig(nodeId: number, config: NodeSetupConfig) {
    configLoading.value = true;
    error.value = null;

    try {
      const response = await nodesApi.updateConfig(nodeId, config);
      nodeConfig.value = response.data;
      // Refresh nodes list
      await fetchNodes();
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to update config';
      throw e;
    } finally {
      configLoading.value = false;
    }
  }

  onUnmounted(stopPolling);

  return {
    nodes,
    loading,
    error,
    operatingNodeId,
    nodeConfig,
    configLoading,
    fetchNodes,
    setupNode,
    fetchNodeConfig,
    updateNodeConfig,
    refresh: fetchNodes,
  };
}
