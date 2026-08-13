import { apiClient } from './client';
import type { NodeSetupStatus, NodeSetupConfig, NodeConfigResponse } from '@/types';

export const nodesApi = {
  getStatus: (nodeId: number) =>
    apiClient.get<NodeSetupStatus>(`/nodes/${nodeId}/status`),

  setup: (nodeId: number, config?: NodeSetupConfig) =>
    apiClient.post<NodeSetupStatus>(`/nodes/${nodeId}/setup`, config),

  getConfig: (nodeId: number) =>
    apiClient.get<NodeConfigResponse>(`/nodes/${nodeId}/config`),

  updateConfig: (nodeId: number, config: NodeSetupConfig) =>
    apiClient.put<NodeConfigResponse>(`/nodes/${nodeId}/config`, config),
};
