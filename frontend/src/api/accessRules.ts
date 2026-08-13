import { apiClient } from './client';
import type { AccessRule, AccessRulesResponse } from '@/types';

export const accessRulesApi = {
  get: (serverId: number, username: string) =>
    apiClient.get<AccessRulesResponse>(
      `/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/access-rules`
    ),

  update: (serverId: number, username: string, rules: AccessRule[]) =>
    apiClient.put<AccessRulesResponse>(
      `/servers/${serverId}/ftp-users/${encodeURIComponent(username)}/access-rules`,
      { rules }
    ),
};
