import { apiClient } from './client';
import type { VirtualPath, VirtualPathsResponse } from '@/types';

export const virtualPathsApi = {
  get: (serverId: number, username: string) =>
    apiClient.get<VirtualPathsResponse>(
      `/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/virtual-paths`
    ),

  update: (serverId: number, username: string, paths: VirtualPath[]) =>
    apiClient.put<VirtualPathsResponse>(
      `/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/virtual-paths`,
      { paths }
    ),
};
