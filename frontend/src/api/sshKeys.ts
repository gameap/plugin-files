import { apiClient } from './client';
import type { SshKeysResponse } from '@/types';

export const sshKeysApi = {
  list: (serverId: number, username: string) =>
    apiClient.get<SshKeysResponse>(
      `/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/ssh-keys`
    ),

  add: (serverId: number, username: string, key: string) =>
    apiClient.post<SshKeysResponse>(
      `/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/ssh-keys`,
      { key }
    ),

  delete: (serverId: number, username: string, index: number) =>
    apiClient.delete<SshKeysResponse>(
      `/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/ssh-keys/${index}`
    ),
};
