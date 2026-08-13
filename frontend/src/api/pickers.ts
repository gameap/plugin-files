import axios from 'axios';
import { BASE } from './client';
import type { PickerServersResponse } from '@/types';

export const pickersApi = {
  searchServers: (q: string, node?: number, limit = 20) =>
    axios.get<PickerServersResponse>(`${BASE}/admin/pickers/servers`, {
      params: { q, limit, ...(node ? { node } : {}) },
    }),
};
