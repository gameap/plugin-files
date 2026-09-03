<template>
  <GBreadcrumbs :items="breadcrumbs" class="mb-4" />

  <div class="mb-4 flex flex-wrap gap-2">
    <GButton color="white" size="small" :disabled="nodesLoading" @click="refreshNodes">
      <GIcon name="refresh" :class="nodesLoading ? 'fa-spin' : ''" />
      <span class="ml-1">{{ trans('refresh') }}</span>
    </GButton>
  </div>

  <FtpNodesFilters v-model="filters" />

  <div v-if="nodesLoading && nodes.length === 0" class="py-10 text-center">
    <Loading />
  </div>

  <div v-else-if="filteredNodes.length === 0" class="py-10">
    <GEmpty :description="trans('no_nodes')" />
  </div>

  <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
    <FtpNodeCard
      v-for="node in filteredNodes"
      :key="node.id"
      :node="node"
      :operating-node-id="operatingNodeId"
      @setup="openSetupModal"
      @update="onUpdateNode"
      @configure="openConfigModal"
    />
  </div>

  <!-- Node Setup Modal -->
  <NodeSetupModal
    v-if="setupModalOpen"
    v-model="setupModalOpen"
    :loading="operatingNodeId !== null"
    mode="install"
    @confirm="onSetupConfirm"
  />

  <!-- Node Config Modal -->
  <NodeSetupModal
    v-if="configModalOpen"
    v-model="configModalOpen"
    :loading="configLoading"
    mode="configure"
    :initial-config="nodeConfig"
    @confirm="onConfigConfirm"
  />
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { usePluginTrans } from '@gameap/plugin-sdk';
import { useAdminNodes } from '@/composables';
import type { NodeSetupConfig } from '@/types';
import FtpNodeCard from '@/components/admin/FtpNodeCard.vue';
import FtpNodesFilters, { type NodeListFilters } from '@/components/admin/FtpNodesFilters.vue';
import NodeSetupModal from '@/components/node/NodeSetupModal.vue';

const { trans } = usePluginTrans();

const breadcrumbs = computed(() => [
  { route: '/', text: 'GameAP', icon: 'gicon gicon-gameap' },
  { text: trans('ftp_users_admin') },
]);

// Nodes
const {
  nodes,
  loading: nodesLoading,
  operatingNodeId,
  nodeConfig,
  configLoading,
  fetchNodes: refreshNodes,
  setupNode,
  fetchNodeConfig,
  updateNodeConfig,
} = useAdminNodes();

const filters = ref<NodeListFilters>({ search: '', status: 'all' });

const filteredNodes = computed(() => {
  const query = filters.value.search.trim().toLowerCase();
  const wantStatus = filters.value.status;

  return nodes.value.filter((node) => {
    if (query) {
      const inName = node.name?.toLowerCase().includes(query);
      const inIp = node.ip?.toLowerCase().includes(query);
      if (!inName && !inIp) return false;
    }
    if (wantStatus !== 'all') {
      const installed = node.plugin_status?.status === 'installed';
      if (wantStatus === 'installed' && !installed) return false;
      if (wantStatus === 'not_installed' && installed) return false;
    }
    return true;
  });
});

// Setup modal state
const setupModalOpen = ref(false);
const setupNodeId = ref<number | null>(null);

// Config modal state
const configModalOpen = ref(false);
const configNodeId = ref<number | null>(null);

onMounted(() => {
  refreshNodes();
});

function openSetupModal(nodeId: number) {
  setupNodeId.value = nodeId;
  setupModalOpen.value = true;
}

async function onSetupConfirm(config: NodeSetupConfig | undefined) {
  if (setupNodeId.value === null) return;

  try {
    await setupNode(setupNodeId.value, config);
    setupModalOpen.value = false;
    setupNodeId.value = null;
  } catch (e) {
    console.error('Failed to setup node:', e);
  }
}

// An empty setup body re-runs the installer with the node's stored
// configuration, which is how an installed node gets upgraded.
async function onUpdateNode(nodeId: number) {
  try {
    await setupNode(nodeId);
  } catch (e) {
    console.error('Failed to update node:', e);
  }
}

async function openConfigModal(nodeId: number) {
  configNodeId.value = nodeId;
  await fetchNodeConfig(nodeId);
  // Only open modal if config was loaded successfully
  if (nodeConfig.value) {
    configModalOpen.value = true;
  } else {
    configNodeId.value = null;
  }
}

async function onConfigConfirm(config: NodeSetupConfig | undefined) {
  if (configNodeId.value === null || !config) return;

  try {
    await updateNodeConfig(configNodeId.value, config);
    configModalOpen.value = false;
    configNodeId.value = null;
  } catch (e) {
    console.error('Failed to update node config:', e);
  }
}
</script>
