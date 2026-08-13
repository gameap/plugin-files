import { apiClient } from './client';
import type { AdminNodesResponse, AdminUsersResponse, UserFilters } from '@/types';

export const adminApi = {
  getNodes: () =>
    apiClient.get<AdminNodesResponse>('/admin/nodes'),

  getUsers: (filters?: UserFilters) =>
    apiClient.get<AdminUsersResponse>('/admin/users', { params: filters }),
};
