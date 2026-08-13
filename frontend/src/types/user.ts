export type Permission = 'read' | 'write' | 'delete' | 'list';

export interface AccessRule {
  path: string;
  permissions: Permission[];
}

export interface VirtualPath {
  virtual: string;
  target: string;
  permissions: Permission[];
  read_only: boolean;
}

export interface FtpUser {
  username: string;
  home_dir: string;
  quota_bytes: number;
  enabled: boolean;
  description: string;
  ssh_keys_count: number;
  access_rules: AccessRule[];
  virtual_paths: VirtualPath[];
}
