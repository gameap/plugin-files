export interface CreateUserRequest {
  username: string;
  password?: string;
  home_dir?: string;
  quota_bytes?: number;
  enabled?: boolean;
  description?: string;
}

export interface CreateUserResponse {
  username: string;
  password?: string;
  home_dir: string;
  quota_bytes: number;
  enabled: boolean;
  description: string;
}

export interface UpdateUserRequest {
  password?: string;
  home_dir?: string;
  quota_bytes?: number;
  enabled?: boolean;
  description?: string;
}

export interface ApiError {
  code: string;
  message: string;
}

export interface AccessRulesRequest {
  rules: import('./user').AccessRule[];
}

export interface AccessRulesResponse {
  rules: import('./user').AccessRule[];
}

export interface VirtualPathsRequest {
  paths: import('./user').VirtualPath[];
}

export interface VirtualPathsResponse {
  paths: import('./user').VirtualPath[];
}

export interface SshKeysResponse {
  keys: string[];
}

export interface AddSshKeyRequest {
  key: string;
}

export interface DeleteResponse {
  deleted: boolean;
}
