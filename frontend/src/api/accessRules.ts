import axios from 'axios';
import { BASE } from './client';
import type { AccessRule, AccessRulesResponse } from '@/types';

export const accessRulesApi = {
  get: (serverId: number, username: string) =>
    axios.get<AccessRulesResponse>(
      `${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/access-rules`
    ),

  update: (serverId: number, username: string, rules: AccessRule[]) =>
    axios.put<AccessRulesResponse>(
      `${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/access-rules`,
      { rules }
    ),
};
