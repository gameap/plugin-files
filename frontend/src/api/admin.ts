import axios from 'axios';
import { BASE } from './client';
import type { AdminNodesResponse, AdminUsersResponse, UserFilters } from '@/types';

export const adminApi = {
  getNodes: () =>
    axios.get<AdminNodesResponse>(`${BASE}/admin/nodes`),

  getUsers: (filters?: UserFilters) =>
    axios.get<AdminUsersResponse>(`${BASE}/admin/users`, { params: filters }),
};
