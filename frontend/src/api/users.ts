import axios from 'axios';
import { BASE } from './client';
import type {
  FtpUser,
  CreateUserRequest,
  CreateUserResponse,
  UpdateUserRequest,
  DeleteResponse,
} from '@/types';

export const usersApi = {
  list: (serverId: number) =>
    axios.get<FtpUser[]>(`${BASE}/servers/${serverId}/ftp-users`),

  get: (serverId: number, username: string) =>
    axios.get<FtpUser>(`${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}`),

  create: (serverId: number, data: CreateUserRequest) =>
    axios.post<CreateUserResponse>(`${BASE}/servers/${serverId}/ftp-users`, data),

  update: (serverId: number, username: string, data: UpdateUserRequest) =>
    axios.put<FtpUser>(`${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}`, data),

  delete: (serverId: number, username: string) =>
    axios.delete<DeleteResponse>(`${BASE}/servers/${serverId}/ftp-users/${encodeURIComponent(username)}`),
};
