import axios from 'axios';
import { BASE } from './client';
import type { VirtualPath, VirtualPathsResponse } from '@/types';

export const virtualPathsApi = {
  get: (serverId: number, username: string) =>
    axios.get<VirtualPathsResponse>(
      `${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/virtual-paths`
    ),

  update: (serverId: number, username: string, paths: VirtualPath[]) =>
    axios.put<VirtualPathsResponse>(
      `${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/virtual-paths`,
      { paths }
    ),
};
