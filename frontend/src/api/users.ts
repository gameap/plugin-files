import { apiClient } from './client';
import type {
  FtpUser,
  CreateUserRequest,
  CreateUserResponse,
  UpdateUserRequest,
  DeleteResponse,
} from '@/types';

export const usersApi = {
  list: (serverId: number) =>
    apiClient.get<FtpUser[]>(`/servers/${serverId}/ftp-users`),

  get: (serverId: number, username: string) =>
    apiClient.get<FtpUser>(`/servers/${serverId}/ftp-users/${encodeURIComponent(username)}`),

  create: (serverId: number, data: CreateUserRequest) =>
    apiClient.post<CreateUserResponse>(`/servers/${serverId}/ftp-users`, data),

  update: (serverId: number, username: string, data: UpdateUserRequest) =>
    apiClient.put<FtpUser>(`/servers/${serverId}/ftp-users/${encodeURIComponent(username)}`, data),

  delete: (serverId: number, username: string) =>
    apiClient.delete<DeleteResponse>(`/servers/${serverId}/ftp-users/${encodeURIComponent(username)}`),
};
