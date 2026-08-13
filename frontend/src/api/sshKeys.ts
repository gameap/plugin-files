import axios from 'axios';
import { BASE } from './client';
import type { SshKeysResponse } from '@/types';

export const sshKeysApi = {
  list: (serverId: number, username: string) =>
    axios.get<SshKeysResponse>(
      `${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/ssh-keys`
    ),

  add: (serverId: number, username: string, key: string) =>
    axios.post<SshKeysResponse>(
      `${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/ssh-keys`,
      { key }
    ),

  delete: (serverId: number, username: string, index: number) =>
    axios.delete<SshKeysResponse>(
      `${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/ssh-keys/${index}`
    ),
};
