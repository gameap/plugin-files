import axios from 'axios';
import { BASE } from './client';
import type { NodeSetupStatus, NodeSetupConfig, NodeConfigResponse } from '@/types';

export const nodesApi = {
  getStatus: (nodeId: number) =>
    axios.get<NodeSetupStatus>(`${BASE}/nodes/${nodeId}/status`),

  setup: (nodeId: number, config?: NodeSetupConfig) =>
    axios.post<NodeSetupStatus>(`${BASE}/nodes/${nodeId}/setup`, config),

  getConfig: (nodeId: number) =>
    axios.get<NodeConfigResponse>(`${BASE}/nodes/${nodeId}/config`),

  updateConfig: (nodeId: number, config: NodeSetupConfig) =>
    axios.put<NodeConfigResponse>(`${BASE}/nodes/${nodeId}/config`, config),
};
